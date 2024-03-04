# HRE design

[Hermes Runtime Extension (*HRE*)][*HRE*] - a set of [*Hermes events*] and *HRE api* defined in the [WIT] files.

Example of the [WIT] file for the some [cron](https://en.wikipedia.org/wiki/Cron) [*HRE*]
(It is an example, not related to what is implemented in Hermes):

```wit
package hermes:cron;

interface cron-api {
    record cron-tagged {
        /// The crontab entry in standard cron format.
        /// The Time is ALWAYS relative to UTC and does not account for local time.
        /// If Localtime adjustment is required it must be handled by the module.
        when: string,

        /// The tag associated with the crontab entry.
        tag: string
    }

    /// # Schedule Recurrent CRON event
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    add: func(entry: cron-tagged, retrigger: bool) -> bool;

    /// # Remove the requested crontab.
    ///
    /// Allows for management of scheduled cron events.
    rm: func(entry: cron-tagged) -> bool;
}

interface cron-events {
    use cron-types.{cron-api};

    /// Triggered when a cron event fires.
    ///
    /// This event is only ever generated for the application that added
    /// the cron job.
    on-cron: func(event: cron-tagged, last: bool) -> bool;
}

world cron {
  import cron-api;
  export cron-events;
}
```

*Hermes events*:

* `on-cron: func(event: cron-tagged, last: bool) -> bool`
  
*HRE api*:

* `add: func(entry: cron-tagged, retrigger: bool) -> bool`
* `rm: func(entry: cron-tagged) -> bool`

## Host implementation
 
Hermes host runtime implementation based on the [`wasmtime`] Rust library.
It starts from the specifying the path to the [WIT] files to use and define for Hermes.

```Rust
use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "path/to/the/wit/files/dir",
});
```



[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[*Hermes events*]: ../../05_building_block_view/hermes_core.md#hermes-event
[*HRE*]: ../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
[`wasmtime`]: https://github.com/bytecodealliance/wasmtime
