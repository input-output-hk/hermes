# Hermes application structure

[*Hermes application*](./../05_building_block_view//hermes_core.md#hermes-application)
as it was told before it is a collection of compiled
[WASM components](https://component-model.bytecodealliance.org/introduction.html),
[*HRE* config files](./hre_init_setup.md)
and some metadata
bundled in [hdf5](https://www.hdfgroup.org/solutions/hdf5/) package.
Each WASM component it is the event handlers implementation of `export` functions from the WIT file, specified by the *HRE*.

Package structure

```bash
├── module1.wasm
├── module2.wasm
├── config.json
└── METADATA
```
