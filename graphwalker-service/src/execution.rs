use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};

use graphwalker_core::machine::{ExecutionContext, Machine};
use graphwalker_core::model::{Action, EdgeIndex, ElementIndex, VertexIndex};
use graphwalker_dsl::generator::parse_generator;
use graphwalker_io::json::{read_json_string, write_json_string};
use graphwalker_io::ModelContext;
use rand::Rng;
use serde_json::Value;

use crate::types::{
    ElementKind, ElementStatus, ExecutionId, ExecutionLimits, ExecutionStatistics, ExecutionStatus,
    ModelResult, RestartResult, ServiceError, ServiceErrorCode, SetDataResult, StartExecution,
    StartExecutionResult, StepElement, StepResult,
};

type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Clone)]
struct ExecutionHandle {
    sender: mpsc::Sender<Command>,
}

#[derive(Clone)]
pub struct ExecutionRegistry {
    executions: Arc<RwLock<HashMap<ExecutionId, ExecutionHandle>>>,
    limits: ExecutionLimits,
}

impl Default for ExecutionRegistry {
    fn default() -> Self {
        Self::new(ExecutionLimits::default())
    }
}

impl ExecutionRegistry {
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            limits,
        }
    }

    pub fn start(&self, request: StartExecution) -> ServiceResult<StartExecutionResult> {
        let mut executions = self.executions.write().map_err(lock_error)?;
        if executions.len() >= self.limits.max_executions {
            return Err(ServiceError::new(
                ServiceErrorCode::ExecutionLimitReached,
                format!(
                    "Execution limit of {} has been reached",
                    self.limits.max_executions
                ),
            ));
        }

        let execution_id = loop {
            let candidate = ExecutionId::new(format!(
                "execution_{:032x}",
                rand::thread_rng().gen::<u128>()
            ));
            if !executions.contains_key(&candidate) {
                break candidate;
            }
        };

        let (sender, receiver) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name(format!("graphwalker-{}", execution_id.as_str()))
            .spawn(move || worker(request, receiver, started_tx))
            .map_err(|error| ServiceError::internal(error.to_string()))?;

        let seed = started_rx.recv().map_err(|_| unavailable_error())??;
        executions.insert(execution_id.clone(), ExecutionHandle { sender });

        Ok(StartExecutionResult { execution_id, seed })
    }

    pub fn status(&self, execution_id: &ExecutionId) -> ServiceResult<ExecutionStatus> {
        self.request(execution_id, |reply| Command::Status { reply })
    }

    pub fn next_step(&self, execution_id: &ExecutionId) -> ServiceResult<StepResult> {
        self.request(execution_id, |reply| Command::NextStep { reply })
    }

    pub fn data(&self, execution_id: &ExecutionId) -> ServiceResult<String> {
        self.request(execution_id, |reply| Command::Data { reply })
    }

    pub fn set_data(
        &self,
        execution_id: &ExecutionId,
        script: impl Into<String>,
    ) -> ServiceResult<SetDataResult> {
        self.request(execution_id, |reply| Command::SetData {
            script: script.into(),
            reply,
        })
    }

    pub fn restart(&self, execution_id: &ExecutionId) -> ServiceResult<RestartResult> {
        self.request(execution_id, |reply| Command::Restart { reply })
    }

    pub fn statistics(&self, execution_id: &ExecutionId) -> ServiceResult<ExecutionStatistics> {
        self.request(execution_id, |reply| Command::Statistics { reply })
    }

    pub fn model(&self, execution_id: &ExecutionId) -> ServiceResult<ModelResult> {
        self.request(execution_id, |reply| Command::Model { reply })
    }

    pub fn elements(&self, execution_id: &ExecutionId) -> ServiceResult<Vec<ElementStatus>> {
        self.request(execution_id, |reply| Command::Elements { reply })
    }

    pub fn close(&self, execution_id: &ExecutionId) -> ServiceResult<()> {
        let handle = self
            .executions
            .write()
            .map_err(lock_error)?
            .remove(execution_id)
            .ok_or_else(|| not_found(execution_id))?;
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        handle
            .sender
            .send(Command::Close { reply: reply_tx })
            .map_err(|_| unavailable_error())?;
        reply_rx.recv().map_err(|_| unavailable_error())
    }

    pub fn len(&self) -> usize {
        self.executions.read().map(|items| items.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn request<T>(
        &self,
        execution_id: &ExecutionId,
        build: impl FnOnce(mpsc::SyncSender<ServiceResult<T>>) -> Command,
    ) -> ServiceResult<T> {
        let handle = self
            .executions
            .read()
            .map_err(lock_error)?
            .get(execution_id)
            .cloned()
            .ok_or_else(|| not_found(execution_id))?;
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        handle
            .sender
            .send(build(reply_tx))
            .map_err(|_| unavailable_error())?;
        reply_rx.recv().map_err(|_| unavailable_error())?
    }
}

enum Command {
    Status {
        reply: mpsc::SyncSender<ServiceResult<ExecutionStatus>>,
    },
    NextStep {
        reply: mpsc::SyncSender<ServiceResult<StepResult>>,
    },
    Data {
        reply: mpsc::SyncSender<ServiceResult<String>>,
    },
    SetData {
        script: String,
        reply: mpsc::SyncSender<ServiceResult<SetDataResult>>,
    },
    Restart {
        reply: mpsc::SyncSender<ServiceResult<RestartResult>>,
    },
    Statistics {
        reply: mpsc::SyncSender<ServiceResult<ExecutionStatistics>>,
    },
    Model {
        reply: mpsc::SyncSender<ServiceResult<ModelResult>>,
    },
    Elements {
        reply: mpsc::SyncSender<ServiceResult<Vec<ElementStatus>>>,
    },
    Close {
        reply: mpsc::SyncSender<()>,
    },
}

struct ExecutionState {
    machine: Machine,
    contexts: Vec<ModelContext>,
    model: Value,
    seed: u64,
    global_data: Option<String>,
}

impl ExecutionState {
    fn new(request: StartExecution) -> ServiceResult<Self> {
        let contexts = read_json_string(&request.model.to_string())
            .map_err(|error| ServiceError::invalid_model(error.to_string()))?;
        let seed = request.seed.unwrap_or_else(|| rand::thread_rng().gen());
        let machine = build_machine(&contexts, seed, request.global_data.as_deref())?;
        let json = write_json_string(&contexts)
            .map_err(|error| ServiceError::internal(error.to_string()))?;
        let model = serde_json::from_str(&json)
            .map_err(|error| ServiceError::internal(error.to_string()))?;
        Ok(Self {
            machine,
            contexts,
            model,
            seed,
            global_data: request.global_data,
        })
    }

    fn restart(&mut self) -> ServiceResult<RestartResult> {
        self.machine = build_machine(&self.contexts, self.seed, self.global_data.as_deref())?;
        Ok(RestartResult { seed: self.seed })
    }
}

fn worker(
    request: StartExecution,
    receiver: mpsc::Receiver<Command>,
    started: mpsc::SyncSender<ServiceResult<u64>>,
) {
    let mut state = match ExecutionState::new(request) {
        Ok(state) => {
            let _ = started.send(Ok(state.seed));
            state
        }
        Err(error) => {
            let _ = started.send(Err(error));
            return;
        }
    };

    while let Ok(command) = receiver.recv() {
        match command {
            Command::Status { reply } => {
                let has_next = state.machine.has_next_step();
                let data = state.machine.current_context().data();
                let _ = reply.send(Ok(ExecutionStatus { has_next, data }));
            }
            Command::NextStep { reply } => {
                let _ = reply.send(next_step(&mut state.machine));
            }
            Command::Data { reply } => {
                let _ = reply.send(Ok(state.machine.current_context().data()));
            }
            Command::SetData { script, reply } => {
                let _ = reply.send(set_data(&mut state.machine, &script));
            }
            Command::Restart { reply } => {
                let _ = reply.send(state.restart());
            }
            Command::Statistics { reply } => {
                let _ = reply.send(Ok(statistics(&state.machine)));
            }
            Command::Model { reply } => {
                let _ = reply.send(Ok(ModelResult {
                    model: state.model.clone(),
                }));
            }
            Command::Elements { reply } => {
                let _ = reply.send(Ok(element_statuses(&state.machine)));
            }
            Command::Close { reply } => {
                let _ = reply.send(());
                break;
            }
        }
    }
}

fn build_machine(
    contexts: &[ModelContext],
    seed: u64,
    global_data: Option<&str>,
) -> ServiceResult<Machine> {
    let mut entries = Vec::with_capacity(contexts.len());
    for context in contexts {
        let generator_expression = context.generator.as_deref().ok_or_else(|| {
            ServiceError::new(
                ServiceErrorCode::InvalidGenerator,
                "Model has no generator specified",
            )
        })?;
        let generator = parse_generator(generator_expression).map_err(|error| {
            ServiceError::new(ServiceErrorCode::InvalidGenerator, error.to_string())
        })?;
        let mut execution_context = ExecutionContext::new_with_seed(context.model.clone(), seed);
        if let Some(start_id) = &context.start_element_id {
            if let Some(element) = execution_context.model().element_by_id(start_id) {
                execution_context.set_next_element(Some(element));
            }
        }
        entries.push((execution_context, generator));
    }

    let machine = Machine::new_with_seed(entries, seed)
        .map_err(|error| ServiceError::invalid_model(error.to_string()))?;
    if let Some(data) = global_data {
        for statement in data
            .split(';')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let action = Action::new(format!("global.{statement}"));
            machine
                .current_context()
                .execute_action(&action)
                .map_err(|error| {
                    ServiceError::new(ServiceErrorCode::InvalidData, error.to_string())
                })?;
        }
    }
    Ok(machine)
}

fn next_step(machine: &mut Machine) -> ServiceResult<StepResult> {
    if !machine.has_next_step() {
        return Ok(StepResult {
            completed: true,
            element: None,
        });
    }
    machine
        .get_next_step()
        .map_err(|error| ServiceError::invalid_model(error.to_string()))?;

    let context_index = machine.current_context_index();
    let context = machine.context(context_index);
    let element = context
        .current_element()
        .ok_or_else(|| ServiceError::new(ServiceErrorCode::Internal, "No current element"))?;
    let (id, name, kind) = match element {
        ElementIndex::Vertex(index) => {
            let vertex = context.model().vertex(index);
            (
                vertex.id().to_string(),
                vertex.name().unwrap_or("").to_string(),
                ElementKind::Vertex,
            )
        }
        ElementIndex::Edge(index) => {
            let edge = context.model().edge(index);
            (
                edge.id().to_string(),
                edge.name().unwrap_or("").to_string(),
                ElementKind::Edge,
            )
        }
    };

    Ok(StepResult {
        completed: false,
        element: Some(StepElement {
            id,
            name,
            model_id: context.model().id().to_string(),
            kind,
            data: context.data(),
            visited_count: context.visit_count(element),
            total_count: context.total_visit_count(),
            stop_condition_fulfillment: machine.get_fulfilment(context_index),
        }),
    })
}

fn set_data(machine: &mut Machine, script: &str) -> ServiceResult<SetDataResult> {
    let action = Action::new(script);
    let context_index = machine.current_context_index();
    machine
        .context_mut(context_index)
        .execute_action(&action)
        .map_err(|error| ServiceError::new(ServiceErrorCode::InvalidData, error.to_string()))?;
    Ok(SetDataResult {
        data: machine.current_context().data(),
    })
}

fn statistics(machine: &Machine) -> ExecutionStatistics {
    let mut total_vertices = 0;
    let mut total_edges = 0;
    let mut visited_vertices = 0;
    let mut visited_edges = 0;

    for context_index in 0..machine.context_count() {
        let context = machine.context(context_index);
        let model = context.model();
        total_vertices += model.vertices().len();
        total_edges += model.edges().len();
        visited_vertices += (0..model.vertices().len())
            .filter(|index| context.is_visited(ElementIndex::Vertex(VertexIndex(*index))))
            .count();
        visited_edges += (0..model.edges().len())
            .filter(|index| context.is_visited(ElementIndex::Edge(EdgeIndex(*index))))
            .count();
    }

    ExecutionStatistics {
        total_vertices,
        total_edges,
        visited_vertices,
        visited_edges,
        unvisited_vertices: total_vertices - visited_vertices,
        unvisited_edges: total_edges - visited_edges,
        vertex_coverage: percentage(visited_vertices, total_vertices),
        edge_coverage: percentage(visited_edges, total_edges),
    }
}

fn percentage(visited: usize, total: usize) -> u32 {
    if total == 0 {
        0
    } else {
        ((visited as f64 / total as f64) * 100.0) as u32
    }
}

fn element_statuses(machine: &Machine) -> Vec<ElementStatus> {
    let mut elements = Vec::new();
    for context_index in 0..machine.context_count() {
        let context = machine.context(context_index);
        let model = context.model();
        for index in 0..model.vertices().len() {
            let element = ElementIndex::Vertex(VertexIndex(index));
            elements.push(ElementStatus {
                model_id: model.id().to_string(),
                element_id: model.vertex(VertexIndex(index)).id().to_string(),
                visited_count: context.visit_count(element),
            });
        }
        for index in 0..model.edges().len() {
            let element = ElementIndex::Edge(EdgeIndex(index));
            elements.push(ElementStatus {
                model_id: model.id().to_string(),
                element_id: model.edge(EdgeIndex(index)).id().to_string(),
                visited_count: context.visit_count(element),
            });
        }
    }
    elements
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::internal(format!("Execution registry lock was poisoned: {error}"))
}

fn not_found(execution_id: &ExecutionId) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::ExecutionNotFound,
        format!("Execution '{}' was not found", execution_id.as_str()),
    )
}

fn unavailable_error() -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::ExecutionUnavailable,
        "Execution worker is unavailable",
    )
}
