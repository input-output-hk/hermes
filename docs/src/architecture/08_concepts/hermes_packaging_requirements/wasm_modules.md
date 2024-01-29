# Packaging WebAssembly (WASM) modules in a Hermes Application

```kroki-d2
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/wasm_module_metadata.dot
```

** SJ Notes **
Wasm modules need the following data:

1. The compiled WASM code
2. Metadata about the wasm module
   1. Name
   2. Author
   3. Repo
   4. License
   5. etc.
4. Configuration data (Json or CBOR) that parameterises the module.
   1. Json schema for "Preset" parameters.  These are not changeable by the user.
   2. Json schema for "Configurable" parameters.  These can be set by the user before the app runs.
   3. The actual "preset" parameters themselves (which must be valid according to the "preset" parameter schema.
5. All of the above should be encoded using CBOR (Needs to be defined with a valid CDDL schema).
6. A Signature of the Module Author and/or publisher validating its the code they "released".
   1. For this we should wrap all the above in COSE (See: RFC-9052, RFC-9053, RFC-0338)
   2. All CBOR is to be specified as needing to be encoded following the deterministic CBOR Rules.  See https://developer.blockchaincommons.com/dcbor/ and https://datatracker.ietf.org/doc/draft-mcnally-deterministic-cbor/
