# Hermes Application Signatures

Signatures in Hermes Applications are created by Authors of the application.

If there is an independent Publisher/s of the application they too can attach a signature to the application.

This method protects any Application from being tampered with once released by the Author,
and also allows it to be safely co-signed by a Publisher.

## Author signature payload

Application package author signature payload according to the signing
[spec](../hermes_signing_procedure/signature_format.md#signature-payload)
should follow this schema:

<!-- markdownlint-disable max-one-sentence-per-line -->
??? note "Schema: `hermes_module_cose_author_payload.schema.json`"

    ```json
    {{ include_file('includes/schemas/hermes_module_cose_author_payload.schema.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->

Application package author signature payload example:

<!-- markdownlint-disable max-one-sentence-per-line -->
??? note "Example: `hermes_module_cose_author_payload.json`"

    ```json
    {{ include_file('includes/schemas/example/hermes_module_cose_author_payload.json', indent=4) }}
    ```
<!-- markdownlint-enable max-one-sentence-per-line -->
