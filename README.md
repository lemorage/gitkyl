# Gitkyl

Gitkyl **[GIT-kul]** is a static site generator for git repositories. Pure Rust.

## Install

```bash
cargo install --git https://github.com/lemorage/gitkyl
```

## Use

```bash
gitkyl                                         # current repo → ./dist
gitkyl /path/to/repo                           # specific repo
gitkyl . -o site                               # custom output
gitkyl --name "My Project" --owner "username"  # custom metadata
gitkyl --theme Catppuccin-Mocha                # dark theme
gitkyl --theme base16-ocean.light              # built-in theme
```

### Theme Options

**Included themes:**
- `Catppuccin-Latte` (default) - Modern warm light theme
- `Catppuccin-Mocha` - Modern dark theme
- `InspiredGitHub` - GitHub-style light theme
- `base16-ocean.light`, `base16-ocean.dark` - Cool modern themes
- `Solarized (light)`, `Solarized (dark)` - Eye-strain optimized

**Custom themes:**
```bash
gitkyl --theme path/to/custom.tmTheme   # External .tmTheme file
```

## Output Structure

```
dist/
├── index.html                    # Root with top-level files/folders
├── tree/master/src.html          # Directory listings
├── blob/master/main.rs.html      # Individual files
└── commits/master/index.html     # Commit history
```

## Build

```bash
cargo build --release     # target/release/gitkyl
cargo test                # run all tests
```

---

BSD-3-Clause · lemorage
