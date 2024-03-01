# Overview

## Hermes Applications and Filesystem Access

Hermes applications are fully sandboxed.  
They do not have direct access to Filesystem resources on the host.

However internally, a Hermes application will see a filesystem hierarchy that represents
all the files that it may read and/or write.

Hermes presents a filesystem hierarchy to the application which is adapted from the [Linux Filesystem Hierarchy Standard V3].
Hermes application packages are structured around this standard to  maximize consistency.

How Hermes manages and organizes application files is necessary when considering the structure of the Application package itself.

## Components to a Hermes Application

Each application consists of two HDF5 files.

1. The Application package itself.
2. The Applications persistent re-writable data.

The application itself sees a unified view of these two files.
Data is divided between them to make merging those views easy and consistent.

## The Full Application Filesystem Hierarchy

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | Writable | Required |
| --- | ----------- | ---- | -------- | --- |
| `/`   | :octicons-file-directory-fill-16: | Root Directory | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/tmp` | :octicons-file-directory-16: | Temporary Files stored in memory | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/etc` | :octicons-file-directory-fill-16: | Writable settings | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/etc/settings.json` | :octicons-file-16: | Hermes Engine settings for this application. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/etc/<module-name>/settings.json` | :octicons-file-16: | Module specific</br>Runtime Configurable Settings | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/srv` | :octicons-file-directory-fill-16: | Data which is served by this system. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/srv/www` | :octicons-file-directory-fill-16: | Files automatically served for this application on HTTP. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/srv/share` | :octicons-file-directory-fill-16: | Data files which are not automatically served but can be shared by all Wasm Modules in the application. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/usr` | :octicons-file-directory-fill-16: |  Shareable, read-only data. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/usr/lib` | :octicons-file-directory-fill-16: |  Application over-rides for webasm library modules. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/usr/lib/<module-name>` | :octicons-file-directory-fill-16: |  Application over-rides for named webasm library module. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/usr/lib/<module-name>/config.json` | :octicons-file-16: |  Config to use for the module instead of its bundled config. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/usr/lib/<module-name>/share` | :octicons-file-directory-fill-16: | Overrides for a modules shareable readonly data | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/lib` | :octicons-file-directory-fill-16: | Wasm Component Module Library Directory | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/lib/<module-name>/metadata.json` | :octicons-file-16: | Modules Metadata | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/lib/<module-name>/module.wasm` | :octicons-file-binary-16: | Actual WASM Module | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/lib/<module-name>/config.schema.json` | :octicons-file-16: | Modules Fixed Configuration Schema | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/lib/<module-name>/config.json` | :octicons-file-16: | Modules Fixed Configuration (Must match the schema) | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/lib/<module-name>/settings.schema.json` | :octicons-file-16: | Modules User Settings Schema | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/lib/<module-name>/share` | :octicons-file-directory-fill-16: | Modules shareable readonly data | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/lib/<module-name>/author.cose` | :octicons-file-badge-16: | Modules Author Signature | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/var/` | :octicons-file-directory-fill-16: |  Contains variable data files. (Persistent) | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/metadata.json` | :octicons-file-16: | Applications Metadata | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/author.cose` | :octicons-file-badge-16: | Application Author Signature | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/publisher.cose` | :octicons-file-badge-16: | Application Publisher Signature | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |

### Icon Legend

* :octicons-file-directory-fill-16: - Directory (Persistent)
* :octicons-file-directory-16: - Directory (Temporary)
* :octicons-file-16: - General File
* :octicons-file-binary-16: - Binary File
* :octicons-file-badge-16: - [COSE](#cose) Certificate
* <span style="color: orange;">:octicons-circle-16:</span> - NO
* <span style="color: green;">:octicons-check-circle-fill-12:</span> - YES

<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

### Writable Data

A Hermes application can have access to several writable data source.

1. Databases - These are not described here and are documented elsewhere.
2. [Temporary file storage](#temporary-file-storage) - Located in ram and not persisted between invocations of Hermes.
3. [Application writable and persistent storage](#application-writable-and-persistent-storage).

#### Temporary File Storage

If so configured, Hermes can provide a fixed size re-writable in-memory file system to a Hermes Application.
The application will see it at `/tmp` and it can use it like any normal filesystem.
It has a maximum available size, defined by the user.
Attempts to write more data than configured will fail.
If no temporary storage has been provided to the application it will not see a `/tmp` directory in its directory hierarchy.

#### Application writable and persistent storage

The application will also be given a re-writable and persistent file storage.
Like the Temporary File Storage it is configured with a maximum size.
Attempts by an application to use more storage than configured will fail.

Some data within the Application persistent writable storage is not actually writable by the application directly.
This data is used by Hermes to store persistent configuration.  
It can be read by the application.
This prevents the application from re-writing application settings without the users permission.

From the full Filesystem Hierarchy, the following directories and files are contained in the Application writable and persistent storage.

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | App Writable | Hermes Writable |
| --- | ----------- | ---- | -------- | --- |
| `/`   | :octicons-file-directory-fill-16: | Root Directory | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/etc` | :octicons-file-directory-fill-16: | Writable settings | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/etc/settings.json` | :octicons-file-16: | Hermes Engine settings for this application. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/etc/<module-name>/settings.json` | :octicons-file-16: | Module specific</br>Runtime Configurable Settings | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/var` | :octicons-file-directory-fill-16: |  Contains variable data files. (Persistent) | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

The application can store any configuration it likes into `/etc`.
Provided it does not modify or delete any settings.json files managed by Hermes itself.
The application can read these files at any time.

The application can store any data it requires, with any organization it needs in the `/var` directory.
The only checks performed by hermes on these files are to ensure that the maximum size of the file system is not exceeded.

These files/directories may *NEVER* appear in a Hermes application package.

The READ ONLY portions of the File System are linked to in the Application Package.
During the linking process,  file over-rides are processed and cross linked.

This simplifies both the Application logic and Hermes engine as the correct file will appear to
exist in the most logical location.

See [Loading a Hermes Application](./app_loading.md)

## Application Package

Applications in Hermes are packaged and distributed using [HDF5] (Hierarchical Data Format Version 5) files.
For detailed information see [Packaging A Hermes Application](./app_package.md).

Application packages consist of the following files.

1. WASM Library Modules
2. Configuration Files
3. Static data files which can be served directly by a Hermes node.
4. Static data files which can be read by any WASM Library Module within the Application.

[HDF5] is a Hierarchial file format.
An Application package consists of a number of HDF5 files in the following relationship.

![Diagram](images/application_package_hierarchy.d2)

### Application Root

Each application package is an HDF5 file that stores all the static assets, data, and WASM modules.
Each application package has associated [metadata](#application-metadata)
that is stored in the `metadata.json` file linked to the root path `/`.

### Application Metadata

Every application includes Metadata which describes:

* the application itself;
* important information about the app;
* where it comes from; and
* what resources it needs to run.

For detailed information see [Application Metadata](./app_metadata.md).

### Application Signatures

Applications MUST be signed by their Author/s.
They can also OPTIONALLY be signed by one or more publishers.
These signatures enable hermes engine to determine if an application is trusted or not.

Except for the party signing the certificate, they are otherwise identical.

For detailed information see [Hermes Application Signatures](./app_signatures.md).

### Application Static HTTP Assets

Static Assets consist of files that may be published only AS-IS via HTTP by the Application Modules.
Application Static Assets are found in the `/srv/www` path.

These Assets are OPTIONAL, however a valid Hermes application MUST consist of:

* at least one set of files at `/srv/www`; or
* at least one WASM Component Module at `/lib/<module-name>`

For detailed information see [Packaging HTTP Served Files in a Hermes Application](./app_srv_www.md).

### Application Data

This data is completely optional.

Applications can also include sharable static data.
This data is not served automatically, but can be read by any WASM module within the application.
Application Data is found in the `/src/share` path.

For detailed information see [Hermes Application Data](./app_data.md).

### Application Modules

Application Modules are found in the `/lib` path.

Individual WebAssembly Component Modules are named and stored in a subdirectory with that name.

For example: `/lib/greeter` is the path to the `greeter` module.

Modules themselves are pre-packaged and signed and are included into a Hermes Application when it is packaged.
Modules may also be sym-linked, for example the same module may be used but configured differently.
In this case, it only needs to be included once, and then named links in `/lib` can reference it.

For example if the `greeter` module can also function as a `goodbye` module, it could be linked as so:

`/lib/goodbye -> /lib/greeter`

This allows modules to be reutilized without wastefully re-including them.
The `/usr/lib` directory allows the runtime contents of a module to be altered by the application.
This allows the same code to be used but operate differently because of its configuration.

For detailed information see [Hermes WASM Component Module](./wasm_modules.md).

## Additional Technical Resources

### HDF5

* [HDF5]

### CBOR

* [CBOR]
* [Deterministic CBOR]
* [`dcbor`](https://github.com/BlockchainCommons/bc-dcbor-rust) Rust crate.
* [CDDL]

### WASI And WIT

* [WASI](https://wasi.dev/)

### COSE

* [RFC 8152](https://www.rfc-editor.org/rfc/rfc8152)
* [RFC 9052](https://www.rfc-editor.org/rfc/rfc9052)
* [RFC 9053](https://www.rfc-editor.org/rfc/rfc9053)

[CBOR]: https://cbor.io/spec.html
[CDDL]: https://datatracker.ietf.org/doc/html/rfc8610
[Deterministic CBOR]: https://www.rfc-editor.org/rfc/rfc8949.html#name-deterministically-encoded-c
[HDF5]: https://docs.hdfgroup.org/hdf5/develop/
[Linux Filesystem Hierarchy Standard V3]: https://refspecs.linuxfoundation.org/FHS_3.0/fhs-3.0.html "A Tooltip"
