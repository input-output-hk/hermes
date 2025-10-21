---
    title: 0005 HDF5 for Packaging and VFS Backing
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

Hermes applications must be distributed as immutable bundles that contain:

* WASM component modules and metadata
* Static web assets (srv/www) and shared data
* Module-local data and configuration
* Signatures for integrity and trust

The bundle should support:

* Random access without full decompression
* Strongly typed metadata and nested composition (modules within apps)
* File- and directory-like hierarchy, with the ability to embed one package inside another

Alternatives considered

* Archive formats (ZIP, TAR): monolithic, slow random access, poor native metadata, composition friction
* Static filesystems (CramFS/SquashFS/EROFS): monolithic, less common in cross‑platform Rust toolchains,
  limited metadata and composition
* OCI images: heavy for our distribution and signing needs; runtime integration complexity

## Decision

Use HDF5 (Hierarchical Data Format v5) as the application package container and as the backing
store for the Virtual Filesystem (VFS).

Properties leveraged

* Hierarchical containers with embedded files and directories
* Rich, strongly typed metadata (attributes) on any object
* Random access to contained objects without full extraction
* Ability to embed HDF5 files within HDF5 files (composition)

## Consequences

Positive

* Unified container and VFS story; efficient random access to package contents
* Clear, typed metadata for app/module manifests and signatures
* Simple mounting model for `srv`, `usr`, `lib`, `etc` into the app VFS

Trade‑offs and risks

* HDF5 is less familiar than ZIP/TAR to some developers
* Tooling for HDF5 browsing is not universal, though viewers exist
* Rust HDF5 libraries must be kept up to date and audited

## Implementation

* `hermes/bin/src/hdf5/*` implements HDF5 primitives
* `hermes/bin/src/vfs/*` provides a VFS abstraction over HDF5 with permissions
* `hermes/bin/src/packaging/*` builds and validates app and module packages, writing to HDF5

## References

* Packaging Requirements: [overview](../08_concepts/hermes_packaging_requirements/overview.md)
* VFS: [VFS](../08_concepts/vfs.md)

---
