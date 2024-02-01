# Defining Application Metadata

Metadata for Hermes Applications is specified as an HDF5 Attribute attached to the root group
in the HDF5 File as a CBOR-encoded object.

## Application Metadata (HDF5 Attribute)

### Diagram: Hermes Application Metadata Stored as CBOR-encoded HDF5 Attributes

```kroki-d2
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/application_metadata.dot
```

### CBOR-encoded Application Metadata

#### Application Author

#### Version

#### Repo

#### License

#### Description

#### Configuration Schema

For its configuration, so a user can configure it.

1. This is the only config a user actually sets,  it will embed (and can have references to that config)
    from individual wasm modules.
2. We need to work out how this will work and define it.

#### Signatures

#### Extra
