VERSION 0.8

IMPORT github.com/input-output-hk/catalyst-ci/earthly/mdlint:v3.6.12 AS mdlint-ci
IMPORT github.com/input-output-hk/catalyst-ci/earthly/cspell:v3.6.12 AS cspell-ci


# cspell: words livedocs sitedocs

# check-markdown : markdown check using catalyst-ci.
check-markdown:
    DO mdlint-ci+CHECK

# markdown-check-fix : markdown check and fix using catalyst-ci.
markdown-check-fix:
    LOCALLY

    DO mdlint-ci+MDLINT_LOCALLY --src=$(echo ${PWD}) --fix=--fix

# clean-spelling-list : Make sure the project dictionary is properly sorted.
clean-spelling-list:
    FROM debian:stable-slim
    DO cspell-ci+CLEAN

# check-spelling : Check spelling in this repo inside a container.
check-spelling:
    DO cspell-ci+CHECK

# check-earthly-names : ensure Cargo/WASM names match Earthly outputs.
check-earthly-names:
    FROM python:3.12-slim

    WORKDIR /work
    COPY . .
    RUN python3 scripts/earthly_name_consistency.py

# repo-docs : target to store the documentation from the root of the repo.
repo-docs:
    # Create artifacts of extra files we embed inside the documentation when its built.
    FROM scratch

    WORKDIR /repo
    COPY --dir *.md LICENSE-APACHE LICENSE-MIT .

    SAVE ARTIFACT /repo repo
