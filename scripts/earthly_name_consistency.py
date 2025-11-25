#!/usr/bin/env python3
"""
Guardrail script that keeps Cargo-produced WASM modules aligned with the Earthly
artifact names we publish from each module directory.

It crawls every `Cargo.toml`, looks for `cdylib` targets (the only ones that
output `.wasm`), extracts the declared library name, and compares it with the
first Earthly `--out=...` or `SAVE ARTIFACT ...` directive found nearby.

Any mismatch causes a non-zero exit code so CI/just/earthly targets can enforce
the contract described in the `Cargo.toml` comments.
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Regex captures artifact names from statements like `--out=foo.wasm` or
# `SAVE ARTIFACT ... foo.wasm`.
EARTHLY_RE = re.compile(r"(?:--out=|SAVE ARTIFACT )([A-Za-z0-9_-]+)\.wasm")


def earthly_name(dir):
    """Extract the WASM artifact name from the module's Earthfile."""
    ef = dir / "Earthfile"
    if not ef.exists():
        return None
    match = EARTHLY_RE.search(ef.read_text())
    return match.group(1) if match else None


def cargo_name(path):
    """Return the cdylib name declared in Cargo.toml, if any."""
    data = tomllib.loads(path.read_text())
    lib = data.get("lib", {})
    # Only modules compiled as `cdylib` produce WASM artifacts we care about.
    if "cdylib" not in lib.get("crate-type", []):
        return None
    return lib.get("name") or data["package"]["name"]


def normalize(name: str | None):
    if not name:
        return None
    # Earthly (snake_case) and Cargo (kebab-case) often differ only by
    # punctuation; normalize both sides before comparing.
    return name.replace("-", "_").lower()


errors = []
for cargo in ROOT.rglob("Cargo.toml"):
    name = cargo_name(cargo)
    if not name:
        continue
    e_name = earthly_name(cargo.parent)
    # Only flag an error when both sources provide a name and the normalized
    # identifiers still differ. (Missing Earthfile entries are ignored so the
    # script can be run in repos that mix Rust and non-Rust modules.)
    if e_name and normalize(e_name) != normalize(name):
        errors.append(f"{cargo.parent}: Cargo '{name}' vs Earthly '{e_name}'")

if errors:
    print("\n".join(errors))
    sys.exit(1)
