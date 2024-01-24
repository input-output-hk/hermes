---
icon: material/strategy
---

# Solution Strategy

<!-- See: https://docs.arc42.org/section-4/ -->

| Goal/Requirement | Solution | Details |
|-|-|-|
| Flexible and modular backend engine to run decentralized applications | The event-driven system is built on the WASM runtime using the [WASM component model](https://component-model.bytecodealliance.org/design/why-component-model.html) approach| [link](./05_building_block_view/hermes_core.md) |
| WASM application packaging | Use [HDF5](https://www.hdfgroup.org/solutions/hdf5/) framework to bundle source code and some other metadata into one file | [lint](./08_concepts/index.md#hermes-application-structure) |
