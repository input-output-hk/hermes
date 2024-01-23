# HRE interface

Each [Hermes Runtime Extension (*HRE*)][*HRE*] defines a
[WIT](https://component-model.bytecodealliance.org/design/wit.html) file.
It is a 1 on 1 match, so every [*HRE*] has to have a corresponding [WIT] file.
It specifies the following important parts:

* *Hermes events* signature, which produced by the corresponding [*HRE*].
* *HRE calls* which could invoked by the *Hermes application*.

Example of the [WIT] file for the [cron](https://en.wikipedia.org/wiki/Cron) [*HRE*]:

```wit
package hermes:cron;

interface cron-types {
    record cron-tagged {
        /// The crontab entry in standard cron format.
        /// The Time is ALWAYS relative to UTC and does not account for local time.
        /// If Localtime adjustment is required it must be handled by the module.
        when: string,

        /// The tag associated with the crontab entry.
        tag: string
    }
}

interface cron-events {
    use cron-types.{cron-tagged};

    /// Triggered when a cron event fires.
    ///
    /// This event is only ever generated for the application that added
    /// the cron job.
    on-cron: func(event: cron-tagged, last: bool) -> bool;
}

interface cron-calls {
    use cron-types.{cron-tagged};

    /// # Schedule Recurrent CRON event
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    add: func(entry: cron-tagged, retrigger: bool) -> bool;

    /// # Remove the requested crontab.
    ///
    /// Allows for management of scheduled cron events.
    rm: func(entry: cron-tagged) -> bool;
}

world cron {
  import cron-calls;
  export cron-events;
}
```

*Hermes events*:

* `on-cron: func(event: cron-tagged, last: bool) -> bool`
  
*HRE calls*:

* `add: func(entry: cron-tagged, retrigger: bool) -> bool`
* `rm: func(entry: cron-tagged) -> bool`

[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[*HRE*]: ../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
