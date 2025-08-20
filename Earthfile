VERSION 0.8

# External CI tool imports
IMPORT github.com/input-output-hk/catalyst-ci/earthly/mdlint:v3.5.4 AS mdlint-ci
IMPORT github.com/input-output-hk/catalyst-ci/earthly/cspell:v3.5.4 AS cspell-ci
IMPORT github.com/input-output-hk/catalyst-ci/earthly/flutter:v3.5.2 AS flutter-ci

# cspell: words livedocs sitedocs

# =============================================================================
# FLUTTER FRONTEND GENERATION FOR HERMES
# =============================================================================
# Generates web frontend assets that communicate with Hermes runtime via HTTP

# Download catalyst-voices Flutter project from GitHub
catalyst-voices-source:
    FROM alpine/git
    RUN git clone https://github.com/input-output-hk/catalyst-voices /source
    SAVE ARTIFACT /source/catalyst_voices /catalyst_voices

# Set up Flutter development environment with dependencies
flutter-builder:
    DO flutter-ci+SETUP
    COPY +catalyst-voices-source/catalyst_voices .
    DO flutter-ci+BOOTSTRAP

# Generate Flutter frontend code for Hermes integration
# Creates HTTP clients, data models, and UI assets
code-generator:
    ARG save_locally=false
    FROM +flutter-builder
    
    # Generate localization files (multi-language support)
    RUN melos l10n
    
    # Generate HTTP client code and JSON serialization
    RUN melos build_runner
    
    # Generate additional repository-specific assets
    RUN melos build_runner_repository
    
    # Save generated assets to local 'frontend/' directory
    IF [ $save_locally = true ]
        RUN mkdir -p /generated/frontend
        
        # Copy all generated Dart files (*.g.dart, *.chopper.dart, *.gen.dart)
        FOR generated_file IN $(find . -name "*.g.dart" -o -name "*.chopper.dart" -o -name "*.gen.dart")
            RUN mkdir -p "/generated/frontend/$(dirname "$generated_file")" && \
                cp "$generated_file" "/generated/frontend/$generated_file"
        END
        
        # Copy localization files
        RUN find . -path "*/l10n/*.dart" -type f -exec sh -c \
            'mkdir -p "/generated/frontend/$(dirname "$1")" && cp "$1" "/generated/frontend/$1"' _ {} \;
        
        SAVE ARTIFACT /generated/frontend AS LOCAL frontend
    END

# =============================================================================
# DOCUMENTATION & QUALITY CHECKS
# =============================================================================

# Check markdown formatting
check-markdown:
    DO mdlint-ci+CHECK

# Fix markdown formatting issues locally
markdown-check-fix:
    LOCALLY
    DO mdlint-ci+MDLINT_LOCALLY --src=$(echo ${PWD}) --fix=--fix

# Clean and sort spelling dictionary
clean-spelling-list:
    FROM debian:stable-slim
    DO cspell-ci+CLEAN

# Run spell checker
check-spelling:
    DO cspell-ci+CHECK

# Package repository documentation files
repo-docs:
    FROM scratch
    WORKDIR /repo
    COPY --dir *.md LICENSE-APACHE LICENSE-MIT .
    SAVE ARTIFACT /repo repo