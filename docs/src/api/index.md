---
icon: material/api
---

<!-- cspell: words RUSTDOC graphviz -->

# Hermes Rust docs

<!-- markdownlint-disable no-inline-html -->
<iframe src="rust-docs/index.html" title="RUSTDOC Documentation" style="height:800px;width:100%;"></iframe>

[OPEN FULL PAGE](./rust-docs/index.html)

## Workspace Dependency Graph

```kroki-graphviz
@from_file:./api/rust-docs/workspace.dot
```

## External Dependencies Graph

```kroki-graphviz
@from_file:./api/rust-docs/full.dot
```

## Build and Development Dependencies Graph

```kroki-graphviz
@from_file:./api/rust-docs/all.dot
```

## Module trees

### hermes crate

```rust
    {{ include_file('src/api/rust-docs/hermes.hermes.bin.modules.tree') }}
```

## Module graphs

### hermes crate

```kroki-graphviz
@from_file:./api/rust-docs/hermes.hermes.bin.modules.dot
```
