---
    title: 0004 Hermes application loading procedure
    adr:
        author: Alex Pozhylenkov <alex.pozhylenkov@iohk.io>
        created: 11-Aug-2024
        status:  draft
---

## Context

Hermes as an engine which runs user's developed applications,
obviously should have a functionality to load a and run provided assembled application.
This procedure must validate application on the integrity and correctness,
prepare application's state, before actually executing application itself.

## Assumptions

Hermes application package could be corrupted or modified.

## Decision

During the each application loading and running process application package should be provide
and all possible validations and initialization should be performed based on the package data.

## Risks

* Potential bad UX user experience for each application running,
  because such validations and state preparation could take some time.

## Alternatives

Split loading application (installing) and running into two different procedures.
So during loading all validations will be performed along with state initialization and saved somewhere.
Run procedure will not require an original application package
and will pick up already validated and initialized data to run application.

## Consequences

* Adds better integrity and consistency over application's initial state/code among each application run.
* Eliminates possibility to execute corrupted application.
