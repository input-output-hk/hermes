# Packaging A Hermes Application

Each Hermes application is distributed in an [HDF5 File], which is organized as a rooted and directed graph.
For practical purposes, the objects in the graph are named HDF5 objects (defined in the [HDF5 Abstract Data Model]).
The graph is navigated in a similar fashion to POSIX file-systems, by concatenating object names with "`/`".

For example:

`/group1/group2/`" traverses the graph from the `root`, which contains `group1`, which contains and retrieves `group2`.

## Prerequisites to Packaging an Application

1. A valid `metadata.json` for the application.
2. Any files required for static data need to be prepared and ready for inclusion.
3. Any files for `/usr/lib` need to be prepared and ready for inclusion.
4. Each WASM Component Module required needs to be pre-packaged and signed.
5. The `hermes` engine executable.

### Tooling

All Packaging operations can be performed with the `hermes` engine when used from the command line.

To get a list of options related to packaging:

```sh
./hermes package --help
```

## The Application Packaging Process

1. Create an unsigned Application Package.
2. Sign it as one or more authors.
3. *Optionally, Sign it as one or more publishers.*

### Creating the unsigned Application Package

```sh
./hermes package app <manifest.json> <app_package_name>
```

* `manifest.json` - Defines the location of all the src artifacts needed to build the package.
  This file must conform to the manifests [json schema](#manifest-schema).
  An example manifest of this [json schema](#manifest-schema) if [here](#manifest-example).
* `app_package_name` - The name to give the application file.

*Note: the extension `.happ` will automatically be added to the `<app_package_name>` to signify this is a Hermes App.*

### Signing the Application Package

As the author of the Application:

```sh
./hermes package sign <X.509 Private Cert> <app_package_name>
```

This takes the X.509 Private Certificate presented, and signs or counter-signs the Application package.

As the publisher of the Application:

```sh
./hermes package sign --publisher <X.509 Private Cert> <app_package_name>
```

This takes the X.509 Private Certificate presented, and signs or counter-signs the Application package.

*Note: A Hermes Application is INVALID if it does not contain at least 1 Author Signature.*

### Inspecting a Hermes Application

```sh
./hermes package inspect <app_package_name>
```

This command will dump the logical contents of the Application package and if it is considered valid or not.
It does not extract files from the package.  
If files need to be extracted or individually accessed outside of Hermes, any [HDF5 Viewer] can be used.

## Manifest Schema

<!-- markdownlint-disable max-one-sentence-per-line code-block-style -->

??? note "Schema: `hermes_app_manifest.schema.json`"

    ```json
    {{ include_file('includes/schemas/hermes_app_manifest.schema.json', indent=4) }}
    ```

## Manifest Example

??? note "Example: `hermes_app_manifest.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_app_manifest.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line code-block-style-->

[HDF5 Viewer]: https://myhdf5.hdfgroup.org/
[HDF5 File]: https://docs.hdfgroup.org/hdf5/develop/_h5_d_m__u_g.html#title4
[HDF5 Abstract Data Model]: https://docs.hdfgroup.org/hdf5/develop/_h5_d_m__u_g.html#title2
