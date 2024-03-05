# Hermes application structure

[*Hermes application*] it is a collection of compiled
[WASM components](https://component-model.bytecodealliance.org/introduction.html),
[*HRE* config files](./hre_init_setup.md)
and some metadata
bundled in [HDF5] file format.
Each WASM component it is the event handlers implementation of `export` functions from the [WIT] file,
specified by the [*Hermes runtime extension*].

For more details, see [Hermes Application Package](../hermes_packaging_requirements/overview.md)

[WIT]: https://component-model.bytecodealliance.org/design/wit.html
[*Hermes runtime extension*]: ./../../05_building_block_view/hermes_core.md#hermes-runtime-extension-hre
[*Hermes application*]: ./../../05_building_block_view//hermes_core.md#hermes-application
[HDF5]: https://www.hdfgroup.org/solutions/hdf5/
