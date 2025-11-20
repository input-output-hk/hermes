# Authentication and Authorization Configuration Guide

This guide explains how authentication and authorization behave in the system and how to configure them.

## Overview

Authentication can be globally enabled or disabled using the environment variable `HERMES_AUTH_ACTIVATE`

* `HERMES_AUTH_ACTIVATE=true` -> Auth validation enabled
* `HERMES_AUTH_ACTIVATE=false` -> Auth validation disabled (all requests bypass auth logic)
If the variable is not set or contains any other value, the default is true

When auth is enabled, the `auth.json` file provides rules for each path (matching via regex) and the HTTP method.
Each rule specifies an `auth_level`, which determines how the request is validated.

```rust
// Auth levels for specific routes
pub enum AuthLevel {
    Required,  // Authentication is mandatory
    Optional,  // Authentication is optional (validates if present)
    None,      // No authentication required
}
```

If the rule for a specific path is not defined, the default level will be applied.

## Configuration File Format

The `auth.json` file defines auth rules using the following structure:

```json
{
    "auth_rules": [
        {
            "path_regex": "^/api/v1/registration(/.*)?$",
            "method": "GET",
            "auth_level": "optional"
        },
        {
            "path_regex": "^/api/v1/document$",
            "method": "PUT",
            "auth_level": "required"
        }
    ],
    "default_auth_level": "none"
}

### Configuration Fields

- **`auth_rules`**: Array of auth rules for specific paths
- **`path_regex`**: Regular expression pattern to match request paths
- **`method`**: HTTP method (GET, POST, PUT, DELETE, etc.)
- **`auth_level`**: Authentication level for matching requests
- **`default_auth_level`**: Default authentication level for unmatched paths

### Auth Level Values

- **`"required"`**: Authentication is mandatory - requests without valid tokens are rejected
- **`"optional"`**: Authentication is optional - tokens are validated if present, but requests without tokens are allowed
- **`"none"`**: No authentication required - all requests are allowed regardless of token presence
