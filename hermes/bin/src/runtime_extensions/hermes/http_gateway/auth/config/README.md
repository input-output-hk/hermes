# Authentication and Authorization Configuration Guide

This guide explains how to configure auth.

## Overview

`auth.json` provides rules for each specific path that match the regular expression and a method.
The level of authentication and authorization depends on the `auth_level` where it can be:

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
