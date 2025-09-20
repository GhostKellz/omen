# GhostFlow API Reference

Complete REST API documentation for GhostFlow.

## Base URL

```
http://localhost:3000/api
```

## Authentication

Currently, GhostFlow operates without authentication. Future versions will support:
- JWT Bearer tokens
- API keys
- OAuth2

## Rate Limiting

*Not yet implemented*

## Error Responses

All error responses follow this format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Flow validation failed",
    "details": {
      "field": "nodes",
      "reason": "At least one node is required"
    }
  },
  "timestamp": "2024-01-08T12:00:00Z"
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `VALIDATION_ERROR` | Request validation failed |
| `NOT_FOUND` | Resource not found |
| `INTERNAL_ERROR` | Server error |
| `EXECUTION_ERROR` | Flow execution failed |
| `TIMEOUT_ERROR` | Request timeout |

---

## Flows

### Create Flow

**POST** `/flows`

Create a new flow definition.

**Request Body:**
```json
{
  "name": "My Flow",
  "description": "Optional flow description",
  "nodes": {
    "start_node": {
      "id": "start_node",
      "node_type": "http_request",
      "name": "Fetch Data",
      "description": "Fetch data from API",
      "parameters": {
        "url": "https://api.example.com/data",
        "method": "GET",
        "headers": {
          "Accept": "application/json"
        }
      },
      "position": {
        "x": 100,
        "y": 100
      },
      "timeout_ms": 30000
    }
  },
  "edges": [],
  "triggers": [
    {
      "id": "manual_trigger",
      "trigger_type": {
        "type": "manual"
      },
      "config": {},
      "enabled": true
    }
  ],
  "parameters": {},
  "secrets": []
}
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "My Flow",
  "description": "Optional flow description",
  "version": "1.0.0",
  "created_at": "2024-01-08T12:00:00Z",
  "updated_at": "2024-01-08T12:00:00Z",
  "created_by": "system"
}
```

### Get Flow

**GET** `/flows/{id}`

Retrieve a specific flow by ID.

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "My Flow",
  "description": "Optional flow description",
  "version": "1.0.0",
  "nodes": { ... },
  "edges": [ ... ],
  "triggers": [ ... ],
  "parameters": { ... },
  "secrets": [ ... ],
  "metadata": {
    "created_at": "2024-01-08T12:00:00Z",
    "updated_at": "2024-01-08T12:00:00Z",
    "created_by": "system",
    "tags": ["example"],
    "category": "automation"
  }
}
```

### List Flows

**GET** `/flows`

List all flows with optional filtering.

**Query Parameters:**
- `limit` (optional): Maximum number of flows to return (default: 50)
- `offset` (optional): Number of flows to skip (default: 0)
- `category` (optional): Filter by category
- `tag` (optional): Filter by tag
- `search` (optional): Search in name and description

**Example:**
```
GET /flows?limit=10&category=automation&search=api
```

**Response:**
```json
{
  "flows": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "My Flow",
      "description": "Optional flow description",
      "version": "1.0.0",
      "created_at": "2024-01-08T12:00:00Z",
      "updated_at": "2024-01-08T12:00:00Z",
      "created_by": "system",
      "enabled": true,
      "tags": ["example"],
      "category": "automation"
    }
  ],
  "total": 1,
  "limit": 10,
  "offset": 0
}
```

### Update Flow

**PUT** `/flows/{id}`

Update an existing flow.

**Request Body:** Same as Create Flow

**Response:** Same as Get Flow

### Delete Flow

**DELETE** `/flows/{id}`

Delete a flow and all its executions.

**Response:**
```json
{
  "message": "Flow deleted successfully"
}
```

### Validate Flow

**POST** `/flows/{id}/validate`

Validate a flow definition without executing it.

**Response:**
```json
{
  "valid": true,
  "errors": [],
  "warnings": [
    "Node 'transform_data' has no incoming connections"
  ]
}
```

### Execute Flow

**POST** `/flows/{id}/execute`

Execute a flow with optional input data.

**Request Body:**
```json
{
  "input": {
    "message": "Hello World",
    "config": {
      "debug": true
    }
  },
  "dry_run": false
}
```

**Response:**
```json
{
  "execution_id": "660e8400-e29b-41d4-a716-446655440000",
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "started_at": "2024-01-08T12:00:00Z"
}
```

---

## Executions

### List Executions

**GET** `/executions`

List flow executions with filtering.

**Query Parameters:**
- `flow_id` (optional): Filter by flow ID
- `status` (optional): Filter by status
- `limit` (optional): Maximum number of executions (default: 50)
- `offset` (optional): Number to skip (default: 0)

**Example:**
```
GET /executions?flow_id=550e8400-e29b-41d4-a716-446655440000&status=completed
```

**Response:**
```json
{
  "executions": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440000",
      "flow_id": "550e8400-e29b-41d4-a716-446655440000",
      "flow_version": "1.0.0",
      "status": "completed",
      "started_at": "2024-01-08T12:00:00Z",
      "completed_at": "2024-01-08T12:00:30Z",
      "execution_time_ms": 30000,
      "trigger": {
        "trigger_type": "manual",
        "source": "api",
        "metadata": {}
      }
    }
  ],
  "total": 1
}
```

### Get Execution

**GET** `/executions/{id}`

Get detailed execution information.

**Response:**
```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "flow_version": "1.0.0",
  "status": "completed",
  "trigger": {
    "trigger_type": "manual",
    "source": "api",
    "metadata": {}
  },
  "input_data": {
    "message": "Hello World"
  },
  "output_data": {
    "result": "Success",
    "processed_count": 5
  },
  "error": null,
  "node_executions": {
    "start_node": {
      "node_id": "start_node",
      "status": "completed",
      "input_data": { ... },
      "output_data": { ... },
      "started_at": "2024-01-08T12:00:00Z",
      "completed_at": "2024-01-08T12:00:05Z",
      "execution_time_ms": 5000,
      "retry_count": 0,
      "logs": [
        {
          "timestamp": "2024-01-08T12:00:00Z",
          "level": "INFO",
          "message": "Starting HTTP request to https://api.example.com/data",
          "details": {}
        }
      ]
    }
  },
  "started_at": "2024-01-08T12:00:00Z",
  "completed_at": "2024-01-08T12:00:30Z",
  "execution_time_ms": 30000,
  "metadata": {
    "executor_id": "default",
    "environment": "production",
    "correlation_id": "req_123",
    "trace_id": "trace_456",
    "span_id": "span_789"
  }
}
```

### Cancel Execution

**POST** `/executions/{id}/cancel`

Cancel a running execution.

**Response:**
```json
{
  "message": "Execution cancelled successfully",
  "status": "cancelled"
}
```

---

## Nodes

### List Available Nodes

**GET** `/nodes`

Get the catalog of available node types.

**Response:**
```json
{
  "nodes": [
    {
      "id": "http_request",
      "name": "HTTP Request",
      "description": "Make HTTP requests to external APIs",
      "category": "action",
      "version": "1.0.0",
      "inputs": [
        {
          "name": "trigger",
          "display_name": "Trigger",
          "description": "Trigger the HTTP request",
          "data_type": "any",
          "required": false
        }
      ],
      "outputs": [
        {
          "name": "response",
          "display_name": "Response",
          "description": "HTTP response data",
          "data_type": "object",
          "required": true
        }
      ],
      "parameters": [
        {
          "name": "url",
          "display_name": "URL",
          "description": "URL to make the request to",
          "param_type": "string",
          "required": true,
          "validation": {
            "pattern": "^https?://.*"
          }
        },
        {
          "name": "method",
          "display_name": "HTTP Method",
          "param_type": "select",
          "default_value": "GET",
          "required": true,
          "options": [
            {"value": "GET", "label": "GET"},
            {"value": "POST", "label": "POST"}
          ]
        }
      ],
      "icon": "globe",
      "color": "#2563eb"
    }
  ]
}
```

### Get Node Definition

**GET** `/nodes/{id}`

Get detailed information about a specific node type.

**Response:** Same structure as individual node in list response.

---

## WebSocket API

### Connection

Connect to real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');
```

### Message Types

#### Execution Started
```json
{
  "type": "execution_started",
  "data": {
    "execution_id": "660e8400-e29b-41d4-a716-446655440000",
    "flow_id": "550e8400-e29b-41d4-a716-446655440000",
    "started_at": "2024-01-08T12:00:00Z"
  }
}
```

#### Node Execution Started
```json
{
  "type": "node_started",
  "data": {
    "execution_id": "660e8400-e29b-41d4-a716-446655440000",
    "node_id": "start_node",
    "started_at": "2024-01-08T12:00:00Z"
  }
}
```

#### Node Execution Completed
```json
{
  "type": "node_completed",
  "data": {
    "execution_id": "660e8400-e29b-41d4-a716-446655440000",
    "node_id": "start_node",
    "status": "completed",
    "output_data": { ... },
    "execution_time_ms": 5000
  }
}
```

#### Execution Completed
```json
{
  "type": "execution_completed",
  "data": {
    "execution_id": "660e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "output_data": { ... },
    "execution_time_ms": 30000
  }
}
```

#### Execution Failed
```json
{
  "type": "execution_failed",
  "data": {
    "execution_id": "660e8400-e29b-41d4-a716-446655440000",
    "error": {
      "error_type": "network_error",
      "message": "Connection timeout",
      "node_id": "start_node"
    }
  }
}
```

### Client Example

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  console.log('Connected to GhostFlow WebSocket');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch(message.type) {
    case 'execution_started':
      console.log('Execution started:', message.data.execution_id);
      break;
    case 'node_completed':
      console.log('Node completed:', message.data.node_id);
      break;
    case 'execution_completed':
      console.log('Execution completed successfully');
      break;
    case 'execution_failed':
      console.error('Execution failed:', message.data.error);
      break;
  }
};

ws.onclose = () => {
  console.log('WebSocket connection closed');
};
```

---

## Response Codes

| Code | Status | Description |
|------|--------|-------------|
| 200 | OK | Request successful |
| 201 | Created | Resource created |
| 400 | Bad Request | Invalid request data |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict |
| 422 | Unprocessable Entity | Validation failed |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Service temporarily unavailable |

---

## SDK Examples

### JavaScript/TypeScript

```typescript
class GhostFlowClient {
  private baseUrl: string;
  
  constructor(baseUrl: string = 'http://localhost:3000/api') {
    this.baseUrl = baseUrl;
  }
  
  async createFlow(flow: FlowDefinition): Promise<Flow> {
    const response = await fetch(`${this.baseUrl}/flows`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(flow)
    });
    return response.json();
  }
  
  async executeFlow(flowId: string, input?: any): Promise<Execution> {
    const response = await fetch(`${this.baseUrl}/flows/${flowId}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ input })
    });
    return response.json();
  }
}
```

### Python

```python
import requests
from typing import Dict, Any, Optional

class GhostFlowClient:
    def __init__(self, base_url: str = "http://localhost:3000/api"):
        self.base_url = base_url
    
    def create_flow(self, flow: Dict[str, Any]) -> Dict[str, Any]:
        response = requests.post(f"{self.base_url}/flows", json=flow)
        response.raise_for_status()
        return response.json()
    
    def execute_flow(self, flow_id: str, input_data: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        payload = {"input": input_data} if input_data else {}
        response = requests.post(f"{self.base_url}/flows/{flow_id}/execute", json=payload)
        response.raise_for_status()
        return response.json()
```

### Curl Examples

```bash
# Create a flow
curl -X POST http://localhost:3000/api/flows \
  -H "Content-Type: application/json" \
  -d @flow-definition.json

# Execute a flow
curl -X POST http://localhost:3000/api/flows/550e8400-e29b-41d4-a716-446655440000/execute \
  -H "Content-Type: application/json" \
  -d '{"input": {"message": "Hello World"}}'

# Get execution status
curl http://localhost:3000/api/executions/660e8400-e29b-41d4-a716-446655440000
```

---

*API Reference version 1.0 - Last updated: 2025-01-08*