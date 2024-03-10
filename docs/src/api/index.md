---
icon: material/api
---

<!-- cspell: words RUSTDOC graphviz -->

# Hermes Rust docs

<!-- markdownlint-disable no-inline-html -->
<iframe src="rust-docs/index.html" title="RUSTDOC Documentation" style="height:800px;width:100%;"></iframe>

[OPEN FULL PAGE](./rust-docs/index.html)

## Workspace Dependency Graph

```graphviz dot workspace_deps.png
{{ include_file('src/api/rust-docs/workspace.dot') }}
```

## External Dependencies Graph

```graphviz dot full_deps.png
{{ include_file('src/api/rust-docs/full.dot') }}
```

## Build and Development Dependencies Graph

```graphviz dot all_deps.png
{{ include_file('src/api/rust-docs/all.dot') }}
```

## Module trees

### hermes crate

```rust
{{ include_file('src/api/rust-docs/hermes.hermes.bin.modules.tree') }}
```

### cardano-chain-follower crate

```rust
{{ include_file('src/api/rust-docs/cardano-chain-follower.lib.modules.tree') }}
```

## Module graphs

### hermes crate

```graphviz dot hermes_modules.png
{{ include_file('src/api/rust-docs/hermes.hermes.bin.modules.dot') }}
```

### cardano-chain-follower crate

```graphviz dot chain_follower_modules.png
{{ include_file('src/api/rust-docs/cardano-chain-follower.lib.modules.dot') }}
```
