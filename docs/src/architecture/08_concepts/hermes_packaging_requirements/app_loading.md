# The Application Loading Process

The high level steps of the app loading process are:

![Diagram](images/app_loading_overview.d2)

## Loading/Creating the App RW HDF5 Filesystem

All Applications will need a RW Filesystem, even if its just for Hermes to store data about the application.
The Process of loading the application is actually the process of creating or validating the RW Filesystem.

Initially the Applications RW HDF5 Filesystem looks like this:

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | Writable | Required |
| --- | ----------- | ---- | -------- | --- |
| `/`   | :octicons-file-directory-fill-16: | Root Directory | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/tmp` | :octicons-file-directory-16: | Temporary Files stored in memory | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/etc` | :octicons-file-directory-fill-16: | Writable settings | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

The Application is at this stage un-configured.
Once the user has configured the Application, the following files are created in the Application RW Storage and loading can continue.

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | Writable | Required |
| --- | ----------- | ---- | -------- | --- |
| `/etc/settings.json` | :octicons-file-16: | Hermes Engine settings for this application. | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
| `/etc/<module-name>/settings.json` | :octicons-file-16: | Module specific</br>Runtime Configurable Settings | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

If the Application has requested RW storage, then it is created (and sized accordingly) at:

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | Writable | Required |
| --- | ----------- | ---- | -------- | --- |
| `/var/` | :octicons-file-directory-fill-16: |  Contains variable data files. (Persistent) | <span style="color: green;">:octicons-check-circle-fill-12:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

## Loading the Application itself

At this stage, the RW Filesystem is now prepared.

HDF5 allows us to create symbolic links between different HDF5 files.
We use this capability to create RO symbolic links between the RW filesystem and the Application HDF5 package.

During this process, any files which are defined in `/usr/lib` which would over-ride the
contents of a module are linked inside the module, rather than the original module contents.

This allows us the re-create the view the application sees of itself, without editing the actual application at-all.

During this process symbolic RO links are created for the following files within the Application package:

<!-- markdownlint-disable max-one-sentence-per-line line-length no-inline-html -->
| Name | Type | Description | Writable | Required |
| --- | ----------- | ---- | -------- | --- |
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
| `/metadata.json` | :octicons-file-16: | Applications Metadata | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/author.cose` | :octicons-file-badge-16: | Application Author Signature | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: green;">:octicons-check-circle-fill-12:</span> |
| `/publisher.cose` | :octicons-file-badge-16: | Application Publisher Signature | <span style="color: orange;">:octicons-circle-16:</span> | <span style="color: orange;">:octicons-circle-16:</span> |
<!-- markdownlint-enable max-one-sentence-per-line line-length no-inline-html -->

## Mounting the `/srv/www` filesystem in the HTTP gateway

At this stage the Applications `/srv/www` from the RW Filesystem (as linked to the application package itself)
is registered with the HTTP gateway inside the hermes node, and it can begin serving those files.

## Loading and initialising the WASM Modules

The final step is to iterate all the WASM Modules in the RW Filesystem (as linked to the App) and load them
in canonical order into the WASM Executor.

This needs to be the final step because when a WASM Module is first loaded, it may be initialized.
That process may require access to any of the data or configuration stored within the Application.
It is only safe to access that data when the entire RW Filesystem and its cross linked
application resources are ready to be accessed.
