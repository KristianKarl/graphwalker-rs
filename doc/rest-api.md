---
layout: default
title: REST API
nav_order: 8
---

# REST API

Start the REST server with:

```bash
graphwalker online -s RESTFUL -p 8080 -m model.json "random(edge_coverage(100))"
```

All endpoints are under the `/graphwalker` path prefix.

## Legacy execution response format

The execution endpoints documented in the next section preserve their existing wire format. Successful responses include `"result": "ok"`. Errors include `"result": "nok"` with an `"error"` message.

The draft-authoring API uses conventional HTTP status codes and typed JSON responses instead. It never adds a `result` wrapper.

---

## POST /graphwalker/load

Load a model and initialize the execution engine.

**Request body:** JSON model definition (the full model file content).

**Response:**

```json
{
  "result": "ok",
  "seed": 7298345612
}
```

The returned `seed` can be used later to reproduce the same traversal.

**Error example:**

```json
{
  "result": "nok",
  "error": "Model has no generator specified"
}
```

---

## GET /graphwalker/hasNext

Check whether more steps are available.

**Response:**

```json
{
  "result": "ok",
  "hasNext": "true"
}
```

Returns `"false"` when the stop condition is fulfilled.

**Error (no model loaded):**

```json
{
  "result": "nok",
  "error": "No model(s) are loaded."
}
```

---

## GET /graphwalker/getNext

Advance to the next element and return it.

**Response:**

```json
{
  "result": "ok",
  "currentElementName": "e_Login",
  "currentElementID": "e1",
  "modelId": "853429e2-0528-48b9-97b3-7725eafbb8b5"
}
```

| Field | Description |
|-------|-------------|
| `currentElementName` | Name of the current element (vertex or edge) |
| `currentElementID` | ID of the current element |
| `modelId` | ID of the model containing the element |

---

## GET /graphwalker/getData

Get the current execution data (all variables).

**Response:**

```json
{
  "result": "ok",
  "data": "loggedIn=true; itemCount=3"
}
```

The `data` field is a string representation of all variables in the execution context.

---

## PUT /graphwalker/setData/{script}

Execute a script to modify execution data. The script is passed as a URL path parameter.

**Example request:**

```
PUT /graphwalker/setData/loggedIn%20%3D%20true%3B
```

(URL-decoded: `loggedIn = true;`)

**Response:**

```json
{
  "result": "ok"
}
```

---

## PUT /graphwalker/restart

Reset execution to the initial state. The model remains loaded but all visit counts, variables, and state are cleared.

**Response:**

```json
{
  "result": "ok"
}
```

---

## GET /graphwalker/getStatistics

Get coverage statistics for the current execution.

**Response:**

```json
{
  "result": "ok",
  "totalNumberOfVertices": 5,
  "totalNumberOfEdges": 8,
  "totalNumberOfVisitedVertices": 3,
  "totalNumberOfVisitedEdges": 6,
  "totalNumberOfUnvisitedVertices": 2,
  "totalNumberOfUnvisitedEdges": 2,
  "vertexCoverage": 60,
  "edgeCoverage": 75
}
```

| Field | Description |
|-------|-------------|
| `totalNumberOfVertices` | Total vertices in all models |
| `totalNumberOfEdges` | Total edges in all models |
| `totalNumberOfVisitedVertices` | Vertices visited at least once |
| `totalNumberOfVisitedEdges` | Edges visited at least once |
| `totalNumberOfUnvisitedVertices` | Vertices not yet visited |
| `totalNumberOfUnvisitedEdges` | Edges not yet visited |
| `vertexCoverage` | Percentage of vertices visited (0&ndash;100) |
| `edgeCoverage` | Percentage of edges visited (0&ndash;100) |

---

## Model draft authoring

Model authoring is an independent, resource-oriented API under `/graphwalker/drafts`. Drafts are process-local and ephemeral: export a completed model before the server stops. Starting or restarting the REST server discards every draft.

The server can be started without preloading a model when only authoring is needed:

```bash
graphwalker online -s RESTFUL -p 8080
```

Every successful mutation returns the new numeric `revision`. Mutation bodies accept an optional `expected_revision`; if it does not equal the current revision, the server returns `409 Conflict` without changing the draft. Supplying revisions is recommended whenever multiple requests or clients could edit the same draft.

JSON request bodies are limited to 1 MiB and must use `Content-Type: application/json` or a compatible `application/*+json` media type. Exported models retain the canonical GraphWalker field names described in the [JSON format](json-format), while authoring request and response envelope fields use `snake_case`.

### Authoring routes

| Method | Route | Success | Purpose |
| --- | --- | --- | --- |
| `POST` | `/graphwalker/drafts` | `201 Created` | Create an empty single-model draft |
| `GET` | `/graphwalker/drafts/{draft_id}` | `200 OK` | Export canonical GraphWalker JSON and its revision |
| `PATCH` | `/graphwalker/drafts/{draft_id}` | `200 OK` | Patch model metadata |
| `DELETE` | `/graphwalker/drafts/{draft_id}` | `200 OK` | Discard the draft |
| `POST` | `/graphwalker/drafts/{draft_id}/vertices` | `201 Created` | Add a vertex |
| `PATCH` | `/graphwalker/drafts/{draft_id}/vertices/{vertex_id}` | `200 OK` | Patch a vertex |
| `POST` | `/graphwalker/drafts/{draft_id}/edges` | `201 Created` | Add an edge |
| `PATCH` | `/graphwalker/drafts/{draft_id}/edges/{edge_id}` | `200 OK` | Patch an edge |
| `DELETE` | `/graphwalker/drafts/{draft_id}/elements/{element_id}` | `200 OK` | Remove a vertex or edge |
| `GET` | `/graphwalker/drafts/{draft_id}/validation` | `200 OK` | Validate the current draft |

### Create a draft

`POST /graphwalker/drafts`

All fields are optional. Omitted list and object fields default to empty values.

```json
{
  "model_id": "checkout-model",
  "name": "Checkout",
  "generator": "random(edge_coverage(100))",
  "actions": ["global.started = true"],
  "requirements": ["REQ-1"],
  "properties": { "owner": "payments" }
}
```

Response (`201 Created`):

```json
{
  "draft_id": "draft_550e8400-e29b-41d4-a716-446655440000",
  "model_id": "checkout-model",
  "revision": 0
}
```

When `model_id` is omitted, GraphWalker generates it. The `draft_id` identifies mutable server state and is deliberately different from the GraphWalker model ID.

### Add vertices and edges

`POST /graphwalker/drafts/{draft_id}/vertices`

```json
{
  "id": "v_cart",
  "name": "v_Cart",
  "shared_state": null,
  "actions": [],
  "requirements": ["REQ-CART"],
  "properties": {},
  "expected_revision": 0
}
```

Response (`201 Created`):

```json
{
  "vertex": {
    "id": "v_cart",
    "name": "v_Cart",
    "shared_state": null,
    "actions": [],
    "requirements": ["REQ-CART"],
    "properties": {}
  },
  "revision": 1
}
```

`POST /graphwalker/drafts/{draft_id}/edges`

```json
{
  "id": "e_start",
  "name": "e_Start",
  "source_vertex_id": null,
  "target_vertex_id": "v_cart",
  "guard": null,
  "actions": [],
  "requirements": [],
  "properties": {},
  "weight": 1.0,
  "dependency": 0,
  "expected_revision": 1
}
```

`target_vertex_id` is required and must identify an existing vertex. Omit `source_vertex_id` or set it to `null` for a start edge. Element IDs must be unique across both vertices and edges. Weight must be between `0.0` and `1.0`; dependency must be between `0` and `100`.

### Patch a model, vertex, or edge

Use `PATCH` with only the fields that should change. An omitted field is retained. An explicit `null` clears an optional field or resets a list/object field to its empty value.

Set the start element:

```http
PATCH /graphwalker/drafts/{draft_id}
Content-Type: application/json
```

```json
{
  "start_element_id": "e_start",
  "expected_revision": 2
}
```

Model patch fields are `name`, `generator`, `start_element_id`, `actions`, `requirements`, `properties`, and `predefined_path_edge_ids`.

Vertex patch fields are `name`, `shared_state`, `actions`, `requirements`, and `properties`. Edge patch fields are `name`, `source_vertex_id`, `target_vertex_id`, `guard`, `actions`, `requirements`, `properties`, `weight`, and `dependency`. `target_vertex_id` cannot be cleared.

### Export and validate

`GET /graphwalker/drafts/{draft_id}` returns a JSON object rather than a JSON-encoded string:

```json
{
  "model": {
    "models": [
      {
        "id": "checkout-model",
        "generator": "random(edge_coverage(100))",
        "vertices": [],
        "edges": []
      }
    ]
  },
  "revision": 3
}
```

`GET /graphwalker/drafts/{draft_id}/validation` returns ordered validation messages:

```json
{
  "valid": true,
  "issues": [],
  "revision": 3
}
```

Validation does not mutate the draft. A missing or invalid generator is reported as an issue.

### Remove elements and discard drafts

`DELETE /graphwalker/drafts/{draft_id}/elements/{element_id}` accepts query parameters:

| Parameter | Default | Description |
| --- | --- | --- |
| `cascade` | `false` | Also remove edges connected to a deleted vertex |
| `cleanup_references` | `false` | Remove references from the start element and predefined path |
| `expected_revision` | omitted | Require the current draft revision |

Example response:

```json
{
  "removed_ids": ["e_checkout", "v_checkout"],
  "revision": 8
}
```

`DELETE /graphwalker/drafts/{draft_id}` releases the complete draft and returns:

```json
{ "discarded": true }
```

Later requests for that ID return `404 Not Found`.

### Authoring errors

Authoring failures always return a JSON body:

```json
{
  "code": "revision_conflict",
  "message": "Expected revision 3, but the draft is at revision 4"
}
```

| Status | Typical codes |
| --- | --- |
| `400 Bad Request` | `invalid_json`, `invalid_query` |
| `404 Not Found` | `draft_not_found` |
| `409 Conflict` | `revision_conflict`, `duplicate_element_id`, `referenced_element` |
| `410 Gone` | `draft_expired` |
| `413 Payload Too Large` | `payload_too_large` |
| `415 Unsupported Media Type` | `unsupported_media_type` |
| `422 Unprocessable Entity` | `invalid_model`, `unknown_vertex`, `missing_target_vertex`, `invalid_weight`, `invalid_dependency`, `invalid_element` |
| `429 Too Many Requests` | `draft_limit_reached`, `model_limit_reached` |
| `500 Internal Server Error` | `draft_unavailable`, `internal` |

Failed mutations are atomic: neither the model nor its revision changes.

---

## Typical usage pattern

```bash
# 1. Load a model (or start the server with -m flag to pre-load)
curl -X POST http://localhost:8080/graphwalker/load -d @model.json

# 2. Loop: check and step
while curl -s http://localhost:8080/graphwalker/hasNext | grep -q '"true"'; do
  STEP=$(curl -s http://localhost:8080/graphwalker/getNext)
  echo "$STEP"
  # Execute the test step indicated by currentElementName
done

# 3. Check final coverage
curl -s http://localhost:8080/graphwalker/getStatistics
```

---

## Seed and determinism

When starting with `--seed <value>`, the same seed produces identical traversal paths. The seed is also returned by the `/load` endpoint, so you can capture it and replay later:

```bash
# Capture the seed
SEED=$(curl -s -X POST http://localhost:8080/graphwalker/load -d @model.json | jq -r '.seed')
echo "Seed: $SEED"

# Replay with the same seed later
graphwalker online -s RESTFUL --seed $SEED -m model.json "random(edge_coverage(100))"
```
