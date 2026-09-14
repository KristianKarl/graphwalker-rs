use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use graphwalker_io::json::{JsonEdge, JsonModel, JsonMultimodel, JsonVertex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{validate_model, ServiceError, ServiceErrorCode, ValidationIssue, ValidationResult};

type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DraftId(String);

impl DraftId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DraftId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DraftId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for DraftId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftLimits {
    pub max_drafts: usize,
    pub max_vertices_per_draft: usize,
    pub max_edges_per_draft: usize,
    pub idle_timeout: Duration,
}

impl Default for DraftLimits {
    fn default() -> Self {
        Self {
            max_drafts: 64,
            max_vertices_per_draft: 10_000,
            max_edges_per_draft: 20_000,
            idle_timeout: Duration::from_secs(30 * 60),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CreateModel {
    pub model_id: Option<String>,
    pub name: Option<String>,
    pub generator: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct AddVertex {
    pub draft_id: DraftId,
    pub id: Option<String>,
    pub name: Option<String>,
    pub shared_state: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct AddEdge {
    pub draft_id: DraftId,
    pub id: Option<String>,
    pub name: Option<String>,
    pub source_vertex_id: Option<String>,
    pub target_vertex_id: Option<String>,
    pub guard: Option<String>,
    pub actions: Vec<String>,
    pub requirements: Vec<String>,
    pub properties: HashMap<String, Value>,
    pub weight: Option<f64>,
    pub dependency: Option<i32>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub enum FieldPatch<T> {
    #[default]
    Keep,
    Set(T),
    Clear,
}

#[derive(Clone, Debug)]
pub struct UpdateModel {
    pub draft_id: DraftId,
    pub name: FieldPatch<String>,
    pub generator: FieldPatch<String>,
    pub start_element_id: FieldPatch<String>,
    pub actions: FieldPatch<Vec<String>>,
    pub requirements: FieldPatch<Vec<String>>,
    pub properties: FieldPatch<HashMap<String, Value>>,
    pub predefined_path_edge_ids: FieldPatch<Vec<String>>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct UpdateVertex {
    pub draft_id: DraftId,
    pub vertex_id: String,
    pub name: FieldPatch<String>,
    pub shared_state: FieldPatch<String>,
    pub actions: FieldPatch<Vec<String>>,
    pub requirements: FieldPatch<Vec<String>>,
    pub properties: FieldPatch<HashMap<String, Value>>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct UpdateEdge {
    pub draft_id: DraftId,
    pub edge_id: String,
    pub name: FieldPatch<String>,
    pub source_vertex_id: FieldPatch<String>,
    pub target_vertex_id: FieldPatch<String>,
    pub guard: FieldPatch<String>,
    pub actions: FieldPatch<Vec<String>>,
    pub requirements: FieldPatch<Vec<String>>,
    pub properties: FieldPatch<HashMap<String, Value>>,
    pub weight: FieldPatch<f64>,
    pub dependency: FieldPatch<i32>,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct RemoveElement {
    pub draft_id: DraftId,
    pub element_id: String,
    pub cascade: bool,
    pub cleanup_references: bool,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DraftCreated {
    pub draft_id: DraftId,
    pub model_id: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct VertexResult {
    pub vertex: JsonVertex,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EdgeResult {
    pub edge: JsonEdge,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelResult {
    pub model: JsonModel,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExportedModel {
    pub model: Value,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DraftSnapshot {
    pub model: Value,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DraftValidation {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemoveResult {
    pub removed_ids: Vec<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscardResult {
    pub discarded: bool,
}

struct Draft {
    document: JsonMultimodel,
    revision: u64,
    last_access: Instant,
    lifecycle: DraftLifecycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DraftLifecycle {
    Active,
    Expired,
    Discarded,
}

#[derive(Default)]
struct RegistryState {
    drafts: HashMap<DraftId, Arc<Mutex<Draft>>>,
    expired: HashSet<DraftId>,
    expired_order: VecDeque<DraftId>,
}

#[derive(Clone)]
pub struct DraftRegistry {
    state: Arc<RwLock<RegistryState>>,
    limits: DraftLimits,
}

impl Default for DraftRegistry {
    fn default() -> Self {
        Self::new(DraftLimits::default())
    }
}

impl DraftRegistry {
    pub fn new(limits: DraftLimits) -> Self {
        Self {
            state: Arc::new(RwLock::new(RegistryState::default())),
            limits,
        }
    }

    pub fn create_model(&self, request: CreateModel) -> ServiceResult<DraftCreated> {
        let model_id = request
            .model_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        validate_nonempty_id(&model_id, "model")?;

        let model = JsonModel {
            name: request.name,
            id: Some(model_id.clone()),
            generator: request.generator,
            start_element_id: None,
            actions: request.actions,
            requirements: request.requirements,
            properties: request.properties,
            vertices: Vec::new(),
            edges: Vec::new(),
            predefined_path_edge_ids: Vec::new(),
        };
        let document = JsonMultimodel {
            name: None,
            seed: None,
            models: vec![model],
        };

        let mut state = self.state.write().map_err(registry_lock_error)?;
        self.remove_expired_locked(&mut state)?;
        if state.drafts.len() >= self.limits.max_drafts {
            return Err(ServiceError::new(
                ServiceErrorCode::DraftLimitReached,
                format!("Draft limit of {} has been reached", self.limits.max_drafts),
            ));
        }
        let draft_id = loop {
            let candidate = DraftId(format!("draft_{}", Uuid::new_v4()));
            if !state.drafts.contains_key(&candidate) {
                break candidate;
            }
        };
        state.drafts.insert(
            draft_id.clone(),
            Arc::new(Mutex::new(Draft {
                document,
                revision: 0,
                last_access: Instant::now(),
                lifecycle: DraftLifecycle::Active,
            })),
        );

        Ok(DraftCreated {
            draft_id,
            model_id,
            revision: 0,
        })
    }

    pub fn add_vertex(&self, request: AddVertex) -> ServiceResult<VertexResult> {
        let id = request.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        validate_nonempty_id(&id, "vertex")?;
        let vertex = JsonVertex {
            id,
            name: request.name,
            shared_state: request.shared_state,
            actions: request.actions,
            requirements: request.requirements,
            properties: request.properties,
        };
        let (vertex, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                ensure_unique_element_id(model, &vertex.id)?;
                model.vertices.push(vertex.clone());
                Ok(vertex)
            })?;
        Ok(VertexResult { vertex, revision })
    }

    pub fn add_edge(&self, request: AddEdge) -> ServiceResult<EdgeResult> {
        let id = request.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        validate_nonempty_id(&id, "edge")?;
        let target_vertex_id = request.target_vertex_id.ok_or_else(|| {
            ServiceError::new(
                ServiceErrorCode::MissingTargetVertex,
                "An edge must have a target vertex",
            )
        })?;
        validate_weight(request.weight)?;
        validate_dependency(request.dependency)?;
        let edge = JsonEdge {
            id,
            name: request.name,
            guard: request.guard,
            actions: request.actions,
            requirements: request.requirements,
            properties: request.properties,
            weight: request.weight,
            dependency: request.dependency,
            source_vertex_id: request.source_vertex_id,
            target_vertex_id: Some(target_vertex_id),
        };
        let (edge, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                ensure_unique_element_id(model, &edge.id)?;
                validate_edge_references(model, &edge)?;
                model.edges.push(edge.clone());
                Ok(edge)
            })?;
        Ok(EdgeResult { edge, revision })
    }

    pub fn update_model(&self, request: UpdateModel) -> ServiceResult<ModelResult> {
        let (model, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                apply_optional_patch(&mut model.name, request.name);
                apply_optional_patch(&mut model.generator, request.generator);
                apply_vec_patch(&mut model.actions, request.actions);
                apply_vec_patch(&mut model.requirements, request.requirements);
                apply_map_patch(&mut model.properties, request.properties);

                match request.start_element_id {
                    FieldPatch::Keep => {}
                    FieldPatch::Clear => model.start_element_id = None,
                    FieldPatch::Set(id) => {
                        ensure_element_exists(model, &id)?;
                        model.start_element_id = Some(id);
                    }
                }
                match request.predefined_path_edge_ids {
                    FieldPatch::Keep => {}
                    FieldPatch::Clear => model.predefined_path_edge_ids.clear(),
                    FieldPatch::Set(ids) => {
                        for id in &ids {
                            if !model.edges.iter().any(|edge| edge.id == *id) {
                                return Err(ServiceError::new(
                                    ServiceErrorCode::InvalidElement,
                                    format!("Predefined path edge '{id}' does not exist"),
                                ));
                            }
                        }
                        model.predefined_path_edge_ids = ids;
                    }
                }
                Ok(model.clone())
            })?;
        Ok(ModelResult { model, revision })
    }

    pub fn update_vertex(&self, request: UpdateVertex) -> ServiceResult<VertexResult> {
        let (vertex, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                let vertex = model
                    .vertices
                    .iter_mut()
                    .find(|vertex| vertex.id == request.vertex_id)
                    .ok_or_else(|| missing_element(&request.vertex_id))?;
                apply_optional_patch(&mut vertex.name, request.name);
                apply_optional_patch(&mut vertex.shared_state, request.shared_state);
                apply_vec_patch(&mut vertex.actions, request.actions);
                apply_vec_patch(&mut vertex.requirements, request.requirements);
                apply_map_patch(&mut vertex.properties, request.properties);
                Ok(vertex.clone())
            })?;
        Ok(VertexResult { vertex, revision })
    }

    pub fn update_edge(&self, request: UpdateEdge) -> ServiceResult<EdgeResult> {
        let (edge, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                let index = model
                    .edges
                    .iter()
                    .position(|edge| edge.id == request.edge_id)
                    .ok_or_else(|| missing_element(&request.edge_id))?;
                let mut edge = model.edges[index].clone();
                apply_optional_patch(&mut edge.name, request.name);
                apply_optional_patch(&mut edge.source_vertex_id, request.source_vertex_id);
                apply_optional_patch(&mut edge.guard, request.guard);
                apply_vec_patch(&mut edge.actions, request.actions);
                apply_vec_patch(&mut edge.requirements, request.requirements);
                apply_map_patch(&mut edge.properties, request.properties);
                apply_number_patch(&mut edge.weight, request.weight);
                apply_number_patch(&mut edge.dependency, request.dependency);
                match request.target_vertex_id {
                    FieldPatch::Keep => {}
                    FieldPatch::Set(id) => edge.target_vertex_id = Some(id),
                    FieldPatch::Clear => {
                        return Err(ServiceError::new(
                            ServiceErrorCode::MissingTargetVertex,
                            "An edge must have a target vertex",
                        ));
                    }
                }
                validate_weight(edge.weight)?;
                validate_dependency(edge.dependency)?;
                validate_edge_references(model, &edge)?;
                model.edges[index] = edge.clone();
                Ok(edge)
            })?;
        Ok(EdgeResult { edge, revision })
    }

    pub fn remove_element(&self, request: RemoveElement) -> ServiceResult<RemoveResult> {
        let (removed_ids, revision) =
            self.mutate(&request.draft_id, request.expected_revision, |model| {
                remove_element(model, &request)
            })?;
        Ok(RemoveResult {
            removed_ids,
            revision,
        })
    }

    pub fn export_model(&self, draft_id: &DraftId) -> ServiceResult<ExportedModel> {
        let (model, revision) = self.read(draft_id, |draft| {
            serde_json::to_value(&draft.document)
                .map_err(|error| ServiceError::internal(error.to_string()))
        })?;
        Ok(ExportedModel { model, revision })
    }

    pub fn snapshot(
        &self,
        draft_id: &DraftId,
        expected_revision: Option<u64>,
    ) -> ServiceResult<DraftSnapshot> {
        let exported = self.export_model(draft_id)?;
        check_revision(exported.revision, expected_revision)?;
        Ok(DraftSnapshot {
            model: exported.model,
            revision: exported.revision,
        })
    }

    pub fn validate(&self, draft_id: &DraftId) -> ServiceResult<DraftValidation> {
        let exported = self.export_model(draft_id)?;
        let ValidationResult {
            valid: _,
            mut issues,
        } = validate_model(&exported.model)?;
        match exported.model["models"][0]
            .get("generator")
            .and_then(Value::as_str)
        {
            None => issues.push(ValidationIssue {
                message: "Model has no generator specified".to_string(),
            }),
            Some(generator) => {
                if let Err(error) = graphwalker_dsl::generator::parse_generator(generator) {
                    issues.push(ValidationIssue {
                        message: error.to_string(),
                    });
                }
            }
        }
        Ok(DraftValidation {
            valid: issues.is_empty(),
            issues,
            revision: exported.revision,
        })
    }

    pub fn discard(&self, draft_id: &DraftId) -> ServiceResult<DiscardResult> {
        let entry = self.entry(draft_id)?;
        let mut draft = entry.lock().map_err(draft_lock_error)?;
        if draft.lifecycle == DraftLifecycle::Expired
            || draft.last_access.elapsed() >= self.limits.idle_timeout
        {
            draft.lifecycle = DraftLifecycle::Expired;
            drop(draft);
            self.record_expired(draft_id)?;
            return Err(expired_error(draft_id));
        }
        if draft.lifecycle != DraftLifecycle::Active {
            return Err(not_found(draft_id));
        }
        draft.lifecycle = DraftLifecycle::Discarded;
        drop(draft);

        let mut state = self.state.write().map_err(registry_lock_error)?;
        state.drafts.remove(draft_id);
        Ok(DiscardResult { discarded: true })
    }

    pub fn len(&self) -> usize {
        self.state
            .read()
            .map(|state| state.drafts.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn mutate<T>(
        &self,
        draft_id: &DraftId,
        expected_revision: Option<u64>,
        operation: impl FnOnce(&mut JsonModel) -> ServiceResult<T>,
    ) -> ServiceResult<(T, u64)> {
        let entry = self.entry(draft_id)?;
        let mut draft = entry.lock().map_err(draft_lock_error)?;
        if self.mark_expired_if_needed(&mut draft) {
            drop(draft);
            self.record_expired(draft_id)?;
            return Err(expired_error(draft_id));
        }
        ensure_active(draft_id, &draft)?;
        check_revision(draft.revision, expected_revision)?;

        let mut candidate = draft.document.clone();
        let result = operation(&mut candidate.models[0])?;
        self.check_model_limits(&candidate.models[0])?;
        draft.document = candidate;
        draft.revision += 1;
        draft.last_access = Instant::now();
        Ok((result, draft.revision))
    }

    fn read<T>(
        &self,
        draft_id: &DraftId,
        operation: impl FnOnce(&Draft) -> ServiceResult<T>,
    ) -> ServiceResult<(T, u64)> {
        let entry = self.entry(draft_id)?;
        let mut draft = entry.lock().map_err(draft_lock_error)?;
        if self.mark_expired_if_needed(&mut draft) {
            drop(draft);
            self.record_expired(draft_id)?;
            return Err(expired_error(draft_id));
        }
        ensure_active(draft_id, &draft)?;
        let result = operation(&draft)?;
        draft.last_access = Instant::now();
        Ok((result, draft.revision))
    }

    fn entry(&self, draft_id: &DraftId) -> ServiceResult<Arc<Mutex<Draft>>> {
        let state = self.state.read().map_err(registry_lock_error)?;
        if let Some(entry) = state.drafts.get(draft_id) {
            Ok(Arc::clone(entry))
        } else if state.expired.contains(draft_id) {
            Err(expired_error(draft_id))
        } else {
            Err(not_found(draft_id))
        }
    }

    fn mark_expired_if_needed(&self, draft: &mut Draft) -> bool {
        if draft.lifecycle == DraftLifecycle::Expired {
            return true;
        }
        if draft.lifecycle != DraftLifecycle::Active
            || draft.last_access.elapsed() < self.limits.idle_timeout
        {
            return false;
        }
        draft.lifecycle = DraftLifecycle::Expired;
        true
    }

    fn record_expired(&self, draft_id: &DraftId) -> ServiceResult<()> {
        let mut state = self.state.write().map_err(registry_lock_error)?;
        state.drafts.remove(draft_id);
        remember_expired(&mut state, draft_id.clone());
        Ok(())
    }

    fn remove_expired_locked(&self, state: &mut RegistryState) -> ServiceResult<()> {
        let mut expired = Vec::new();
        for (id, entry) in &state.drafts {
            let mut draft = entry.lock().map_err(draft_lock_error)?;
            if draft.lifecycle == DraftLifecycle::Active
                && draft.last_access.elapsed() >= self.limits.idle_timeout
            {
                draft.lifecycle = DraftLifecycle::Expired;
                expired.push(id.clone());
            }
        }
        for id in expired {
            state.drafts.remove(&id);
            remember_expired(state, id);
        }
        Ok(())
    }

    fn check_model_limits(&self, model: &JsonModel) -> ServiceResult<()> {
        if model.vertices.len() > self.limits.max_vertices_per_draft {
            return Err(ServiceError::new(
                ServiceErrorCode::ModelLimitReached,
                format!(
                    "Vertex limit of {} has been reached",
                    self.limits.max_vertices_per_draft
                ),
            ));
        }
        if model.edges.len() > self.limits.max_edges_per_draft {
            return Err(ServiceError::new(
                ServiceErrorCode::ModelLimitReached,
                format!(
                    "Edge limit of {} has been reached",
                    self.limits.max_edges_per_draft
                ),
            ));
        }
        Ok(())
    }
}

fn remember_expired(state: &mut RegistryState, draft_id: DraftId) {
    const MAX_EXPIRED_TOMBSTONES: usize = 1_024;

    if state.expired.insert(draft_id.clone()) {
        state.expired_order.push_back(draft_id);
    }
    while state.expired_order.len() > MAX_EXPIRED_TOMBSTONES {
        if let Some(oldest) = state.expired_order.pop_front() {
            state.expired.remove(&oldest);
        }
    }
}

fn validate_nonempty_id(id: &str, kind: &str) -> ServiceResult<()> {
    if id.is_empty() {
        Err(ServiceError::new(
            ServiceErrorCode::InvalidElement,
            format!("The {kind} ID cannot be empty"),
        ))
    } else {
        Ok(())
    }
}

fn ensure_active(draft_id: &DraftId, draft: &Draft) -> ServiceResult<()> {
    match draft.lifecycle {
        DraftLifecycle::Active => Ok(()),
        DraftLifecycle::Expired => Err(expired_error(draft_id)),
        DraftLifecycle::Discarded => Err(not_found(draft_id)),
    }
}

fn ensure_unique_element_id(model: &JsonModel, id: &str) -> ServiceResult<()> {
    if model.vertices.iter().any(|vertex| vertex.id == id)
        || model.edges.iter().any(|edge| edge.id == id)
    {
        Err(ServiceError::new(
            ServiceErrorCode::DuplicateElementId,
            format!("Element ID '{id}' already exists"),
        ))
    } else {
        Ok(())
    }
}

fn ensure_element_exists(model: &JsonModel, id: &str) -> ServiceResult<()> {
    if model.vertices.iter().any(|vertex| vertex.id == id)
        || model.edges.iter().any(|edge| edge.id == id)
    {
        Ok(())
    } else {
        Err(missing_element(id))
    }
}

fn validate_edge_references(model: &JsonModel, edge: &JsonEdge) -> ServiceResult<()> {
    if let Some(source) = &edge.source_vertex_id {
        ensure_vertex_exists(model, source)?;
    }
    let target = edge.target_vertex_id.as_deref().ok_or_else(|| {
        ServiceError::new(
            ServiceErrorCode::MissingTargetVertex,
            "An edge must have a target vertex",
        )
    })?;
    ensure_vertex_exists(model, target)
}

fn ensure_vertex_exists(model: &JsonModel, id: &str) -> ServiceResult<()> {
    if model.vertices.iter().any(|vertex| vertex.id == id) {
        Ok(())
    } else {
        Err(ServiceError::new(
            ServiceErrorCode::UnknownVertex,
            format!("Vertex '{id}' does not exist"),
        ))
    }
}

fn validate_weight(weight: Option<f64>) -> ServiceResult<()> {
    if weight.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
        Err(ServiceError::new(
            ServiceErrorCode::InvalidWeight,
            "Weight must be between 0 and 1",
        ))
    } else {
        Ok(())
    }
}

fn validate_dependency(dependency: Option<i32>) -> ServiceResult<()> {
    if dependency.is_some_and(|value| !(0..=100).contains(&value)) {
        Err(ServiceError::new(
            ServiceErrorCode::InvalidDependency,
            "Dependency must be between 0 and 100",
        ))
    } else {
        Ok(())
    }
}

fn check_revision(current: u64, expected: Option<u64>) -> ServiceResult<()> {
    if expected.is_some_and(|revision| revision != current) {
        Err(ServiceError::new(
            ServiceErrorCode::RevisionConflict,
            format!(
                "Draft revision conflict: expected {}, current {current}",
                expected.unwrap()
            ),
        ))
    } else {
        Ok(())
    }
}

fn apply_optional_patch<T>(target: &mut Option<T>, patch: FieldPatch<T>) {
    match patch {
        FieldPatch::Keep => {}
        FieldPatch::Set(value) => *target = Some(value),
        FieldPatch::Clear => *target = None,
    }
}

fn apply_number_patch<T>(target: &mut Option<T>, patch: FieldPatch<T>) {
    apply_optional_patch(target, patch);
}

fn apply_vec_patch<T>(target: &mut Vec<T>, patch: FieldPatch<Vec<T>>) {
    match patch {
        FieldPatch::Keep => {}
        FieldPatch::Set(value) => *target = value,
        FieldPatch::Clear => target.clear(),
    }
}

fn apply_map_patch(target: &mut HashMap<String, Value>, patch: FieldPatch<HashMap<String, Value>>) {
    match patch {
        FieldPatch::Keep => {}
        FieldPatch::Set(value) => *target = value,
        FieldPatch::Clear => target.clear(),
    }
}

fn remove_element(model: &mut JsonModel, request: &RemoveElement) -> ServiceResult<Vec<String>> {
    if let Some(index) = model
        .edges
        .iter()
        .position(|edge| edge.id == request.element_id)
    {
        ensure_references_can_be_cleaned(
            model,
            &[request.element_id.as_str()],
            request.cleanup_references,
        )?;
        let removed = model.edges.remove(index).id;
        cleanup_references(model, &[removed.as_str()]);
        return Ok(vec![removed]);
    }

    let vertex_index = model
        .vertices
        .iter()
        .position(|vertex| vertex.id == request.element_id)
        .ok_or_else(|| missing_element(&request.element_id))?;
    let incident_edges = model
        .edges
        .iter()
        .filter(|edge| {
            edge.source_vertex_id.as_deref() == Some(request.element_id.as_str())
                || edge.target_vertex_id.as_deref() == Some(request.element_id.as_str())
        })
        .map(|edge| edge.id.clone())
        .collect::<Vec<_>>();
    if !incident_edges.is_empty() && !request.cascade {
        return Err(ServiceError::new(
            ServiceErrorCode::ReferencedElement,
            format!(
                "Vertex '{}' has connected edges; set cascade to remove it",
                request.element_id
            ),
        ));
    }
    let mut removed_ids = vec![request.element_id.clone()];
    removed_ids.extend(incident_edges);
    let removed_refs = removed_ids.iter().map(String::as_str).collect::<Vec<_>>();
    ensure_references_can_be_cleaned(model, &removed_refs, request.cleanup_references)?;

    model.vertices.remove(vertex_index);
    model.edges.retain(|edge| !removed_ids.contains(&edge.id));
    cleanup_references(model, &removed_refs);
    Ok(removed_ids)
}

fn ensure_references_can_be_cleaned(
    model: &JsonModel,
    removed_ids: &[&str],
    cleanup: bool,
) -> ServiceResult<()> {
    let referenced = model
        .start_element_id
        .as_deref()
        .is_some_and(|id| removed_ids.contains(&id))
        || model
            .predefined_path_edge_ids
            .iter()
            .any(|id| removed_ids.contains(&id.as_str()));
    if referenced && !cleanup {
        Err(ServiceError::new(
            ServiceErrorCode::ReferencedElement,
            "Element is referenced by model metadata; set cleanup_references to remove it",
        ))
    } else {
        Ok(())
    }
}

fn cleanup_references(model: &mut JsonModel, removed_ids: &[&str]) {
    if model
        .start_element_id
        .as_deref()
        .is_some_and(|id| removed_ids.contains(&id))
    {
        model.start_element_id = None;
    }
    model
        .predefined_path_edge_ids
        .retain(|id| !removed_ids.contains(&id.as_str()));
}

fn missing_element(id: &str) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::InvalidElement,
        format!("Element '{id}' does not exist"),
    )
}

fn not_found(draft_id: &DraftId) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::DraftNotFound,
        format!("Draft '{}' was not found", draft_id.as_str()),
    )
}

fn expired_error(draft_id: &DraftId) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::DraftExpired,
        format!("Draft '{}' has expired", draft_id.as_str()),
    )
}

fn registry_lock_error<T>(error: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::internal(format!("Draft registry lock was poisoned: {error}"))
}

fn draft_lock_error<T>(error: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::DraftUnavailable,
        format!("Draft lock was poisoned: {error}"),
    )
}
