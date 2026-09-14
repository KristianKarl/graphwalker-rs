mod dto;

use graphwalker_service as service;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::Serialize;
use serde_json::json;

use dto::*;

/// MCP protocol adapter for GraphWalker model authoring and execution.
#[derive(Clone, Default)]
pub struct GraphWalkerMcp {
    drafts: service::DraftRegistry,
    executions: service::ExecutionRegistry,
}

fn patch<T>(value: McpPatch<T>) -> service::FieldPatch<T> {
    match value {
        McpPatch::Keep => service::FieldPatch::Keep,
        McpPatch::Set(value) => service::FieldPatch::Set(value),
        McpPatch::Clear => service::FieldPatch::Clear,
    }
}

fn structured<T: Serialize>(value: T) -> CallToolResult {
    match serde_json::to_value(value) {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => adapter_error(
            "internal",
            format!("Could not serialize tool result: {error}"),
        ),
    }
}

fn adapter_error(code: impl Into<String>, message: impl Into<String>) -> CallToolResult {
    let error = ToolErrorOutput {
        code: code.into(),
        message: message.into(),
    };
    CallToolResult::structured_error(
        serde_json::to_value(error).unwrap_or_else(
            |_| json!({ "code": "internal", "message": "Internal MCP adapter error" }),
        ),
    )
}

fn service_error(error: service::ServiceError) -> CallToolResult {
    let code = serde_json::to_value(error.code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internal".to_string());
    adapter_error(code, error.message)
}

fn result<T: Serialize>(value: Result<T, service::ServiceError>) -> CallToolResult {
    match value {
        Ok(value) => structured(value),
        Err(error) => service_error(error),
    }
}

fn vertex_output(vertex: graphwalker_io::json::JsonVertex) -> VertexOutput {
    VertexOutput {
        id: vertex.id,
        name: vertex.name,
        shared_state: vertex.shared_state,
        actions: vertex.actions,
        requirements: vertex.requirements,
        properties: vertex.properties,
    }
}

fn edge_output(edge: graphwalker_io::json::JsonEdge) -> Result<EdgeOutput, service::ServiceError> {
    let target_vertex_id = edge.target_vertex_id.ok_or_else(|| {
        service::ServiceError::new(
            service::ServiceErrorCode::Internal,
            "The service returned an edge without a target vertex",
        )
    })?;
    Ok(EdgeOutput {
        id: edge.id,
        name: edge.name,
        source_vertex_id: edge.source_vertex_id,
        target_vertex_id,
        guard: edge.guard,
        actions: edge.actions,
        requirements: edge.requirements,
        properties: edge.properties,
        weight: edge.weight,
        dependency: edge.dependency,
    })
}

fn statistics_output(value: service::ExecutionStatistics) -> StatisticsOutput {
    StatisticsOutput {
        total_vertices: value.total_vertices,
        total_edges: value.total_edges,
        visited_vertices: value.visited_vertices,
        visited_edges: value.visited_edges,
        unvisited_vertices: value.unvisited_vertices,
        unvisited_edges: value.unvisited_edges,
        vertex_coverage: value.vertex_coverage,
        edge_coverage: value.edge_coverage,
    }
}

#[tool_router]
impl GraphWalkerMcp {
    #[tool(
        description = "Check that the local GraphWalker MCP server is running",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    fn health(&self) -> rmcp::Json<HealthOutput> {
        rmcp::Json(HealthOutput {
            status: "ok",
            server_version: env!("CARGO_PKG_VERSION"),
        })
    }

    #[tool(
        description = "Create an empty, process-local GraphWalker model draft",
        output_schema = rmcp::handler::server::tool::schema_for_type::<CreateModelOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn create_model(&self, Parameters(input): Parameters<CreateModelInput>) -> CallToolResult {
        result(
            self.drafts
                .create_model(service::CreateModel {
                    model_id: input.model_id,
                    name: input.name,
                    generator: input.generator,
                    actions: input.actions,
                    requirements: input.requirements,
                    properties: input.properties,
                })
                .map(|created| CreateModelOutput {
                    draft_id: created.draft_id.to_string(),
                    model_id: created.model_id,
                    revision: created.revision,
                }),
        )
    }

    #[tool(
        description = "Add one vertex to a GraphWalker model draft and increment its revision",
        output_schema = rmcp::handler::server::tool::schema_for_type::<VertexMutationOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn add_vertex(&self, Parameters(input): Parameters<AddVertexInput>) -> CallToolResult {
        result(
            self.drafts
                .add_vertex(service::AddVertex {
                    draft_id: input.draft_id.as_str().into(),
                    id: input.id,
                    name: input.name,
                    shared_state: input.shared_state,
                    actions: input.actions,
                    requirements: input.requirements,
                    properties: input.properties,
                    expected_revision: input.expected_revision,
                })
                .map(|added| VertexMutationOutput {
                    vertex: vertex_output(added.vertex),
                    revision: added.revision,
                }),
        )
    }

    #[tool(
        description = "Add one edge to a GraphWalker model draft and increment its revision",
        output_schema = rmcp::handler::server::tool::schema_for_type::<EdgeMutationOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn add_edge(&self, Parameters(input): Parameters<AddEdgeInput>) -> CallToolResult {
        let added = self
            .drafts
            .add_edge(service::AddEdge {
                draft_id: input.draft_id.as_str().into(),
                id: input.id,
                name: input.name,
                source_vertex_id: input.source_vertex_id,
                target_vertex_id: Some(input.target_vertex_id),
                guard: input.guard,
                actions: input.actions,
                requirements: input.requirements,
                properties: input.properties,
                weight: input.weight,
                dependency: input.dependency,
                expected_revision: input.expected_revision,
            })
            .and_then(|added| {
                Ok(EdgeMutationOutput {
                    edge: edge_output(added.edge)?,
                    revision: added.revision,
                })
            });
        result(added)
    }

    #[tool(
        description = "Patch GraphWalker model metadata in a draft and increment its revision",
        output_schema = rmcp::handler::server::tool::schema_for_type::<ModelMutationOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn update_model(&self, Parameters(input): Parameters<UpdateModelInput>) -> CallToolResult {
        result(
            self.drafts
                .update_model(service::UpdateModel {
                    draft_id: input.draft_id.as_str().into(),
                    name: patch(input.name),
                    generator: patch(input.generator),
                    start_element_id: patch(input.start_element_id),
                    actions: patch(input.actions),
                    requirements: patch(input.requirements),
                    properties: patch(input.properties),
                    predefined_path_edge_ids: patch(input.predefined_path_edge_ids),
                    expected_revision: input.expected_revision,
                })
                .and_then(|updated| {
                    serde_json::to_value(updated.model)
                        .map(|model| ModelMutationOutput {
                            model,
                            revision: updated.revision,
                        })
                        .map_err(|error| {
                            service::ServiceError::new(
                                service::ServiceErrorCode::Internal,
                                error.to_string(),
                            )
                        })
                }),
        )
    }

    #[tool(
        description = "Patch one vertex in a GraphWalker model draft and increment its revision",
        output_schema = rmcp::handler::server::tool::schema_for_type::<VertexMutationOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn update_vertex(&self, Parameters(input): Parameters<UpdateVertexInput>) -> CallToolResult {
        result(
            self.drafts
                .update_vertex(service::UpdateVertex {
                    draft_id: input.draft_id.as_str().into(),
                    vertex_id: input.vertex_id,
                    name: patch(input.name),
                    shared_state: patch(input.shared_state),
                    actions: patch(input.actions),
                    requirements: patch(input.requirements),
                    properties: patch(input.properties),
                    expected_revision: input.expected_revision,
                })
                .map(|updated| VertexMutationOutput {
                    vertex: vertex_output(updated.vertex),
                    revision: updated.revision,
                }),
        )
    }

    #[tool(
        description = "Patch one edge in a GraphWalker model draft and increment its revision",
        output_schema = rmcp::handler::server::tool::schema_for_type::<EdgeMutationOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn update_edge(&self, Parameters(input): Parameters<UpdateEdgeInput>) -> CallToolResult {
        let updated = self
            .drafts
            .update_edge(service::UpdateEdge {
                draft_id: input.draft_id.as_str().into(),
                edge_id: input.edge_id,
                name: patch(input.name),
                source_vertex_id: patch(input.source_vertex_id),
                target_vertex_id: patch(input.target_vertex_id),
                guard: patch(input.guard),
                actions: patch(input.actions),
                requirements: patch(input.requirements),
                properties: patch(input.properties),
                weight: patch(input.weight),
                dependency: patch(input.dependency),
                expected_revision: input.expected_revision,
            })
            .and_then(|updated| {
                Ok(EdgeMutationOutput {
                    edge: edge_output(updated.edge)?,
                    revision: updated.revision,
                })
            });
        result(updated)
    }

    #[tool(
        description = "Remove an element from a GraphWalker draft; cascade and reference cleanup require explicit opt-in",
        output_schema = rmcp::handler::server::tool::schema_for_type::<RemoveElementOutput>(),
        annotations(read_only_hint = false, destructive_hint = true, open_world_hint = false)
    )]
    fn remove_element(&self, Parameters(input): Parameters<RemoveElementInput>) -> CallToolResult {
        result(
            self.drafts
                .remove_element(service::RemoveElement {
                    draft_id: input.draft_id.as_str().into(),
                    element_id: input.element_id,
                    cascade: input.cascade,
                    cleanup_references: input.cleanup_references,
                    expected_revision: input.expected_revision,
                })
                .map(|removed| RemoveElementOutput {
                    removed_ids: removed.removed_ids,
                    revision: removed.revision,
                }),
        )
    }

    #[tool(
        description = "Export a GraphWalker model draft as a canonical JSON object without changing it",
        output_schema = rmcp::handler::server::tool::schema_for_type::<ExportModelOutput>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    fn export_model(&self, Parameters(input): Parameters<DraftInput>) -> CallToolResult {
        result(
            self.drafts
                .export_model(&input.draft_id.as_str().into())
                .map(|exported| ExportModelOutput {
                    model: exported.model,
                    revision: exported.revision,
                }),
        )
    }

    #[tool(
        description = "Permanently discard a process-local GraphWalker model draft",
        output_schema = rmcp::handler::server::tool::schema_for_type::<DiscardModelOutput>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    fn discard_model(&self, Parameters(input): Parameters<DraftInput>) -> CallToolResult {
        result(
            self.drafts
                .discard(&input.draft_id.as_str().into())
                .map(|discarded| DiscardModelOutput {
                    discarded: discarded.discarded,
                }),
        )
    }

    #[tool(
        description = "Validate exactly one inline GraphWalker JSON model or process-local draft without changing it",
        output_schema = rmcp::handler::server::tool::schema_for_type::<ValidateModelOutput>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    fn validate_model(&self, Parameters(input): Parameters<ValidateModelInput>) -> CallToolResult {
        match (input.model, input.draft_id) {
            (Some(model), None) => result(service::validate_model(&model).map(|validation| {
                ValidateModelOutput {
                    valid: validation.valid,
                    issues: validation
                        .issues
                        .into_iter()
                        .map(|issue| issue.message)
                        .collect(),
                    revision: None,
                }
            })),
            (None, Some(draft_id)) => result(self.drafts.validate(&draft_id.as_str().into()).map(
                |validation| {
                    ValidateModelOutput {
                        valid: validation.valid,
                        issues: validation
                            .issues
                            .into_iter()
                            .map(|issue| issue.message)
                            .collect(),
                        revision: Some(validation.revision),
                    }
                },
            )),
            _ => adapter_error(
                "invalid_input",
                "Provide exactly one of 'model' or 'draft_id'",
            ),
        }
    }

    #[tool(
        description = "Convert an inline GraphML/yEd document into canonical GraphWalker JSON without storing it",
        output_schema = rmcp::handler::server::tool::schema_for_type::<ConvertGraphmlOutput>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    fn convert_graphml(
        &self,
        Parameters(input): Parameters<ConvertGraphmlInput>,
    ) -> CallToolResult {
        result(
            service::convert_graphml(&input.graphml).map(|converted| ConvertGraphmlOutput {
                model: converted.model,
            }),
        )
    }

    #[tool(
        description = "Start isolated GraphWalker execution state from exactly one inline model or draft snapshot",
        output_schema = rmcp::handler::server::tool::schema_for_type::<StartExecutionOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn start_execution(
        &self,
        Parameters(input): Parameters<StartExecutionInput>,
    ) -> CallToolResult {
        let (model, source_revision) = match (input.model, input.draft_id) {
            (Some(model), None) if input.revision.is_none() => {
                let validation = match service::validate_model(&model) {
                    Ok(validation) => validation,
                    Err(error) => return service_error(error),
                };
                if !validation.valid {
                    return adapter_error(
                        "invalid_model",
                        validation
                            .issues
                            .into_iter()
                            .map(|issue| issue.message)
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                }
                (model, None)
            }
            (None, Some(draft_id)) => {
                let snapshot = match self
                    .drafts
                    .snapshot(&draft_id.as_str().into(), input.revision)
                {
                    Ok(snapshot) => snapshot,
                    Err(error) => return service_error(error),
                };
                let validation = match self.drafts.validate(&draft_id.as_str().into()) {
                    Ok(validation) => validation,
                    Err(error) => return service_error(error),
                };
                if validation.revision != snapshot.revision {
                    return service_error(service::ServiceError::new(
                        service::ServiceErrorCode::RevisionConflict,
                        format!(
                            "Draft changed from revision {} to {} while starting execution",
                            snapshot.revision, validation.revision
                        ),
                    ));
                }
                if !validation.valid {
                    return adapter_error(
                        "invalid_model",
                        validation
                            .issues
                            .into_iter()
                            .map(|issue| issue.message)
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                }
                (snapshot.model, Some(snapshot.revision))
            }
            (Some(_), None) => {
                return adapter_error("invalid_input", "'revision' is only valid with 'draft_id'")
            }
            _ => {
                return adapter_error(
                    "invalid_input",
                    "Provide exactly one of 'model' or 'draft_id'",
                )
            }
        };
        result(
            self.executions
                .start(service::StartExecution {
                    model,
                    seed: input.seed,
                    global_data: input.global_data,
                })
                .map(|started| StartExecutionOutput {
                    execution_id: started.execution_id.to_string(),
                    seed: started.seed,
                    source_revision,
                }),
        )
    }

    #[tool(
        description = "Advance a GraphWalker execution by at most one element",
        output_schema = rmcp::handler::server::tool::schema_for_type::<NextStepOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn next_step(&self, Parameters(input): Parameters<ExecutionInput>) -> CallToolResult {
        result(
            self.executions
                .next_step(&input.execution_id.as_str().into())
                .map(|step| NextStepOutput {
                    completed: step.completed,
                    element: step.element.map(|element| StepElementOutput {
                        id: element.id,
                        name: element.name,
                        model_id: element.model_id,
                        kind: match element.kind {
                            service::ElementKind::Edge => "edge",
                            service::ElementKind::Vertex => "vertex",
                        }
                        .to_string(),
                        data: element.data,
                        visited_count: element.visited_count,
                        total_count: element.total_count,
                        stop_condition_fulfillment: element.stop_condition_fulfillment,
                    }),
                }),
        )
    }

    #[tool(
        description = "Inspect GraphWalker execution data and coverage without advancing it",
        output_schema = rmcp::handler::server::tool::schema_for_type::<ExecutionStatusOutput>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    fn execution_status(
        &self,
        Parameters(input): Parameters<ExecutionStatusInput>,
    ) -> CallToolResult {
        let execution_id: service::ExecutionId = input.execution_id.as_str().into();
        let status = match self.executions.status(&execution_id) {
            Ok(status) => status,
            Err(error) => return service_error(error),
        };
        let statistics = match self.executions.statistics(&execution_id) {
            Ok(statistics) => statistics,
            Err(error) => return service_error(error),
        };
        let elements = if input.include_elements {
            match self.executions.elements(&execution_id) {
                Ok(elements) => Some(
                    elements
                        .into_iter()
                        .map(|element| ElementStatusOutput {
                            model_id: element.model_id,
                            element_id: element.element_id,
                            visited_count: element.visited_count,
                        })
                        .collect(),
                ),
                Err(error) => return service_error(error),
            }
        } else {
            None
        };
        structured(ExecutionStatusOutput {
            has_next: status.has_next,
            data: status.data,
            statistics: statistics_output(statistics),
            elements,
        })
    }

    #[tool(
        description = "Execute a data script in the current GraphWalker execution context",
        output_schema = rmcp::handler::server::tool::schema_for_type::<SetExecutionDataOutput>(),
        annotations(read_only_hint = false, destructive_hint = false, open_world_hint = false)
    )]
    fn set_execution_data(
        &self,
        Parameters(input): Parameters<SetExecutionDataInput>,
    ) -> CallToolResult {
        result(
            self.executions
                .set_data(&input.execution_id.as_str().into(), input.script)
                .map(|updated| SetExecutionDataOutput { data: updated.data }),
        )
    }

    #[tool(
        description = "Restart a GraphWalker execution using its original model, seed, and global data",
        output_schema = rmcp::handler::server::tool::schema_for_type::<RestartExecutionOutput>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn restart_execution(&self, Parameters(input): Parameters<ExecutionInput>) -> CallToolResult {
        result(
            self.executions
                .restart(&input.execution_id.as_str().into())
                .map(|restarted| RestartExecutionOutput {
                    restarted: true,
                    seed: restarted.seed,
                }),
        )
    }

    #[tool(
        description = "Close and permanently release one process-local GraphWalker execution",
        output_schema = rmcp::handler::server::tool::schema_for_type::<CloseExecutionOutput>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    fn close_execution(&self, Parameters(input): Parameters<ExecutionInput>) -> CallToolResult {
        result(
            self.executions
                .close(&input.execution_id.as_str().into())
                .map(|()| CloseExecutionOutput { closed: true }),
        )
    }
}

#[tool_handler]
impl ServerHandler for GraphWalkerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Build, revise, export, validate, and execute GraphWalker model-based tests. Draft and execution IDs are process-local; export models before disconnecting if they must persist.",
            )
    }
}
