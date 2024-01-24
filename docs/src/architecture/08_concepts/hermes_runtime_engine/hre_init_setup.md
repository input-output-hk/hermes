# HRE initial setup

To properly setup a [*Hermes runtime extension (HRE)*][*HRE*] module
a specific configuration file should be provided by the [*Hermes application*].
This configuration file is loaded during the [*HRE*] initialization process
and provides a necessary data to start a specific [*HRE*] for a specific [*Hermes application*].

Each [*HRE*] defines a JSON schema of the desired configuration for it.
For example for some kind of networking [*HRE*] module a json schema could look like:

```json
{
    "host": {
      "type": "string"
    },
    "port": {
      "type": "integer"
    },
    "timeout": {
      "type": "integer",
      "minimum": 0
    },
    "maxConnections": {
      "type": "integer",
      "minimum": 1
    },
}
```

[*Hermes application*]: ./../../05_building_block_view/hermes_core.md#hermes-application
[*HRE*]: ./../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
