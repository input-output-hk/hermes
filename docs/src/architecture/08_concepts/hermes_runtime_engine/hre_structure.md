# HRE structure

[Hermes Runtime Extension (*HRE*)][*HRE*] - stands as logically separate module (like a library) of the [*Hermes engine*]
and provides an additional functionality to the Hermes runtime, therefore to [*Hermes application*].
[WIT] files represent a source of truth of the [*Hermes events*] and *HRE api* definitions for a specific [*HRE*],
and describe a standardized communication interface between [*Hermes application*]
and [*Hermes engine's*][*Hermes engine*] runtime itself.

Each [*HRE*] implementation take place inside `hermes/bin/srs/runtime_extensions` directory,
for both Hermes related and WASI specific.

Here is an example of the [WIT] files for the [hermes-cron] [*HRE*]:

`world.wit`

```wit
package hermes:cron;

world all {
    import api;
    export event;
}
```

`event.wit`

```wit
interface event {
    use api.{cron-event-tag, cron-tagged};

    on-cron: func(event: cron-tagged, last: bool) -> bool;
}

world cron-event {
    export event;
}
```

`api.wit`

```wit
interface api {    
    type cron-event-tag = string;

    type cron-sched = string;

    record cron-tagged {
        when: cron-sched,

        tag: cron-event-tag
    }

    add: func(entry: cron-tagged, retrigger: bool) -> bool;
    rm: func(entry: cron-tagged) -> bool;
    
    ...
}

world cron-api {
    import api;
}
```

*Hermes events*:

* `on-cron: func(event: cron-tagged, last: bool) -> bool;`
  
*HRE api*:

* `add: func(entry: cron-tagged, retrigger: bool) -> bool;`
* `rm: func(entry: cron-tagged) -> bool;`

## Host implementation structure

The Hermes host runtime is implemented using the [wasmtime].
It automatically generates code based on the WIT files:

```Rust
use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "path/to/the/wit/files/dir",
});
```

Internally, it generates a diverse set of traits, structs, functions, and more derived from the WIT files.
This process results in a type-safe interface for interacting with WASM modules and implementing host functionalities.

All host implementations specific to a particular [*HRE*] are defined within the corresponding
`host.rs` files.

For example `../hermes/cron/host.rs`:

```Rust
use crate::{
    runtime_extensions::{
        bindings::{
            hermes::cron::api::{CronEventTag, CronTagged, Host},
            wasi::clocks::monotonic_clock::Instant,
        },
    },
    state::HermesState,
};


impl Host for HermesState {

    fn add(&mut self, entry: CronTagged, retrigger: bool) -> wasmtime::Result<bool> {
        ...
    }

    fn rm(&mut self, entry: CronTagged) -> wasmtime::Result<bool> {
        ...
    }
    ...
}
```

All [*Hermes events*] implementations specific to a particular [*HRE*] are defined within the corresponding
`event.rs` files.

For example `../hermes/cron/event.rs`:

```Rust
/// On cron event
struct OnCronEvent {
    /// The tagged cron event that was triggered.
    tag: CronTagged,
    /// This cron event will not retrigger.
    last: bool,
}

impl HermesEventPayload for OnCronEvent {
    fn event_name(&self) -> &str {
        "on-cron"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
       module.instance.hermes_cron_event().call_on_cron(
            &mut module.store,
            &self.tag,
            self.last,
        )?;
        Ok(())
    }
}
```

***NOTE*** that these [*Hermes event*][*Hermes events*] host definitions
are not an implementation of the [*Hermes event*][*Hermes events*] itself.
It is a way how to execute [*Hermes event*][*Hermes events*]
and pass corresponding data for the [*Hermes event*][*Hermes events*] handler,
implemented by the [*Hermes application*],
inside [*Hermes engine*] runtime.

[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[hermes-cron]: https://github.com/input-output-hk/hermes/tree/main/wasm/wasi/wit/deps/hermes-cron
[*Hermes engine*]: ./../../05_building_block_view/hermes_engine.md#hermes-engine
[*Hermes application*]: ./../../05_building_block_view/hermes_engine.md#hermes-application
[*Hermes events*]: ../../05_building_block_view/hermes_engine.md#hermes-event
[*HRE*]: ../../05_building_block_view/hermes_engine.md#hermes-runtime-extension-hre
[wasmtime]: https://docs.wasmtime.dev/introduction.html
