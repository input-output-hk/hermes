---
icon: material/folder-lock
---

# Virtual Filesystem (VFS)

Hermes provides a virtual filesystem per application backed by HDF5.
It exposes a hierarchical view with mount points corresponding to packaged content and writable areas controlled by permissions.

Structure (runtime view)

* `www/`: Static web assets from the package `/srv/www` (read-only).
* `share/`: App-wide shared assets from the package `/srv/share` (read-only).
* `lib/<module>/`: Module metadata, WASM components, and bundled data (read-only).
* `usr/` and `usr/lib/`: Application overrides for module data/config (read-only).
* `etc/`: Writable settings area (currently empty by default).
* `tmp/`: Writable scratch space stored inside the per-app `.hfs` file.
* `ipfs/`: Read-only IPFS-backed reads (used by IPFS helpers).
* `srv/`: Present in the VFS but package `srv/*` content is mounted into `/www` and `/share`.

Permissions

* Read-only for packaged assets ensures reproducibility and integrity.
* Writable locations are limited to `tmp/` and `etc/`.

Persistence

* The VFS is stored as `~/.hermes/<app>.hfs` and persists across runs unless cleaned.

APIs

* Read/write primitives are mediated by the VFS with permission checks.
* Bootstrapping constructs the HDF5-backed structure and mounts packaged content.

References

* `hermes/bin/src/vfs/*`, `hermes/bin/src/hdf5/*`
* Packaging layout: [Packaging Requirements](./hermes_packaging_requirements/overview.md)
