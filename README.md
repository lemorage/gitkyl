# Gitkyl

Gitkyl **[GIT-kul]** is a static site generator for git repositories. Pure Rust.

## Install

```bash
cargo install --git https://github.com/lemorage/gitkyl
```

## Use

```bash
gitkyl                    # current repo → ./dist
gitkyl /path/to/repo      # specific repo
gitkyl . -o site          # custom output
```

## Build

```bash
cargo build --release     # target/release/gitkyl
cargo test                # verify
```

## What's Next

- File browsing
- Syntax highlighting
- LSP navigation
- Semantic search

---

BSD-3-Clause · lemorage
