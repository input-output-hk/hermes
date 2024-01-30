# Packaging Overview

Applications in Hermes are packaged and distributed using
[HDF5](https://docs.hdfgroup.org/hdf5/develop/) (Hierarchical Data Format Version 5) files.

## Hermes HDF5 File Metadata

1. App Author
2. Version
3. Repo
4. License
5. Description
6. Json Schema for its configuration, so a user can configure it.
    1. This is the only config a user actually sets,  it will embed (and can have references to that config)
        from individual wasm modules.
    2. We need to work out how this will work and define it.
7. other stuff

### App Author

### Version

### Repo

### License

### Description

### Json Schema

For its configuration, so a user can configure it.

1. This is the only config a user actually sets,  it will embed (and can have references to that config)
    from individual wasm modules.
2. We need to work out how this will work and define it.

### Extra

## Hermes HDF5 File Structure

```kroki-d2
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/hdf5_file_structure.dot
```

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
