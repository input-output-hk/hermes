# Overview

## Application Package

Applications in Hermes are packaged and distributed using [HDF5] (Hierarchical Data Format Version 5) files.

For detailed information see [Packaging A Hermes Application](./application.md).

### HDF5 File And Data Model

## Package Metadata

For detailed information see [Defining Application Metadata](./metadata.md).

### Diagram: Metadata is stored in HDF5 Attributes linked to stored objects

```kroki-excalidraw
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/hermes_app_object_metadata.excalidraw
```

### HDF5 Attributes

* [HDF5 Attributes](https://docs.hdfgroup.org/hdf5/develop/_h5_a__u_g.html#sec_attribute)

### CBOR And dCBOR

* [CBOR Specification](https://cbor.io/spec.html)
* [`dcbor`](https://github.com/BlockchainCommons/bc-dcbor-rust) Rust crate.

### CDDL

## Application Static Assets

For detailed information see [Packaging Static Files in a Hermes Application](./static.md).

## Application Data

For detailed information see [Hermes Application Data](./data.md).

## Application Modules

### WASI And WIT

* [WASI](https://wasi.dev/)
* [WIT](https://component-model.bytecodealliance.org/design/wit.html)

## Package Signatures

For detailed information see [Hermes Application Signatures](./signatures.md).

### COSE

* [RFC 8152](https://www.rfc-editor.org/rfc/rfc8152)
* [RFC 9052](https://www.rfc-editor.org/rfc/rfc9052)
* [RFC 9053](https://www.rfc-editor.org/rfc/rfc9053)

## **SJ Notes**

Each package needs metadata which can be associated with it.
So we may want to define that in its own file.

Example metadata are similar to a wasm module.

We also need to sign the package, and it needs to be able to support multisig, so it can be signed by the Author and then
countersigned by a publisher.

Similar to WASM modules, we will use COSE (See my notes on wasm modules).
Because we are signing the Application file, we need a way to generate a secure hash over all the data in the package, and
then just sign that hash, and embed it.

We won't be signing the whole HDF5 file itself.

But because we are getting a deterministic hash from the data inside the HDF5 file for the signature, the only thing that can
change is the signature itself.

This would allow an Author to sign it, pass it to a publisher who validates it, and just countersigns it.

 The publisher can't alter any of files, or the Authors signature will fail.

[HDF5]: https://docs.hdfgroup.org/hdf5/develop/
