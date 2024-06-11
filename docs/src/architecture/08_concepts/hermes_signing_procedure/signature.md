# Signature format

Hermes signatures, whether for application package or WASM module package, adhere to a specific format.
This format is based on the [COSE] standard, which is a [CBOR]-defined object.

The [COSE] format is a widely adopted standard for secure communication and data exchange.
It provides a flexible and compact way to represent cryptographic objects,
such as digital signatures.
By using [COSE], Hermes ensures the integrity and authenticity of its signatures,
ensuring the security and trustworthiness of the signed content.

## [COSE] Headers

The [COSE] standard defines two types of headers: `protected` and `unprotected`.

* The `protected` headers contain parameters about the current layer
  that are to be cryptographically protected.
  These headers are mandatory for certain cryptographic computations and must be included in the signature.
  If no cryptographic computation is required, the `protected` headers can be empty.

* The `unprotected` headers contain parameters about the current layer
  that are not cryptographically protected.
  These headers are optional and can be used to provide additional information or metadata.

In Hermes, the following headers with its values **MUST** be included in the [COSE] signature.

`protected`:

* `alg`: `EdDSA`
  (This parameter is used to indicate the algorithm used for the security processing).
* `content type`: `application/json`
  (This parameter is used to indicate the content type of the data in the payload or ciphertext fields).

## Signature type

[COSE] is a flexible security protocol that supports various types of security messages.

The comprehensive list includes:

* COSE Signed Data Object
* COSE Single Signer Data Object
* COSE Encrypted Data Object
* COSE Single Recipient Encrypted Data Object
* COSE MACed Data Object
* COSE Mac w/o Recipients Object

However, Hermes will utilize only `COSE Signed Data Object` to enable signing with multiple users.
So every [COSE] signature **MUST** be encoded as `COSE Signed Data Object` even if it contains a one signature in it.

## Signature payload

As mentioned earlier, the content type of the [COSE] signature payload is JSON.
Therefore, the payload must conform to the following schema:

<!-- markdownlint-disable max-one-sentence-per-line -->
??? note "Schema: `hermes_module_cose_payload.schema.json`"

    ```json
        {{ include_file('includes/schemas/hermes_module_cose_payload.schema.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->

### Example

<!-- markdownlint-disable max-one-sentence-per-line -->
??? note "Example: `hermes_module_cose_payload.json`"

    ```json
        {{ include_file('includes/schemas/example/hermes_module_cose_payload.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->

## Signature headers

`COSE Signed Data Object` signature type defined as a [CBOR] structure,
which includes the same meaning headers.

In Hermes, the following headers with its values **MUST** be included in the [COSE] signature.

`protected`:

* `kid`: a Blake2B hash of the signer's [x.509] certificate acociated with it's keys
  (This parameter identifies one piece of data
  that can be used as input to find the needed cryptographic key.).

[COSE]: https://datatracker.ietf.org/doc/html/rfc8152
[CBOR]: https://datatracker.ietf.org/doc/html/rfc8949
[x.509]: https://en.wikipedia.org/wiki/X.509
