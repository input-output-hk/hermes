---
icon: material/folder-lock
---

# Virtual Filesystem (VFS)

Hermes provides a virtual filesystem per application backed by HDF5.
It exposes a hierarchical view with mount points corresponding to packaged content and writable areas controlled by permissions.

Structure (typical)

* `srv/www`: Static web assets served by the HTTP gateway (read-only).
* `lib/<module>/`: Module-specific data and WASM component binaries (read-only).
* `usr/` and `usr/lib/`: Application-provided shared assets (read-only).
* `etc/`: Configuration (some files may be writable under controlled policy).
* `tmp/`: Temporary storage (in-memory or ephemeral; writable).

Permissions

* Read-only for packaged assets ensures reproducibility and integrity.
* Selected write locations (e.g., `tmp`) allow modules to persist ephemeral data.

APIs

* Read/write primitives are mediated by the VFS with permission checks.
* Bootstrapping constructs the HDF5-backed structure and mounts packaged content.

References

* `hermes/bin/src/vfs/*`, `hermes/bin/src/hdf5/*`
* Packaging layout: `08_concepts/hermes_packaging_requirements/overview.md`
