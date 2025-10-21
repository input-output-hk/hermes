---
    title: 0008 COSE Signatures over CBOR for Package Integrity
    adr:
        author: Steven Johnson <steven.johnson@iohk.io>
        created: 21-Oct-2025
        status:  accepted
        extends:
            - 0001-arch-std
            - 0002-adr
    tags:
        - arc42
        - ADR
---

## Context

Hermes packages (apps and modules) must be authenticated and tamper‑evident.
We need a compact, widely supported signature format aligning with constrained environments and binary payloads.

Alternatives considered

* JOSE/JWS (JSON‑based): verbose for binary payloads, larger footprint
* Custom signature formats: harder to verify broadly and standardize

## Decision

Use COSE (CBOR Object Signing and Encryption) with EdDSA (Ed25519) for signing package author payloads.
Use X.509‑compatible certificate material for key identification (e.g., `kid` as a Blake2b hash of the signer certificate).

## Consequences

Positive

* Compact and binary‑friendly; standard and interoperable
* Clear header semantics (protected/unprotected), supports multiple object types

Trade‑offs and risks

* Certificate lifecycle must be managed (rotation, revocation)
* Tooling familiarity varies among developers

## Implementation

* `hermes/bin/src/packaging/sign/*` implements keys, certificates, and signatures
* Validation performed during package load before app execution

## References

* Concepts: [Signature Payload](../08_concepts/hermes_signing_procedure/signature_format.md#signature-payload)

---
