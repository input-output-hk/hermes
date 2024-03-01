# Hermes Application Data

Applications can contain read-only shared embedded data.

This data is shared between all WASM Component Modules within the application.

The data can be used for any purpose defined by the Application and is stored in `/srv/share`.

*Note: Even though the data is "shared" it is not available to any other application running inside a Hermes node.*
