---
    title: 0003 Hermes runtime extensions api version control
    adr:
        author: Steven Johnson <steven.johnson@iohk.io>
        created: 16-Feb-2024
        status:  draft
---

## Context

Hermes should be a robust and reliable platform for Hermes applications to be run on.

## Assumptions

* Hermes runtime extensions api could not be stable and evolve during the development process.

## Decision

As a part of the metadate of the Hermes application, provide a `api_version` field.
During the application loading step it should be validated,
against the current Hermes `api_version` on which this application going to be executed.
Validation is a equlity check.

## Risks

* Disallows backward compatability for the older applications to be run on the latest version of Hermes.

## Consequences

* Eliminates maintaince and support complexity for the Hermes development itself.
* Eliminates the need of the internal api version managment system.
* Force Hermes application developers and it's users to use the latest version of the Hermes engine.