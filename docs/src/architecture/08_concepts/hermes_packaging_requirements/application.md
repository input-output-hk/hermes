# Packaging A Hermes Application

Each Hermes application is distributed in an [HD5 File], which is organized as a rooted and directed graph.
For practical purposes, the objects in the graph are named HDF5 objects (defined in the [HDF5 Abstract Data Model]).
The graph is navigated in a similar fashion to POSIX file-systems, by concatenating object names with "`/`".

For example:

`/group1/group2/`" traverses the graph from the `root`, which contains `group1`, which contains and retrieves `group2`.

## Application Package Structure (HDF5 File)

### `/`

The root group of the Application file-system.

### `/static`

Static assets that can be served AS-IS to the local host over HTTP by WASM modules, or default values
used for configuring the Application, etc.

Files that are stored here are meant to be public.

### `/data`

Data that provides the dynamic functionality for WebAssembly modules.

Files that are stored here are meant to be private, and should only be accessed by WASM modules.

### `/modules`

WebAssembly modules, and WIT definitions.
Each application can store multiple modules.

### Example: A Hermes Application Package

```bash
/
├── static
│   ├── data
│   │   └── default
│   │       ├── config.json
│   │       └── ...
│   └── www
│       ├── site1
│       │  ├── html
│       │  └── ...
│       └── site2
│          ├── html
│          └── ...
├── data
│   ├── module1
│   │   ├── js_template
│   │   └── ...
│   └── ...
└── modules
    ├── module1
    │   ├── module1.wasm
    │   └── ...
    └── module2
        ├── module2.wasm
        └── ...
```

### Diagram: HDF5 file structure for a Hermes Application

```kroki-d2
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/hdf5_file_structure.dot
```

## Application Metadata (HDF5 Attributes)

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

### Other Stuff

### Diagram: HDF5 Attributes for a Hermes Application

```kroki-d2
@from_file:architecture/08_concepts/hermes_packaging_requirements/images/application_metadata.dot
```

[HD5 File]: https://docs.hdfgroup.org/hdf5/develop/_h5_d_m__u_g.html#title4
[HDF5 Abstract Data Model]: https://docs.hdfgroup.org/hdf5/develop/_h5_d_m__u_g.html#title2
