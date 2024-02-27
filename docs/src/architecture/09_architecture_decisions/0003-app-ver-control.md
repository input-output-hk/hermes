---
    title: 0003 Hermes runtime extensions api version control
    adr:
        author: Alex Pozhylenkov <alex.pozhylenkov@iohk.io>
        created: 16-Feb-2024
        status:  draft
---

## Context

Hermes should be a robust and reliable platform for Hermes applications to be run on.
So it has to have a defined boundaries of which application would be executed and which not.

**NOTE**
There is an acknowledged need for version control of the APIs in a mature Hermes state, however at this stage we will not define such a policy until the core hermes engine and libraries have matured enough to make an informed choice.

## Assumptions

* Hermes runtime extensions api could not be stable and evolve during the development process.

## Decision

As a part of the metadata of the Hermes application, provide a `api_version` field.
During the application loading step it should be validated,
against the current Hermes `api_version` on which this application going to be executed.
Validation is a equality check.

## Risks

* Failing to specify a version control policy and method of enforcement at a sufficiently advanced state of maturity could make it difficult to interoperate with Hermes applications over time.

## Consequences

Consequences to consider:

* Eliminates maintenance and support complexity for the Hermes development itself.
* Eliminates the need of the internal api version management system.
* Force Hermes application developers and it's users to use the latest version of the Hermes engine.
* As `api_version` is separated from the Hermes version itself,
allows to continue deliver new versions which does not change runtime extensions api.
