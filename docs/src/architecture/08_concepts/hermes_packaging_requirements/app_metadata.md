# Application Metadata

Metadata for Hermes Applications is specified as a `metadata.json` file located in the root group
in the HDF5 File.
It is a `json` document and must conform to the Application metadata json schema to be valid.

## Application Metadata Contents

The Application Metadata is formally defined by [`hermes_app_metadata.schema.json`](#metadata-schema).

All Application metadata files MUST conform to that schema, or the Application will be considered invalid.

### Diagram: Hermes Application Metadata

![Diagram](images/application_metadata.d2)

*Note: Diagram is illustrative of metadata contents only.
See [`hermes_app_metadata.schema.json`](#metadata-schema) for the formal definition.*

### Application identifying Data

* Application Name : Single Line name of the application.
* Icon
* Version
* Description - Short Description of the Application
* About - Long form Description of the Application
* Copyright - Copyright Notices
* License - SPDX Strings and/or a Link to a License held within the app image.

### Application Developer/Author

* Developers Name
* Optional contact information
* Optional payment address

### Resources

Each application will need a minimum set of resources from Hermes.
The Metadata also lists the minimum viable resources required for the application.
It should also list what are the resources the Application would like to have available.
It can optionally list the maximum allocatable resources which could enable enhanced features or other functions in the application.

If a resource minimum is set as 0, then it means the resource can be denied by the user but the application can still operate.

### Permissions

When Hermes has permissioned resources, the metadata will list the permissions being requested by the application.

## Configuration

Other than resourcing and permissions, the `metadata.json` file does not contain the configuration of the application.
Application configuration is defined by the WASM Component modules.

## Metadata Schema

<!-- markdownlint-disable max-one-sentence-per-line -->

??? note "Schema: `hermes_app_metadata.schema.json`"

    ```json
    {{ include_file('includes/schemas/hermes_app_metadata.schema.json', indent=4) }}
    ```

## Metadata Example

??? note "Example: `hermes_app_metadata.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_app_metadata.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->
