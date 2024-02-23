# Overview

## Application Package

Applications in Hermes are packaged and distributed using [HDF5] (Hierarchical Data Format Version 5) files.
For detailed information see [Packaging A Hermes Application](./application.md).

### Application Root

Each application package is an HDF5 file that stores all the static assets, data, and WASM modules at its root.
Each application package has associated metadata that is stored as [HDF5 Attributes] linked to the root path `/`.

### Application Metadata

Application metadata is stored as [CBOR] bytes.
The structure of the metadata is defined using [CDDL].
[Deterministic CBOR] encoding and decoding is required.

#### Diagram: Metadata is stored in HDF5 Attributes linked to stored objects

```kroki-excalidraw
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/hermes_app_metadata.excalidraw
```

For detailed information see [Defining Application Metadata](./metadata.md).

### Application Signatures

Application signatures are included in the metadata.
These signatures are [COSE](#cose) bytes of a deterministic hash of the contents of the Application Package.
Multisig support is required, so that Application Authors can sign, and the Application Publisher can verify and countersign
the Application without being able to modify the original contents.
For detailed information see [Hermes Application Signatures](./signatures.md).

### Application Static Assets

Static Assets consist of files that may be published only AS-IS via HTTP by the Application Modules.
Application Static Assets are found in the `/var/www` path.
For detailed information see [Packaging Static Files in a Hermes Application](./static.md).

### Application Data

Application Data is found in the `/var/data` path.
For detailed information see [Hermes Application Data](./data.md).

### Application Modules

Application Modules are found in the `/modules` path.
Individual WebAssembly Modules, and [WIT] definitions are stored in a subdirectory.
For example: `/modules/greeter` is the path to the `greeter` module.

#### Module Metadata

Application modules have associated metadata, including signatures.
Module metadata is linked to the module path that it describes.

#### Module Signatures

Same as with [Application Signatures](#application-signatures), Module Signatures are included in the metadata.
These signatures are [COSE](#cose) bytes of a deterministic hash of the contents of the Module contents.
Multisig support is required, so that Module Authors can sign, and the Module Publisher can verify and countersign the Module
without being able to modify the original contents.

## Additional Technical Resources

### HDF5

* [HDF5]
* [HDF5 Attributes]

### CBOR

* [CBOR]
* [Deterministic CBOR]
* [`dcbor`](https://github.com/BlockchainCommons/bc-dcbor-rust) Rust crate.
* [CDDL]

### WASI And WIT

* [WASI](https://wasi.dev/)
* [WIT]

### COSE

* [RFC 8152](https://www.rfc-editor.org/rfc/rfc8152)
* [RFC 9052](https://www.rfc-editor.org/rfc/rfc9052)
* [RFC 9053](https://www.rfc-editor.org/rfc/rfc9053)

[CBOR]: https://cbor.io/spec.html
[CDDL]: https://datatracker.ietf.org/doc/html/rfc8610
[Deterministic CBOR]: https://www.rfc-editor.org/rfc/rfc8949.html#name-deterministically-encoded-c
[HDF5]: https://docs.hdfgroup.org/hdf5/develop/
[HDF5 Attributes]: https://docs.hdfgroup.org/hdf5/develop/_h5_a__u_g.html#sec_attribute
[WIT]: https://component-model.bytecodealliance.org/design/wit.html
