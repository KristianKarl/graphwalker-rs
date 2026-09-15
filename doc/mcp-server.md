---
layout: default
title: MCP Server
nav_order: 11
---

# MCP Server

`graphwalker-mcp` is a local [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server. It lets an MCP client build GraphWalker models, validate and convert models, and drive deterministic model-based test executions.

The server selects the next model element; it does **not** operate the system under test. The client must perform each returned action or assertion in the target system, report any resulting data with `set_execution_data`, and then request the next step.

## How the pieces fit together

An MCP-based test run has three participants:

1. **The MCP client** starts `graphwalker-mcp`, calls its tools, and coordinates the run.
2. **The GraphWalker MCP server** stores model drafts and execution state and chooses the next edge or vertex.
3. **The test adapter** drives the system under test. This may be browser automation, an HTTP client, a mobile driver, or code supplied by the MCP client.

Edges normally describe actions, such as `e_SubmitLogin`. Vertices normally describe assertions, such as `v_DashboardDisplayed`. When `next_step` returns an edge, the client performs the action. When it returns a vertex, the client verifies the observable state. A failed action or assertion should stop the test run; calling `next_step` again would incorrectly record progress through behavior that did not occur.

The server uses stdio and has no HTTP listener of its own:

```text
MCP client  <-- stdio -->  graphwalker-mcp
     |
     +-- browser/API/test adapter --> system under test
```

## Install

GraphWalker requires Rust 1.88 or later.

To install the MCP binary from a repository checkout:

```bash
cargo install --locked --path graphwalker-mcp
```

This puts `graphwalker-mcp` in Cargo's binary directory, normally `$HOME/.cargo/bin`. Alternatively, build it without installing:

```bash
cargo build --release --locked -p graphwalker-mcp
```

The binary is then at `target/release/graphwalker-mcp` (`graphwalker-mcp.exe` on Windows). Tagged releases also provide archives for Linux x86-64, macOS Apple silicon, and Windows x86-64. Each archive has a matching `.sha256` checksum file.

## Configure an MCP client

The server uses stdio. An MCP client starts it as a child process; it is not an interactive terminal program and does not listen on a network port.

Use the installed binary if Cargo's binary directory is on the client's `PATH`:

```json
{
  "mcpServers": {
    "graphwalker": {
      "command": "graphwalker-mcp",
      "args": []
    }
  }
}
```

If the client does not inherit that `PATH`, use the absolute path to the installed or built binary:

```json
{
  "mcpServers": {
    "graphwalker": {
      "command": "/absolute/path/to/graphwalker-mcp",
      "args": []
    }
  }
}
```

On Windows, use a JSON-escaped absolute path such as `C:\\Tools\\GraphWalker\\graphwalker-mcp.exe`. No environment variables, API keys, or other secrets are required.

The exact location of MCP configuration depends on the client. Use the client's command-based or stdio server configuration and restart or reconnect the client after changing it. Do not configure an HTTP URL or port for `graphwalker-mcp`.

## Quick start with MCP Inspector

[MCP Inspector](https://github.com/modelcontextprotocol/inspector) is a convenient way to verify the installation before configuring another client. It requires Node.js 22.19 or later.

From the repository root:

```bash
cargo build --release --locked -p graphwalker-mcp
npx @modelcontextprotocol/inspector "$(pwd)/target/release/graphwalker-mcp"
```

Open the one-time URL printed by Inspector, then:

1. Select **Tools** and list the available tools.
2. Call `health` with an empty input object. It should return `status: "ok"`.
3. Follow the model-authoring workflow below, copying returned IDs and revisions exactly.

For a non-interactive discovery check:

```bash
npx @modelcontextprotocol/inspector --cli \
  "$(pwd)/target/release/graphwalker-mcp" \
  --method tools/list
```

The Inspector is only an MCP client for invoking tools. It does not perform actions in the system under test.

## Build and execute a model

The JSON snippets in this section are **tool arguments**, not commands to paste into a shell. Select the named tool in your MCP client and supply the corresponding object.

The following tool-call sequence creates a two-state login model. Values such as `draft_id` and generated IDs in responses are examples; use the values the server returns.

1. Create a draft:

   ```json
   {"name":"Login","generator":"random(edge_coverage(100))"}
   ```

   `create_model` returns `draft_id`, `model_id`, and revision `0`.

2. Add the logged-out and logged-in vertices, carrying the latest revision into each mutation:

   ```json
   {"draft_id":"<draft_id>","id":"v_logged_out","name":"v_LoggedOut","expected_revision":0}
   ```

   ```json
   {"draft_id":"<draft_id>","id":"v_logged_in","name":"v_LoggedIn","expected_revision":1}
   ```

3. Add a start edge and the login and logout transitions:

   ```json
   {"draft_id":"<draft_id>","id":"e_start","name":"e_Start","target_vertex_id":"v_logged_out","expected_revision":2}
   ```

   ```json
   {"draft_id":"<draft_id>","id":"e_login","name":"e_Login","source_vertex_id":"v_logged_out","target_vertex_id":"v_logged_in","expected_revision":3}
   ```

   ```json
   {"draft_id":"<draft_id>","id":"e_logout","name":"e_Logout","source_vertex_id":"v_logged_in","target_vertex_id":"v_logged_out","expected_revision":4}
   ```

   The return transition prevents the logged-in vertex from becoming a cul-de-sac when using random generation with full edge coverage.

4. Select the start edge and validate the draft:

   ```json
   {"draft_id":"<draft_id>","start_element_id":"e_start","expected_revision":5}
   ```

   ```json
   {"draft_id":"<draft_id>"}
   ```

   Call `update_model` for the first input and `validate_model` for the second. Validation returns `valid: true` and the current revision when the model is executable.

5. Call `export_model` with `{"draft_id":"<draft_id>"}` to obtain canonical GraphWalker JSON. Save this result outside the server if the model must persist.

6. Start from an immutable snapshot of revision 6:

   ```json
   {"draft_id":"<draft_id>","revision":6,"seed":1234}
   ```

   `start_execution` returns an `execution_id`, the effective seed, and `source_revision`. Supplying the seed makes a random traversal reproducible.

7. Repeatedly call `next_step` with `{"execution_id":"<execution_id>"}`. For every returned `element`, execute its named transition or verify its named state in the system under test. Stop when `completed` is `true`.

   If a system action produces data used by later guards, update the execution context explicitly:

   ```json
   {"execution_id":"<execution_id>","script":"authenticated = true"}
   ```

8. Inspect coverage with `execution_status`, optionally restart with `restart_execution`, then release process-local state:

   ```json
   {"execution_id":"<execution_id>","include_elements":true}
   ```

   Call `close_execution` with the execution ID and `discard_model` with the draft ID when finished.

## Execute an existing JSON model

If a canonical model already exists, an execution can be started without creating a draft. Read the JSON file in the client, pass the complete JSON object as `model`, and optionally provide a seed:

```json
{
  "model": {
    "models": [
      {
        "id": "login-model",
        "name": "Login",
        "generator": "random(edge_coverage(100))",
        "startElementId": "e_start",
        "vertices": [
          { "id": "v_logged_out", "name": "v_LoggedOut" },
          { "id": "v_logged_in", "name": "v_LoggedIn" }
        ],
        "edges": [
          {
            "id": "e_start",
            "name": "e_Start",
            "targetVertexId": "v_logged_out"
          },
          {
            "id": "e_login",
            "name": "e_Login",
            "sourceVertexId": "v_logged_out",
            "targetVertexId": "v_logged_in"
          },
          {
            "id": "e_logout",
            "name": "e_Logout",
            "sourceVertexId": "v_logged_in",
            "targetVertexId": "v_logged_out"
          }
        ]
      }
    ]
  },
  "seed": 1234
}
```

The example is a complete minimal model. Replace it with the canonical contents of your model file. Call `validate_model` with the same model object first when the source is not already known to be valid.

Use one of these two inputs with `start_execution`, never both:

| Source | Input | When to use it |
|--------|-------|----------------|
| Draft snapshot | `draft_id` and exact `revision` | The model was authored in the current server process |
| Inline model | Complete canonical JSON in `model` | The model came from a file or the server was restarted |

## Drive the system under test

An execution is deliberately stepwise. A client-side runner should use this control flow:

```text
started = start_execution(model or draft, seed)

while true:
    step = next_step(started.execution_id)
    if step.completed:
        break

    if step.element.kind == "edge":
        perform the named action in the system under test
    else:
        assert the named observable state in the system under test

    if the action or assertion failed:
        stop and report the mismatch

status = execution_status(started.execution_id, include_elements = true)
close_execution(started.execution_id)
```

GraphWalker records an element as visited when `next_step` returns it. For that reason, do not pre-fetch several steps before operating the system under test. Request one element, perform or verify it, record the result in the client, and only then request the next element.

Use stable, descriptive element names so the adapter can map them to implementation functions:

```text
e_OpenLoginPage       -> open_login_page()
v_LoginPageDisplayed -> assert_login_page_displayed()
e_SubmitCredentials  -> submit_credentials()
v_DashboardDisplayed -> assert_dashboard_displayed()
```

Properties attached to vertices and edges can carry adapter-specific details such as URLs, selectors, field names, API paths, and expected text. GraphWalker preserves these properties in the canonical model, while the client decides how to interpret them.

### Share observed data with guards

Use `set_execution_data` when the system under test produces a value that later guards or actions need. For example, after the adapter observes that authentication succeeded:

```json
{
  "execution_id": "<execution_id>",
  "script": "authenticated = true"
}
```

The script updates GraphWalker's execution data; it does not modify the system under test. Keep secrets out of model properties, data scripts, logs, and exported models.

### Reproduce a traversal

Pass a fixed `seed` to `start_execution` when using a random generator. The same model, seed, and initial data produce the same GraphWalker traversal. External system state can still change the test outcome, so record the model revision or exported model alongside the seed.

Call `restart_execution` to reset an existing execution to its original model snapshot, seed, and global data. Restarting does not reset the browser, database, remote service, or any other external state; the client must reset those separately.

## Draft and execution lifecycle

Draft and execution identifiers are local to one `graphwalker-mcp` process:

- A draft is mutable authoring state. Every successful mutation increments its revision.
- An execution is an isolated snapshot. Editing its source draft does not change a running execution.
- Disconnecting the client or stopping the server loses all drafts and executions.
- `export_model` is the persistence boundary. Save its canonical JSON result before disconnecting.
- `close_execution` releases an execution; `discard_model` releases a draft.

A safe cleanup order is:

1. Call `execution_status` and store the final coverage/result information.
2. Call `close_execution`.
3. Confirm that exported JSON exists outside the server if the model must persist.
4. Call `discard_model`.

## Tool reference

All successful calls return structured JSON. Tool input schemas are also advertised through MCP discovery, so compatible clients can construct forms or validate calls automatically.

| Tool | Required input | Result and behavior |
|------|----------------|---------------------|
| `health` | None | Returns server status and version; read-only. |
| `create_model` | None | Creates a draft and returns `draft_id`, `model_id`, and `revision`. Optional model metadata includes `model_id`, `name`, `generator`, `actions`, `requirements`, and `properties`. |
| `add_vertex` | `draft_id` | Adds a vertex and returns it with the new revision. Optional fields include `id`, `name`, `shared_state`, metadata, and `expected_revision`. |
| `add_edge` | `draft_id`, `target_vertex_id` | Adds an edge and returns it with the new revision. `source_vertex_id` is omitted for a start edge; other optional fields include `id`, `name`, `guard`, metadata, `weight`, `dependency`, and `expected_revision`. |
| `update_model` | `draft_id` | Patches model metadata, start element, or predefined path and returns the canonical model with its new revision. |
| `update_vertex` | `draft_id`, `vertex_id` | Patches a vertex and returns it with the new revision. |
| `update_edge` | `draft_id`, `edge_id` | Patches an edge and returns it with the new revision. |
| `remove_element` | `draft_id`, `element_id` | Removes an element and returns all removed IDs plus the new revision. Connected edges require `cascade: true`; metadata references require `cleanup_references: true`. |
| `export_model` | `draft_id` | Returns canonical GraphWalker JSON and the current revision without changing the draft. |
| `discard_model` | `draft_id` | Permanently releases the draft and returns `discarded: true`. |
| `validate_model` | Exactly one of `model`, `draft_id` | Returns `valid`, an issue-message array, and the revision for a draft. Read-only. |
| `convert_graphml` | `graphml` | Converts an inline GraphML/yEd document to canonical GraphWalker JSON without storing it. |
| `start_execution` | Exactly one of `model`, `draft_id` | Creates isolated execution state and returns `execution_id`, effective `seed`, and the draft `source_revision` when applicable. `revision` is valid only with a draft. |
| `next_step` | `execution_id` | Advances by at most one element; returns `completed` and an optional element with IDs, kind, execution data, visit counts, and stop-condition fulfillment. |
| `execution_status` | `execution_id` | Returns current data and aggregate coverage without advancing. Set `include_elements: true` for per-element visit counts. |
| `set_execution_data` | `execution_id`, `script` | Evaluates a GraphWalker/Rhai data script and returns the updated execution data. |
| `restart_execution` | `execution_id` | Restarts with the original model, seed, and global data and returns the seed. |
| `close_execution` | `execution_id` | Permanently releases the execution and returns `closed: true`. |

For update tools, an omitted patch field means “leave unchanged”; an explicit JSON `null` means “clear this field.” Empty arrays and objects replace existing collections with empty collections. Every successful draft mutation increments its revision. Use `expected_revision` to prevent one client from silently overwriting another mutation.

Starting an execution from a draft copies a snapshot. Later changes to that draft do not affect the execution.

## State, limits, and errors

Drafts and executions live only in the `graphwalker-mcp` process. They disappear when the client disconnects or the server exits. Export models that need to survive the session and explicitly close/discard state that is no longer needed.

The current defaults are:

- 64 active drafts per server process;
- 10,000 vertices and 20,000 edges per draft;
- 30 minutes of draft inactivity before expiry;
- 64 active executions per server process.

These are fixed defaults in the current binary and are not command-line configuration options. The server does not currently impose separate documented byte limits on inline model, GraphML, or data-script inputs; clients should keep calls bounded. `next_step` advances at most one element, so the client controls traversal work one call at a time.

Tool failures set MCP's tool-error indicator and return structured `{ "code", "message" }` content. Service codes include `invalid_model`, `invalid_generator`, `invalid_data`, draft/execution limit and lookup errors, `revision_conflict`, invalid or duplicate element errors, reference errors, and `internal`. Adapter-level input conflicts use `invalid_input`. A failed draft mutation is atomic: it does not change the model or revision. Malformed MCP requests and schema/type errors are protocol errors handled by the MCP SDK.

The server reads and writes only the stdio protocol stream. Its tools do not read files, make network requests, launch commands, or listen remotely. Treat `set_execution_data` scripts as active model input: they can change execution variables and influence guards and traversal.

## Troubleshooting

### The server appears to hang when started in a terminal

This is expected. The process is waiting for newline-delimited MCP messages on stdin. Configure it in an MCP client or use MCP Inspector instead of treating it as an interactive CLI.

### The client cannot find `graphwalker-mcp`

Use an absolute command path in the client configuration. If you built from source, confirm that `target/release/graphwalker-mcp` exists. If you used `cargo install`, check Cargo's binary directory and ensure the MCP client inherits the expected `PATH`.

### A draft or execution ID is not found

IDs do not survive a server restart or client disconnect. Start a new execution from exported canonical JSON, or create/import a new draft.

### A mutation reports `revision_conflict`

Another successful mutation changed the draft. Use the newest returned revision and reapply the intended change. Do not guess or increment revisions locally.

### `start_execution` rejects the model

Call `validate_model` and correct every reported issue. Also verify that exactly one of `model` or `draft_id` was supplied and that `revision` is only used with `draft_id`.

### Coverage does not reach the stop condition

Inspect `execution_status` with `include_elements: true`. Unreachable edges, guards that never become true, or a cul-de-sac can prevent full coverage. Correct the model rather than repeatedly advancing an execution that cannot satisfy its generator.

On Windows, pass an absolute path ending in `target\\release\\graphwalker-mcp.exe` when configuring a client or Inspector.

## Protocol support

The server advertises tools over stdio using the official Rust MCP SDK. Automated integration tests cover MCP revisions `2025-11-25` and `2026-07-28`, tool discovery, schemas, successful workflows, structured failures, and stdout framing.

See the [MCP implementation plan](mcp) for architecture decisions, phase history, and future work.
