# Hermes application execution

```kroki-excalidraw
@from_file:architecture/08_concepts/hermes_runtime_engine/images/hermes_application.excalidraw
```

Basically, the [*Hermes application*] is a set of [*Hermes event*] handler functions and nothing more.
The source code could be split into different WASM components,
but they will have the same state specified for this [*Hermes application*].

Application's state initializes during the application initializing process
and mainly based on the configuration of the [*Hermes runtime extensions*] config files.

For each event handling execution,
the application's state remains **consistent** and **immutable**.
It means that before any event processing,
it is made a copy of the initial application's state,
this copy used during the execution and removed after it.
So the overall application state remains the same.

[*Hermes event*]: ./../../05_building_block_view/hermes_core.md#hermes-event
[*Hermes runtime extensions*]: ./../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
[*Hermes application*]: ./../../05_building_block_view/hermes_core.md#hermes-application
