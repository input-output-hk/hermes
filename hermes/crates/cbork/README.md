<!-- cspell: words cbork CBOR -->

# cbork

CBOR Kit

We need to support the parsing of CDDL in the following priority sequence.
Each needs to be complete before extending with the subsequent specification extension.
We do not need to handle choosing which extensions are enabled.

1. CDDL Spec: <https://www.rfc-editor.org/rfc/rfc8610>
2. Errata to include: <https://www.ietf.org/archive/id/draft-ietf-cbor-update-8610-grammar-01.html>
3. Extensions: <https://www.rfc-editor.org/rfc/rfc9165>
4. Modules: <https://cbor-wg.github.io/cddl-modules/draft-ietf-cbor-cddl-modules.html> and <https://github.com/cabo/cddlc>

There are semantic rules about well formed CDDL files that are not enforced by the grammar.
The full parser will also need to validate those rules.
The primary rule is that the very first definition in the file is the base type.

We should also be able to detect if there are orphaned definitions in the CDDL file.

There may be other checks we need to perform on the parsed AST for validity.
