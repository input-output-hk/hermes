# Hermes Application Packaging Requirements

## Overview

TODO

** SJ Notes **

Each package needs metadata which can be associated with it.
So we may want to define that in its own file.
Example metadata are similar to a webasm module.
1. App Author
2. Version
3. Repo
4. License
5. Description
6. Json Schema for its configuration, so a user can configure it.
   1. This is the only config a user actually sets,  it will embed (and can have references to that config) from individual wasm modules.
   2. We need to work out how this will work and define it.
7. other stuff

We also need to sign the package, and it needs to be able to support multisig, so it can be signed by the Author and then countersigned by a publisher.

Similar to WASM modules, we will use COSE (See my notes on wasm modules).
Because we are signing the Application file, we need a way to generate a secure hash over all the data in the package, and then just sign that hash, and embed it.
We won't be signing the whole HDF5 file itself.
But because we are getting a deterministic hash from the data inside the HDF5 file for the signature, the only thing that can change is the signature itself.
This would allow an Author to sign it, pass it to a publisher who validates it, and just countersigns it.  The publisher can't alter any of files, or the Authors signature will fail.
