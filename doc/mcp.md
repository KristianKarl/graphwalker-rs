---
layout: default
title: MCP Server Plan
nav_order: 11
---

# MCP Server Plan

## Purpose

Add a Model Context Protocol (MCP) server that lets AI clients validate GraphWalker models and drive model-based test executions. The first release should expose GraphWalker's existing behavior without turning the MCP layer into a second execution engine.

The initial user workflows are:

1. Build a graph model incrementally by creating a draft and adding vertices and edges.
2. Inspect, revise, validate, and export the draft as standard GraphWalker JSON.
3. Validate an existing JSON model and receive actionable issues.
4. Convert a GraphML/yEd model to GraphWalker JSON.
5. Start a deterministic execution from an inline model or completed draft.
6. Request one model element at a time, perform that action in another system, and feed resulting data back into GraphWalker.
7. Inspect coverage and close or restart an execution.

## Design decisions

### Use the official Rust SDK

Use the official [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) SDK instead of implementing JSON-RPC or MCP framing in this repository. Phase 0 pins the released `rmcp` 3.3.0 crate in `Cargo.toml` and `Cargo.lock`, with default features disabled and only `macros`, `server`, and `transport-io` enabled.

The workspace MSRV is Rust 1.88, matching `rmcp` 3.3.0. It is declared through `[workspace.package]` and inherited by every workspace crate. README and getting-started prerequisites carry the same version. Do not use an unpinned Git dependency for a released GraphWalker version.

### Start with local stdio transport

The MVP will run as a `graphwalker-mcp` child process over standard input/output. Stdio is the normal local MCP transport and has a substantially smaller security and deployment surface than a remotely reachable server.

All protocol messages go to stdout. Diagnostics use stderr through `tracing`; ordinary logging must never corrupt the stdout protocol stream.

Streamable HTTP is a follow-up. Adding it requires an explicit decision about binding defaults, authentication/authorization, TLS termination, request limits, rate limiting, and session storage. The obsolete standalone HTTP+SSE transport is out of scope.

### Expose tools first

The MVP advertises the MCP `tools` capability only. Prompts, resources, subscriptions, and tasks are not required for the initial workflows. Static documentation can be exposed as MCP resources later if real clients benefit from it.

Tool definitions and their order are static and deterministic. Every tool has a generated JSON Schema, a concise description that states its side effects, and structured JSON output.

### Keep execution state explicit

Validation and conversion are stateless. Test traversal is stateful, so `start_execution` returns an opaque `execution_id`; every later execution tool requires that ID. This makes state visible to the calling model, permits several independent executions in one server process, and remains compatible with stateless HTTP if that transport is added later.

Each execution owns its model, generator state, seed, variables, visit counts, and original restart configuration. Calls for the same execution are serialized. Calls for different executions may proceed independently.

Model construction is also stateful. `create_model` returns an opaque `draft_id` for a mutable draft; authoring tools require that ID. This is deliberately distinct from the graph's own model ID. A model draft is separate from an execution and can be edited until it is explicitly discarded or expires. Starting an execution takes an immutable snapshot, so later draft edits cannot change a run already in progress.

### Support incremental model authoring

The MCP server should do more than accept a prebuilt JSON document. It should expose GraphWalker's existing model-builder capabilities as a safe draft workflow. This lets an AI client construct a model over several calls while the server enforces IDs, references, value ranges, and the GraphWalker JSON shape.

The service stores drafts in the canonical GraphWalker data-transfer representation used by `graphwalker-io`; it must not invent a second model format. Exported output has the same root `{ "models": [...] }` structure accepted by the CLI, REST API, and Studio.

Draft mutations are atomic. A failed mutation leaves the draft unchanged. Every successful mutation increments a numeric `revision`, returned with the result. Mutating tools accept an optional `expected_revision`; a mismatch returns `model_revision_conflict` instead of overwriting a newer edit. This also gives clients a safe basis for retries and concurrent editing.

IDs supplied by the caller must be unique across vertices and edges in a graph. If omitted, the server generates and returns a stable ID. Edge targets must refer to an existing vertex; the source is optional only for a start edge. Deleting a vertex with connected edges fails unless `cascade: true` is explicitly provided.

### Share a typed service, not transport response JSON

The current `graphwalker-restful/src/actor.rs` is already the common boundary used by the REST and WebSocket adapters, but it lives in a transport-named crate and returns REST-shaped `serde_json::Value` objects. Reusing it directly would couple MCP behavior to fields such as `"result": "ok"` and the string-valued REST `hasNext` response.

Extract its model loading, checking, conversion, stepping, restart, data, and statistics behavior into a new transport-neutral `graphwalker-service` workspace crate. Its public API should use Rust request/response/error types. The existing REST/WebSocket actor becomes an adapter that maps those types to its current wire format, preserving existing APIs. The MCP crate maps the same types to MCP results.

`graphwalker-core` remains the domain and traversal engine; it must not depend on I/O, the model checker, HTTP, or MCP.

### Keep REST additions separate

The shared-service extraction may change REST internals, but it must not add or alter REST endpoints. Existing REST and WebSocket contracts are preserved and verified before MCP authoring tools are introduced.

If graph-construction operations are also exposed over REST, implement them only in a later, dedicated phase and change set. That work gets its own endpoint design, documentation, compatibility review, and REST integration tests. MCP tool handlers and REST handlers may share `graphwalker-service`, but neither transport may call through or depend on the other transport's adapter types.

The MCP MVP therefore does not depend on new REST endpoints. Failure or deferral of the REST graph-authoring phase must not block shipping the local MCP server.

```text
graphwalker-core / dsl / io / model-checker
                    |
           graphwalker-service
              /            \
 graphwalker-restful      graphwalker-mcp
          |                     |
 REST + WebSocket             stdio
```

## Proposed MCP tools

Names are intentionally GraphWalker-specific in their descriptions, while remaining short enough for clients that repeatedly include tool metadata in model context.

### Model-authoring tools

| Tool | Important input | Structured result | Side effects |
| --- | --- | --- | --- |
| `create_model` | Optional model name/ID, generator, model actions, requirements, and properties | `draft_id`, graph model ID, `revision` | Creates an empty draft |
| `add_vertex` | `draft_id`, optional vertex ID, name, shared state, actions, requirements, properties, optional `expected_revision` | Created vertex and new `revision` | Adds one vertex |
| `add_edge` | `draft_id`, optional edge ID, optional source vertex ID, required target vertex ID, name, guard, actions, requirements, weight, dependency, properties, optional `expected_revision` | Created edge and new `revision` | Adds one edge |
| `update_model` | `draft_id`, patch of generator, start element, name, actions, requirements, properties, predefined path, optional `expected_revision` | Updated metadata and new `revision` | Updates draft metadata |
| `update_vertex` | `draft_id`, vertex ID, field patch, optional `expected_revision` | Updated vertex and new `revision` | Updates one vertex |
| `update_edge` | `draft_id`, edge ID, field patch, optional `expected_revision` | Updated edge and new `revision` | Updates one edge |
| `remove_element` | `draft_id`, element ID, `cascade` (default `false`), optional `expected_revision` | Removed IDs and new `revision` | Removes an edge or vertex |
| `export_model` | `draft_id` | Complete GraphWalker JSON object, `revision` | None |
| `discard_model` | `draft_id` | `discarded` | Releases draft state |

Model-authoring rules:

- `create_model` creates one graph inside a GraphWalker multimodel document. Additional graphs and shared-state composition can be added later with an `add_model` tool; the first implementation is intentionally single-graph authoring.
- `generator` may be set at creation or through `update_model`, but validation and execution report a missing generator clearly.
- `start_element_id` must refer to an existing vertex or edge. It may be set after the referenced element is added.
- `add_edge` rejects unknown vertex references. An omitted `source_vertex_id` creates a start edge, consistent with the JSON model format.
- Weight must be between `0.0` and `1.0`; dependency must be between `0` and `100`. Full semantic validation still runs through `graphwalker-model-checker`.
- Patch inputs distinguish an omitted field from an explicit `null`, allowing optional values such as guard, shared state, and source vertex to be cleared.
- Removing a vertex with `cascade: true` also removes its incident edges and returns all removed IDs. Removing an element referenced by `start_element_id` or `predefined_path_edge_ids` either requires an explicit cleanup option or fails with a precise reference error; it must never leave silent dangling references.
- `export_model` returns a JSON object rather than a JSON-encoded string. It can be saved verbatim, passed to `validate_model`, loaded through REST, or used with `start_execution`.
- Drafts are process-local and ephemeral. The MVP does not write files automatically; persistence is the client's responsibility.

### Validation, conversion, and execution tools

| Tool | Important input | Structured result | Side effects |
| --- | --- | --- | --- |
| `validate_model` | Exactly one of `model`: GraphWalker JSON object or `draft_id` | `valid`, ordered `issues`, draft `revision` when applicable | None |
| `convert_graphml` | `graphml`: XML string | `model`: JSON object | None |
| `start_execution` | Exactly one of inline `model` or `draft_id`, optional draft `revision`, `seed`, and `global_data` | `execution_id`, effective `seed`, source revision when applicable | Creates execution state from a snapshot |
| `next_step` | `execution_id` | `completed`, optional element, current data, visit count, and stop-condition fulfillment | Advances by at most one element |
| `execution_status` | `execution_id`, optional `include_elements` | `has_next`, current data, aggregate coverage, optionally per-element visit counts | None |
| `set_execution_data` | `execution_id`, `script` | updated data | Mutates execution variables |
| `restart_execution` | `execution_id` | effective seed and reset status | Resets using the original model, seed, and global data |
| `close_execution` | `execution_id` | `closed` | Releases execution state |

Detailed API rules:

- Accept a JSON value for `model`, not JSON encoded inside a string. Serialize it only at the existing `graphwalker-io` boundary.
- Require each model to contain its generator expression in the same way as the existing `-g`/`--gw` CLI input. Generator overrides can be added later if needed.
- When `validate_model` or `start_execution` receives a `draft_id`, reject a simultaneous inline `model`. If a revision is supplied, reject the call when it no longer matches the draft.
- `start_execution` must validate the snapshot first and refuse invalid drafts rather than creating a partly usable execution.
- Return both element ID and name, model ID, and element kind (`edge` or `vertex`) from `next_step`.
- Make `next_step` return `completed: true` without advancing when the stop condition is already fulfilled. This removes the need for a separate `has_next` round trip.
- Keep coverage numbers numeric and booleans boolean, even where the legacy REST API uses strings.
- Return an explicit error for an unknown or closed execution ID.
- Treat invalid models, invalid generator expressions, invalid data scripts, and unavailable transitions as tool execution errors with stable error codes and actionable messages. Reserve JSON-RPC protocol errors for malformed MCP requests.
- Preserve deterministic replay: restarting an execution must reuse its original seed. This differs from the current REST restart implementation, which constructs an unseeded machine, and should be corrected for every transport during the service extraction.

A bounded, stateless `generate_path` convenience tool may be added after the interactive API works. It must require a `max_steps` limit so `never`, unreachable coverage goals, or a problematic model cannot consume unbounded CPU or return an unbounded response.

## Crate and code layout

Add these workspace members:

```text
graphwalker-service/
  Cargo.toml
  src/
    lib.rs              # typed public API and errors
    draft.rs            # mutable model drafts and validation
    execution.rs        # one GraphWalker machine and restart snapshot
    registry.rs         # draft/execution IDs, isolation, limits, cleanup

graphwalker-mcp/
  Cargo.toml
  src/
    lib.rs              # MCP handler and tool implementations
    main.rs             # stdio startup and shutdown
    schema.rs           # tool input/output types when not local to handlers
  tests/
    stdio.rs            # end-to-end MCP protocol tests
```

Update:

- the root `Cargo.toml` workspace member list;
- `graphwalker-restful` to call `graphwalker-service` while preserving REST and WebSocket contracts;
- `graphwalker-cli` only if shared dependency wiring or help text changes are required;
- `README.md` with the new binary, a quick-start client configuration, and a link to user-facing MCP documentation;
- this document, or a separate final MCP reference page, with exact schemas and client examples once implementation settles.

Prefer the standalone `graphwalker-mcp` executable over adding `graphwalker mcp`: MCP hosts configure and launch a stable executable command, while the existing `graphwalker` CLI remains focused on human-facing commands. Distribution can package both binaries together.

## Safety and resource limits

The stdio MVP does not read arbitrary paths, make network requests, or execute operating-system commands. Models and GraphML are passed inline. This avoids ambiguous client workspace roots and prevents accidental file disclosure.

Before release, define and test conservative configurable defaults for:

- maximum model/GraphML input size;
- maximum vertices, edges, active drafts, and active executions;
- maximum script size and Rhai operation count/call depth in addition to the existing expression-depth limit;
- draft and execution idle lifetimes and cleanup on client disconnect;
- per-call timeout or step budget;
- maximum optional per-element status output.

Validate every input before mutating state. Do not include secrets, full unexpected payloads, or internal backtraces in tool output. MCP tool descriptions must accurately identify state-changing calls so clients can present appropriate confirmation UI.

The future HTTP transport must default to loopback unless explicitly configured otherwise and must not ship as an unauthenticated public endpoint.

## Implementation phases

### Phase 0: Compatibility spike

1. Record the workspace MSRV in Cargo metadata or a toolchain policy.
2. Select and pin a compatible `rmcp` version and minimal feature set.
3. Prove a tiny stdio server can initialize, list one tool, shut down cleanly, and keep stdout free of logs.
4. Confirm the SDK version against the current MCP conformance suite and document which protocol revisions it negotiates.

Exit criterion: the dependency and MSRV decision is explicit, and an MCP client can complete the protocol lifecycle locally.

Phase 0 completed on 2026-09-14:

- The workspace MSRV is Rust 1.88 and the full workspace test suite passes with Rust 1.88.0.
- `graphwalker-mcp` is a workspace binary using exactly `rmcp` 3.3.0 with the minimal stdio server feature set.
- The server exposes a temporary but useful structured `health` tool; GraphWalker domain tools remain assigned to later phases.
- Stdio integration tests cover both the `2025-11-25` initialize/initialized lifecycle and the current `2026-07-28` discovery lifecycle, including tool listing, tool invocation, structured output, EOF shutdown, and protocol-only stdout.
- `rmcp` 3.3.0 supports the stable `2026-07-28` protocol and compatibility with `2025-11-25` and earlier revisions. The official SDK repository runs the MCP conformance suites in its [conformance workflow](https://github.com/modelcontextprotocol/rust-sdk/actions/workflows/conformance.yml). The current conformance server harness targets URL-based servers, so GraphWalker's stdio-only adapter is verified by its local protocol integration tests; direct server-suite execution is deferred until Streamable HTTP exists.

### Phase 1: Extract the existing execution service

This phase is an internal refactor and contains no new REST features.

1. Introduce typed model, step, status, statistics, and error results.
2. Move existing transport-neutral behavior from `graphwalker-restful::actor` into `graphwalker-service`.
3. Add the execution registry with opaque IDs, limits, and per-execution serialization.
4. Preserve seed and global-data initialization across restart.
5. Adapt the existing REST and WebSocket handlers to the service without adding routes or changing documented payloads.

Exit criterion: all existing REST, WebSocket, CLI, and core tests pass unchanged, and service-level tests cover deterministic restart and isolated concurrent executions.

Phase 1 completed on 2026-09-14:

- `graphwalker-service` is a transport-neutral workspace crate with typed model, validation, conversion, step, status, statistics, element-status, restart, data, identifier, limit, and error results.
- `ExecutionRegistry` issues opaque random 128-bit execution IDs and defaults to at most 64 active executions. Callers can supply a different limit through `ExecutionLimits`.
- Each execution owns a worker thread because the GraphWalker machine intentionally contains thread-local state. A channel serializes calls to one execution, while separate workers let independent executions advance concurrently.
- Restart rebuilds the machine with its original model snapshot, effective seed, and global-data initialization. Data supplied later through `set_data` is intentionally transient.
- The existing REST and WebSocket actor now maps typed service results back to the established wire formats. No routes, commands, or documented payload shapes were added or changed.
- Service tests cover deterministic restart, generated-seed replay, preserved global data, same-execution serialization, isolated concurrent executions, registry limits and cleanup, stable typed errors, validation, GraphML conversion, statistics, model export, and element status. Adapter regression tests cover the legacy REST/WebSocket response conventions.
- The complete workspace test suite, including all existing CLI, REST, WebSocket, core, Studio, and MCP tests, passes with Rust 1.88.0.

### Phase 2: Implement and test graph construction in the service

This phase adds transport-neutral authoring behavior, not REST endpoints or MCP protocol wiring.

1. Add the draft registry with opaque IDs, revisions, limits, expiry, and per-draft serialization.
2. Implement `create_model`, `add_vertex`, `add_edge`, and `export_model` service operations first.
3. Add update, remove, validate, discard, and immutable execution-snapshot operations.
4. Make every mutation atomic and enforce structural references and resource limits.
5. Complete the dedicated graph-construction test matrix below before exposing the operations through a transport.

Exit criterion: the service can construct and export a model accepted by `graphwalker-io`, and all success, failure, atomicity, revision, and isolation tests pass.

### Phase 3: Implement the MCP adapter

1. Add `graphwalker-mcp` and advertise only the implemented capabilities.
2. Define schema-derived input/output structs for the model-authoring and execution tools.
3. Map service errors to MCP tool errors and structured content without REST response wrappers.
4. Run over stdio with graceful EOF/cancellation handling and stderr-only diagnostics.
5. Add MCP protocol tests for every graph-construction tool, plus the complete authoring-to-execution workflow.

Exit criterion: an MCP client can build and revise a model, export and validate it, start an execution from its snapshot, walk it to completion, update data, inspect coverage, restart deterministically, and close both the execution and draft.

### Phase 4: Add graph construction to REST separately

This phase is a separate implementation and should be delivered in a separate pull request after the service and MCP contracts are stable.

1. Design resource-oriented REST routes for draft creation, vertex/edge mutation, validation, export, and deletion.
2. Document the REST request/response schemas independently; do not expose MCP content envelopes or copy legacy `"result": "ok"` conventions without a compatibility decision.
3. Reuse only the typed `graphwalker-service` API.
4. Add REST-specific status-code, content-type, malformed-body, concurrency/revision, and lifecycle tests.
5. Update `doc/rest-api.md` or add a dedicated model-authoring REST reference.

Exit criterion: REST graph construction has an approved HTTP contract and passes its own integration suite without changing the MCP tools or existing execution endpoints.

### Phase 5: Documentation and release integration

1. Document building and launching `graphwalker-mcp`.
2. Add example configuration for at least one generic MCP client, using an absolute command path or installed binary name without client-specific secrets.
3. Document the draft-authoring workflow, tool inputs and outputs, limits, error behavior, and the fact that the server does not execute the system under test itself.
4. Add packaging/release artifacts for the new binary.
5. Test manually with [MCP Inspector](https://github.com/modelcontextprotocol/inspector).

Exit criterion: a new user can install the binary, configure a client, and complete the example workflow from the documentation. REST documentation may be released independently with Phase 4.

### Later phases

- Add a bounded one-shot `generate_path` tool if usage shows it reduces excessive round trips.
- Consider read-only resources for the JSON format, available generators, and stop conditions.
- Add Streamable HTTP only with a concrete remote-deployment and authorization design.
- Consider progress reporting or MCP tasks only for operations that become measurably long-running.

## Test plan

### Graph-construction test matrix

The four foundational authoring operations are release-critical. Test each first through the typed service API and then through the MCP protocol adapter. Protocol tests must assert both behavior and the advertised JSON input/output schemas.

| Operation | Required coverage |
| --- | --- |
| `create_model` | Minimal and fully populated inputs; generated and caller-supplied graph IDs; canonical defaults; metadata preservation; unique draft IDs; active-draft limit; invalid field types; no draft leaked after failure |
| `add_vertex` | Minimal and fully populated vertices; generated and supplied IDs; actions, requirements, shared state, and arbitrary properties; duplicate ID rejection across both vertices and edges; revision increment/conflict; draft isolation; unchanged draft after failure |
| `add_edge` | Normal, self-loop, and source-less start edges; generated and supplied IDs; all optional fields; unknown source/target rejection; missing target rejection; duplicate ID rejection; weight/dependency boundaries and out-of-range values; revision conflict; unchanged draft after failure |
| `export_model` | Empty, minimal valid, and fully populated drafts; JSON object rather than encoded string; stable ordering; exact GraphWalker field names; current revision; parse/export round trip through `graphwalker-io`; output accepted by validation, REST load, and `start_execution`; unknown/expired draft errors; no mutation during export |

Add an end-to-end golden fixture test that performs this exact sequence:

1. `create_model` with a generator.
2. Add two vertices.
3. Add a source-less start edge and a normal edge between the vertices.
4. Set the start element.
5. Export and compare the semantic JSON structure with a checked-in fixture.
6. Validate the exported model with no issues.
7. Start an execution from the draft and walk it to completion.

Avoid brittle assertions on generated UUID values: assert their format and referential consistency, or provide fixed IDs in golden tests. For generated IDs, verify uniqueness across repeated and concurrent calls. Every negative mutation test must compare the complete draft and revision before and after the call to prove atomicity.

### Unit and service tests

- An empty draft can be created, inspected, and discarded.
- Vertices and start/normal edges produce canonical GraphWalker JSON.
- Generated and caller-provided IDs are stable and unique.
- Edges with unknown endpoints and out-of-range weight/dependency values are rejected atomically.
- Update patches can set and clear optional fields.
- Deleting a connected vertex requires explicit cascading and reports every removed ID.
- Stale revisions are rejected without changing the draft.
- Exporting, parsing, and exporting a draft preserves its semantics.
- An execution created from a draft is unaffected by later edits to that draft.
- JSON validation returns ordered issues and never creates an execution.
- GraphML conversion returns a JSON object that can be loaded again.
- The same model and seed produce the same sequence.
- Restart reproduces the original sequence and reapplies global data.
- Different execution IDs have independent data and visit counts.
- Close releases state and later calls return `execution_not_found`.
- Malformed models, missing generators, invalid scripts, and limit violations produce stable errors.
- A fulfilled stop condition makes `next_step` idempotently return `completed: true`.
- All configured limits have boundary tests.

### Protocol integration tests

Spawn the compiled stdio binary with an MCP client test harness and verify:

- initialization/discovery and server identity;
- deterministic `tools/list` output and valid input/output schemas;
- successful and failing `tools/call` responses;
- structured content matches each declared output schema;
- no non-protocol bytes appear on stdout;
- EOF, cancellation, and process shutdown release execution state;
- two interleaved execution IDs cannot affect one another;
- two interleaved draft IDs cannot affect one another;
- mutating authoring calls return revisions and stale `expected_revision` values fail predictably.

Keep automated tests self-contained; MCP Inspector is a manual interoperability check, not a CI dependency.

### REST graph-construction tests

These tests belong only to Phase 4 and must not be mixed into the MCP test target. Repeat the graph-construction golden workflow over HTTP, then cover HTTP-specific behavior: route matching, methods, JSON content types, status codes, malformed bodies, request-size limits, missing/expired drafts, revision conflicts, and concurrent requests. Existing REST execution contract tests remain unchanged and run as regression coverage.

### Repository checks

Run the full workspace checks after each phase:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## MVP acceptance criteria

The MCP work is ready for an initial release when:

- `graphwalker-mcp` is a workspace binary installable with the project;
- a conforming local MCP client can discover and invoke all documented tools over stdio;
- a client can incrementally create, edit, validate, export, and discard a graph model;
- the service and MCP test matrices for `create_model`, `add_vertex`, `add_edge`, and `export_model` pass, including negative and atomicity cases;
- exported drafts conform to the existing GraphWalker JSON format and can be loaded by existing interfaces;
- validation, conversion, traversal from inline models or draft snapshots, data updates, statistics, deterministic restart, and cleanup work against existing fixtures;
- REST and WebSocket compatibility tests still pass;
- draft and execution IDs isolate concurrent work, and resource limits prevent unbounded work;
- stdout contains only MCP protocol messages;
- the supported Rust and MCP versions are documented;
- no filesystem, network, or remote-listening capability is enabled by default.

New REST graph-construction endpoints are not part of these MCP acceptance criteria. They are accepted and released independently under Phase 4; only preservation of the existing REST/WebSocket contracts gates the MCP release.

## Open questions to resolve during Phase 0

1. What default limits are appropriate for model size, active drafts/executions, idle lifetime, and step count?
2. Should `set_execution_data` remain available in the first release, given that it evaluates GraphWalker/Rhai expressions, or should the MVP initially expose traversal as read-only?
3. Which MCP clients and operating systems are release targets for the first interoperability matrix?
4. Should multi-model draft authoring and shared-state composition be part of the MVP, or follow the initial single-graph implementation?

## References

- [GraphWalker model-based testing overview](https://graphwalker.github.io/graphwalker-rs/model-based-testing)
- [Model Context Protocol specification](https://modelcontextprotocol.io/specification/)
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP tools specification and security considerations](https://modelcontextprotocol.io/specification/draft/server/tools)
- [MCP Inspector](https://github.com/modelcontextprotocol/inspector)
