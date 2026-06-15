# CLI Test Data

This directory contains test data files for the CLI tool, used to verify the multi-protocol flow executor functionality.

## Schema Reference

### Node Types

| Type | Description | data Config |
|------|-------------|-------------|
| `entry` | Entry node | Empty object `{}` |
| `http` | HTTP request node | url, method, headers, body |
| `conditional` | Conditional branch node | condition_type, field_path, operator, expected_value / advanced_expression |

### Flow Structure

Flow files use the **FlowFile wrapping format**:

```json
{
  "_version": 1,
  "flow": {
    "uuid": "flow-identifier",
    "name": "Flow Name",
    "initial_variables": { ... },
    "nodes": [...],
    "connections": [...]
  },
  "_meta": null
}
```

### Node Structure

```json
{
  "uuid": "node-001",
  "type": "entry|http|conditional",
  "name": "Node Name",
  "position": { "x": 0.0, "y": 0.0 },
  "data": { ... },
  "ports": null
}
```

### Connection Structure

```json
{
  "id": "conn-001",
  "source_node_uuid": "node-001",
  "target_node_uuid": "node-002",
  "source_port_id": "output|true|false",
  "target_port_id": "input"
}
```

### Condition Operators

| Operator | Description | Applicable Types |
|----------|-------------|------------------|
| `equal` | Equal to | All types |
| `not_equal` | Not equal to | All types |
| `greater` | Greater than | Numbers |
| `greater_or_equal` | Greater than or equal to | Numbers |
| `less` | Less than | Numbers |
| `less_or_equal` | Less than or equal to | Numbers |
| `contains` | Contains | Strings, Arrays |
| `starts_with` | Starts with | Strings |
| `ends_with` | Ends with | Strings |
| `exists` | Exists | All types |

### Variable References

- `{{xxx}}` - Reference flow initial variables (flattened to top-level keys)
- `{{http_response.data.xxx}}` - Reference HTTP response data
- `{{http_response.status}}` - Reference HTTP status code

## Normal Scenario Test Files

| File | Description | Coverage |
|------|-------------|----------|
| `01_simple_entry_http.json` | Simple flow | Entry → HTTP GET basic path |
| `02_http_all_methods.json` | HTTP methods | GET/POST/PUT/DELETE/PATCH |
| `03_http_with_headers_body.json` | HTTP config | Headers, nested JSON body |
| `04_conditional_simple.json` | Simple conditional | Entry → HTTP → Conditional |
| `05_conditional_all_operators.json` | All operators | 10 condition operators |
| `06_conditional_advanced.json` | Advanced conditional | advanced_expression |
| `07_variable_reference.json` | Variable reference | initial_variables in URL/Headers |
| `08_multi_http_chain.json` | HTTP chain | 3 HTTP requests chained |
| `09_full_workflow.json` | Full workflow | Entry → HTTP → Conditional → HTTP (branches) |

## Edge Case Test Files

| File | Description | Expected Behavior |
|------|-------------|-------------------|
| `01_empty_flow.json` | Empty flow | No nodes executed |
| `02_no_entry_node.json` | No Entry node | Validator error |
| `03_missing_required_fields.json` | Missing required fields | Deserialization failure |
| `04_invalid_json_syntax.json` | Invalid JSON syntax | JSON parse failure |
| `05_invalid_node_type.json` | Invalid node type | Deserialization failure |
| `06_orphan_node.json` | Orphan node | Node not executed |
| `07_http_missing_url.json` | HTTP missing URL | Execution failure |
| `08_http_invalid_method.json` | Invalid HTTP method | Deserialization failure |
| `09_http_empty_body_for_post.json` | POST without body | Send empty body |
| `10_conditional_missing_operator.json` | Conditional missing operator | Uses default value |
| `11_conditional_empty_expression.json` | Empty condition expression | Evaluation failure |
| `12_connection_invalid_reference.json` | Invalid connection reference | Node not found |

## Usage

### Build CLI Tool

```bash
cd src-tauri
cargo build --release
```

### Execute Test Files

```bash
./target/release/mpe run -f ../tests/cli/01_simple_entry_http.json
```

### Validate JSON Syntax

```bash
for f in tests/cli/*.json tests/cli/edge_cases/*.json; do
  jq . "$f" > /dev/null && echo "PASS $f" || echo "FAIL $f"
done
```

## Notes

1. **Schema Version**: Uses `entry` type, not the legacy `start`/`end`
2. **Network Dependency**: Some tests use httpbin.org
3. **Edge Cases**: Files in edge_cases/ are designed to trigger errors
