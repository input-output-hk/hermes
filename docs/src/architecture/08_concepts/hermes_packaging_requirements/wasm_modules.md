# Packaging WebAssembly (WASM) modules in a Hermes Application

## Overview of a WASM Component Module

WASM Component Modules consist of:

* Metadata which describes the module.
* The compiled WASM Code itself, which MUST target the Hermes WASM Component Model API.
* An Optional configuration JSON schema.
* An Optional default configuration file.
* An Optional settings JSON schema.
* And a required author's signature.
  
![Diagram](images/wasm_component_module.d2)

## WASM Component Module Metadata

Metadata for a module must conform to the Hermes Module metadata Schema.
It holds information so that the Wasm module can be identified, including its source and license.
The metadata is purely descriptive and does not contain any information related to the configuration of the module itself.

<!-- markdownlint-disable max-one-sentence-per-line -->

### WASM Component Module Metadata - Schema

??? note "Schema: `hermes_module_metadata.schema.json`"

    ```json
    {{ include_file('includes/schemas/hermes_module_metadata.schema.json', indent=4) }}
    ```

### WASM Component Module Metadata - Example

??? note "Example: `hermes_module_metadata.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_module_metadata.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->

## WASM Component Module Configuration

Each WASM Component Module can be configured.
The purpose of this is to allow the same WASM code to have different parameterized functionality.

If one thinks of the Wasm Component Module as a kind of Class,  
the configuration allows a specific Instance of the class to be created.

Copies of the same Module can have different config,
which allows the creation of multiple Instances of the same class.

The packaging process efficiently removes redundancy in the package and will link between modules that are identical.

This configuration is defined by the WASM Component Module Author, and can be modified by the Application author.
The user has no ability to alter the configuration.

Configuration is controlled by two files.

* `config.schema.json` - This is a JSON Schema document which defines the structure expected of the configuration for the Module.
* `config.json` - The default configuration of the module.

Both must be present if configuration is defined.
It is valid for a WASM Component Module to not have any configuration,
and so there would be no configuration files present in this case.

## Wasm Component Module Settings

Settings are module configuration that are defined by the user of the application.
This is how a user of the application can configure how the application should run.
If there is user controllable configuration, then the WASM Component Module will contain a `settings.schema.json` file.
This file defines the configuration options available to the user.
The user MUST make a configuration for the application for each WASM module that requires it before the application can run.
This is simplified because the schema can contain `defaults` which will be used if the user has made no selection.
Therefore, if a WASM Module declares defaults for all options, the user need not make any changes to it.

This file is optional, and is only included in the WASM Component Module if there is actual configuration that can be changed.
Otherwise, it is not present.

## WASM Module read-only shareable data

WASM Modules may need data sets to execute their functionality efficiently.
Data which is strongly associated with a module is packaged with a module in its `share` directory.

While this data is strongly associated with a module, it may be used by any module within the application.
It is also possible for an Application to modify the shared data a WASM Module can see, without altering the signed Module itself.

There is no restriction on the kinds or amount of shared data within a module.
Nor is it required by a Module.

## WASM Component Module signatures

Individual Modules have an Author.
This allows us to compose applications by using pre-written WASM Component Modules as building blocks.
But to do so, the Author of the Module must sign it.

This allows us to validate that the Module is coming from a trusted source.

Accordingly, similar to Applications themselves, individual WASM Component Modules needs to be signed by their author.

The signature file is called `author.cose` and it is a signature across all the other files within the module.
A module is invalid if the signature does not match,
OR there are files present which are either unknown or not included in the signature.

## Packaging a WASM Component Module

Similar to an Application, Hermes WASM Component Modules are packaged and signed by the Hermes application.

Packaging a Module is controlled by a manifest file, which must conform to the Hermes WASM Component Module Manifest JSON schema.

### The WASM Component Module Packaging Process

1. Create an unsigned WASM Component Module.
2. Sign it as one or more authors.
3. *Optionally, sign it as one or more publishers.*

#### Creating the unsigned Application Package

<!-- markdownlint-disable code-block-style -->
```sh
./hermes module package <manifest.json> [<optional output path>] [--name <module name override>]
```
<!-- markdownlint-enable code-block-style -->

* `manifest.json` - Defines the location of all the src artifacts needed to build the package.
  This file must conform to the manifests [JSON schema](#wasm-component-module-manifest---schema).
  An example manifest of this [JSON schema](#wasm-component-module-manifest---schema)
  is [here](#wasm-component-module-manifest---example).
* `[<optional output path>]` - By default the module will be created in the same directory where manifest placed.
  This option allows the path of the generated module to be set, it can be absolute or relative to the manifest directory.
* `--name module name override` - The name to give the module file, instead of taking it from the manifest file.

*Note: the extension `.hmod` will automatically be added to the `module name`
to signify this is a Hermes WASM Component Module.*

#### Signing the Application Package

As the author of the Application:

<!-- markdownlint-disable code-block-style -->
```sh
./hermes module sign <X.509 Private Cert> <module_name.hmod>
```
<!-- markdownlint-enable code-block-style -->

This takes the X.509 Private Certificate presented, and signs or counter-signs the Application package.

*Note: A Hermes WASM Component Module is INVALID if it does not contain at least 1 Author Signature.*

#### Inspecting a Hermes Application

<!-- markdownlint-disable code-block-style -->
```sh
./hermes package inspect <app_package_name>
```
<!-- markdownlint-enable code-block-style -->

This command will dump the logical contents of the WASM Component Module and if it is considered valid or not.
It does not extract files from the module.  
If files need to be extracted or individually accessed outside of Hermes, any [HDF5 Viewer] can be used.
As the module is compressed, part of the information that is displayed should be the total module size on-disk,
and the true size of the uncompressed data it contains.
The compressed/uncompressed statistic should be per file, and also for the total module.

<!-- markdownlint-disable max-one-sentence-per-line -->

### WASM Component Module Manifest - Schema

??? note "Schema: `hermes_module_manifest.schema.json`"

    ```json
    {{ include_file('includes/schemas/hermes_module_manifest.schema.json', indent=4) }}
    ```

### WASM Component Module Manifest - Example

??? note "Example: `hermes_module_manifest.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_module_manifest.json', indent=4) }}
    ```

### WASM Component Module Manifest - Minimal Example

??? note "Example: MINIMAL `hermes_module_minimal_manifest.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_module_minimal_manifest.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->
