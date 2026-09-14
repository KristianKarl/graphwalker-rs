---
layout: default
title: MCP Server
nav_order: 11
---

# MCP Server

`graphwalker-mcp` is a local [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server. It lets an MCP client build GraphWalker models, validate and convert models, and drive deterministic model-based test executions.

The server selects the next model element; it does **not** operate the system under test. The client must perform each returned action or assertion in the target system, report any resulting data with `set_execution_data`, and then request the next step.

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

## Build and execute a model

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

3. Add a start edge and the login transition:

   ```json
   {"draft_id":"<draft_id>","id":"e_start","name":"e_Start","target_vertex_id":"v_logged_out","expected_revision":2}
   ```

   ```json
   {"draft_id":"<draft_id>","id":"e_login","name":"e_Login","source_vertex_id":"v_logged_out","target_vertex_id":"v_logged_in","expected_revision":3}
   ```

4. Select the start edge and validate the draft:

   ```json
   {"draft_id":"<draft_id>","start_element_id":"e_start","expected_revision":4}
   ```

   ```json
   {"draft_id":"<draft_id>"}
   ```

   Call `update_model` for the first input and `validate_model` for the second. Validation returns `valid: true` and the current revision when the model is executable.

5. Call `export_model` with `{"draft_id":"<draft_id>"}` to obtain canonical GraphWalker JSON. Save this result outside the server if the model must persist.

6. Start from an immutable snapshot of revision 5:

   ```json
   {"draft_id":"<draft_id>","revision":5,"seed":1234}
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

## Test with MCP Inspector

[MCP Inspector](https://github.com/modelcontextprotocol/inspector) requires Node.js 22.19 or later. Build the server first, then launch the Inspector web UI against its absolute path:

```bash
cargo build --release --locked -p graphwalker-mcp
npx @modelcontextprotocol/inspector "$(pwd)/target/release/graphwalker-mcp"
```

Open the one-time URL printed by Inspector, select **Tools**, list the advertised tools, and run the workflow above. For a scriptable discovery check:

```bash
npx @modelcontextprotocol/inspector --cli "$(pwd)/target/release/graphwalker-mcp" --method tools/list
```

On Windows, pass the absolute path to `target\\release\\graphwalker-mcp.exe` instead. Inspector is a manual interoperability tool and is not required by the GraphWalker build or automated test suite.

## Protocol support

The server advertises tools over stdio using the official Rust MCP SDK. Automated integration tests cover MCP revisions `2025-11-25` and `2026-07-28`, tool discovery, schemas, successful workflows, structured failures, and stdout framing.

See the [MCP implementation plan](mcp) for architecture decisions, phase history, and future work.
