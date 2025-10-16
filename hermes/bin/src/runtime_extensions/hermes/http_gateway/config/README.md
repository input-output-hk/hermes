# HTTP Gateway Configuration Guide

This guide explains how to configure HTTP endpoint subscriptions for the Hermes WebAssembly gateway using `endpoints.json`.

## Overview

The HTTP gateway uses endpoint subscriptions to intelligently route incoming HTTP requests to specific WebAssembly modules instead of broadcasting to all modules. This improves performance and allows for more granular request handling.

## Configuration Structure

The configuration file should contain an array of endpoint subscription objects:

```json
{
  "subscriptions": [
    {
      "subscription_id": "unique_identifier",
      "module_id": "target_wasm_module",
      "methods": ["GET", "POST"],
      "path_regex": "^/api/users(/.*)?$",
      "content_types": ["application/json"],
      "json_schema": "optional-schema-file.json"
    }
  ]
}
```

## Field Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `subscription_id` | `string` | Unique identifier for logging and debugging |
| `module_id` | `string` | Name of the WebAssembly module that will handle matching requests |
| `path_regex` | `string` | Regular expression pattern to match request URLs |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `methods` | `array<string>` | `[]` (all methods) | HTTP methods this subscription accepts |
| `content_types` | `array<string>` | `[]` (all types) | MIME types this subscription accepts |
| `json_schema` | `string` | `null` | Path to JSON schema file for request validation |

## Configuration Examples

### Basic API Route

```json
{
  "endpoint_subscriptions": [
    {
      "subscription_id": "basic_api",
      "module_id": "api_handler",
      "path_regex": "^/api/.*$"
    }
  ]
}
```

### Specific User Management

```json
{
  "endpoint_subscriptions": [
    {
      "subscription_id": "user_crud",
      "module_id": "user_service",
      "methods": ["GET", "POST", "PUT", "DELETE"],
      "path_regex": "^/api/v1/users(/[0-9]+)?$",
      "content_types": ["application/json"]
    }
  ]
}
```

### Multiple Endpoints with Validation

```json
{
  "endpoint_subscriptions": [
    {
      "subscription_id": "user_registration",
      "module_id": "auth_service",
      "methods": ["POST"],
      "path_regex": "^/api/v1/register$",
      "content_types": ["application/json"],
      "json_schema": "schemas/user-registration.json"
    },
    {
      "subscription_id": "user_login",
      "module_id": "auth_service", 
      "methods": ["POST"],
      "path_regex": "^/api/v1/login$",
      "content_types": ["application/json"],
      "json_schema": "schemas/user-login.json"
    },
    {
      "subscription_id": "static_files",
      "module_id": "file_server",
      "methods": ["GET"],
      "path_regex": "^/static/.*\\.(css|js|png|jpg|gif)$"
    }
  ]
}
```

## Regex Pattern Guide

### Common Patterns

| Pattern | Description | Matches |
|---------|-------------|---------|
| `^/api/users$` | Exact match | `/api/users` only |
| `^/api/users/.*$` | Users with path | `/api/users/123`, `/api/users/profile` |
| `^/api/users/[0-9]+$` | Users with numeric ID | `/api/users/123`, `/api/users/456` |
| `^/api/users(/.*)?$` | Users with optional path | `/api/users`, `/api/users/123` |
| `^/static/.*\\.(css\|js)$` | Static CSS/JS files | `/static/app.css`, `/static/main.js` |

### Best Practices for Regex

1. **Use anchors**: Always start with `^` and end with `$` for precise matching
2. **Escape special characters**: Use `\\.` for literal dots, `\\|` for literal pipes
3. **Be specific**: More specific patterns get higher priority
4. **Test your patterns**: Use online regex testers to verify behavior

## HTTP Methods

Common method combinations:

```json
// Read-only endpoints
"methods": ["GET"]

// Full CRUD operations  
"methods": ["GET", "POST", "PUT", "DELETE"]

// Data submission endpoints
"methods": ["POST", "PUT"]

// Accept all methods (default)
"methods": []
```

## Content Types

Common content type configurations:

```json
// JSON APIs
"content_types": ["application/json"]

// File uploads
"content_types": ["multipart/form-data"]

// Web forms
"content_types": ["application/x-www-form-urlencoded"]

// Multiple types
"content_types": ["application/json", "text/xml"]

// Accept all types (default)
"content_types": []
```

## Specificity and Priority

The gateway uses a specificity scoring system to determine which subscription handles a request when multiple patterns match:

### High Priority (Score: 25+)
- Very specific regex patterns with many literal characters
- Anchored patterns with character classes: `^/api/users/[0-9]+/profile$`

### Medium Priority (Score: 15-24)  
- Moderately specific patterns with method/content-type restrictions
- Patterns with some wildcards: `^/api/v1/users/.*$`

### Low Priority (Score: <15)
- General catch-all patterns: `^/api/.*$` or `.*`
- No method or content-type restrictions

### Priority Examples

```json
{
  "endpoint_subscriptions": [
    {
      "subscription_id": "specific_user_profile",
      "module_id": "user_service", 
      "methods": ["GET", "PUT"],
      "path_regex": "^/api/v1/users/[0-9]+/profile$",
      "content_types": ["application/json"]
      // High priority - very specific
    },
    {
      "subscription_id": "general_users",
      "module_id": "user_service",
      "path_regex": "^/api/v1/users/.*$"
      // Medium priority - somewhat specific
    },
    {
      "subscription_id": "api_catchall", 
      "module_id": "default_handler",
      "path_regex": "^/api/.*$"
      // Low priority - catch-all pattern
    }
  ]
}
```

## Complete Example

```json
{
  "endpoint_subscriptions": [
    {
      "subscription_id": "user_management_api",
      "module_id": "user_service",
      "methods": ["GET", "POST", "PUT", "DELETE"],
      "path_regex": "^/api/v1/users(/[0-9]+)?(/[a-z]+)?$",
      "content_types": ["application/json"],
      "json_schema": "schemas/user.json"
    },
    {
      "subscription_id": "authentication",
      "module_id": "auth_service",
      "methods": ["POST"],
      "path_regex": "^/api/v1/(login|logout|register)$",
      "content_types": ["application/json"]
    },
    {
      "subscription_id": "file_uploads",
      "module_id": "file_service",
      "methods": ["POST", "PUT"],
      "path_regex": "^/api/v1/upload(/.*)?$",
      "content_types": ["multipart/form-data"]
    },
    {
      "subscription_id": "health_check",
      "module_id": "monitoring_service",
      "methods": ["GET"],
      "path_regex": "^/(health|status)$"
    },
    {
      "subscription_id": "default_api_handler",
      "module_id": "default_service",
      "path_regex": "^/api/.*$"
    }
  ]
}
```

## Troubleshooting

### Common Issues

1. **No matches found**: Check that your regex patterns are correctly escaped and anchored
2. **Wrong module selected**: More specific patterns override general ones - check specificity scores
3. **Method not accepted**: Ensure the HTTP method is in the `methods` array or leave it empty for all methods
4. **Content-type rejection**: Verify content types match or use empty array for all types

### Testing Your Configuration

1. **Regex testing**: Use tools like https://regex101.com/ to test your patterns
2. **Priority testing**: More specific patterns should have higher scores
3. **Log monitoring**: Check gateway logs for subscription matching information

### Debug Logging

The gateway logs subscription matching decisions:

```
[INFO] Routing HTTP request to specific module: user_service
[INFO] Found subscription for POST /api/v1/users/create: module user_service  
[INFO] Module 'nonexistent_module' not found in available modules: ["user_service", "auth_service"], broadcasting to all
```

## Migration and Updates

When updating configurations:

1. **Test in development** before deploying to production
2. **Backup existing config** before making changes  
3. **Monitor logs** after deployment to ensure correct routing
4. **Gradual rollout** for major routing changes