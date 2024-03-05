# HRE interface

[Hermes Runtime Extension (*HRE*)][*HRE*] - a set of [*Hermes events*] and *HRE api* defined in the [WIT] files.

Each [*HRE*] implementation take place inside `hermes/bin/srs/runtime_extensions` directory,
for both Hermes related and WASI specific.

Here is an example of the [WIT] files for the some [hermes-cron] [*HRE*]:

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

Hermes host runtime implementation based on the [`wasmtime`] Rust library.
It auto-generates a code based on the [WIT] files:

```Rust
use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "path/to/the/wit/files/dir",
});
```

Under the hood it generates a set of different traits, structs, functions etc.
which are based on the [WIT] files and provides a type safe interface for interaction
with WASM modules and for implementation host functionality.

All host implementations for a specific [*HRE*]
are defined inside corresponded `../hermes/cron/host.rs` and could look like this:

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

All [*Hermes events*] implementations for a specific [*HRE*]
are defined inside corresponded `../hermes/cron/event.rs` and could look like this:

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

[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[hermes-cron]: https://github.com/input-output-hk/hermes/tree/main/wasm/wasi/wit/deps/hermes-cron
[*Hermes events*]: ../../05_building_block_view/hermes_core.md#hermes-event
[*HRE*]: ../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
[`wasmtime`]: https://docs.wasmtime.dev/introduction.html
