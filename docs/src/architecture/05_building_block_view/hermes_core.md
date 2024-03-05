---
icon: material/engine
---

# Hermes engine

*Hermes engine* represents an event-driven system running on top of the WASM runtime utilizing
[WASM component model](https://component-model.bytecodealliance.org/design/why-component-model.html) approach.
Every user's application is expected to be compiled as a WASM module,
which means that it could be developed on any language with the WASM support e.g. Java, C, Rust, Go etc.

```kroki-excalidraw
@from_file:architecture/05_building_block_view/images/hermes_core.excalidraw
```

## Hermes runtime extension (HRE)

*Hermes runtime extension (HRE)* - a Hermes module
which will provides an additional functionality, besides the  to the [*Hermes application*] and stands as a library.
It defines the following parts:

* [*Hermes events*].
* *HRE api* - defines a set of types and functions which could be used from WASM by the [*Hermes application*].
  
Specification of the [*Hermes events*] and *HRE api* defined in [WIT] files.

## Hermes event

*Hermes event* - an event produced and defined by [*HRE*] that encapsulates all the necessary data needed to process it.
After successful delivery, each event can be executed by the [*Hermes application*],
depending on whether that specific [*Hermes application*] has subscribed to such events or not.

## Hermes events queue

*Hermes events queue* - a queue-like data structure.
[*Hermes events*] are added to the one end, one by one, by the [*HRE*].
The [*Hermes application*] then executes/consumes these events from the other end of the queue.
The queue preserves the order of event execution based on how they were added in it.

## Hermes application

*Hermes application* - a collection of WASM components, which are packed together and executes a specific business logic.
It mainly serves as an event handler for the of the [*Hermes Events*].
Each *Hermes application* can interact with the [*HRE*] through the defined *HRE api* based on corresponding
[WIT] definitions.

[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[*HRE*]: #hermes-runtime-extension-hre
[*Hermes Events*]: #hermes-event
[*Hermes application*]: #hermes-application
