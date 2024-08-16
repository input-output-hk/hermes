---
    title: 0004 Hermes application loading procedure
    adr:
        author: Alex Pozhylenkov <alex.pozhylenkov@iohk.io>
        created: 11-Aug-2024
        status:  draft
---

## Context

Hermes, as an engine that runs user-developed applications, should have the functionality to load and
execute the provided assembled application.
This process must validate the application's integrity and correctness and prepare the application's state
before executing it.

## Assumptions

Hermes application package could be corrupted or modified.

## Decision

During each application loading and running process, the application package should be provided.
All possible validations and initialization should be performed based on the package data.

## Risks

* Potential bad  user experience (UX) for each application run due to time consumption in validations and
  state preparation

## Alternatives

Split loading application (installing) and running into two different procedures.
So during loading all validations will be performed along with state initialization and saved somewhere.
Run procedure will not require an original application package
and will pick up already validated and initialized data to run application.

## Consequences

* Adds better integrity and consistency over application's initial state/code among each application run.
* Eliminates possibility to execute corrupted application.
