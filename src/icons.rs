// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026 lemorage
//
// File icons: Catppuccin Zed Icons (MIT License)
// https://github.com/catppuccin/zed-icons
//
// UI icons: Custom SVGs matching Catppuccin style

//! Inline SVG icon rendering
//!
//! Provides file type icons and UI icons embedded at compile time.
//! File icons sourced from Catppuccin Zed Icons (MIT licensed).
//! UI icons are custom SVGs matching the Catppuccin aesthetic.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/icons.rs"));
}

use maud::{Markup, PreEscaped, html};
use std::path::Path;

pub use generated::ICON_COUNT;

/// Renders file icon as inline SVG
///
/// # Arguments
///
/// * `path`: File path relative to repository root
///
/// # Returns
///
/// Icon markup with inline SVG wrapped in icon container
pub fn file_icon(path: &str) -> Markup {
    let svg = get_icon_svg(path);
    html! {
        span class="file-icon" {
            (PreEscaped(svg))
        }
    }
}

/// Renders parent directory icon (arrow up)
pub fn parent_dir_icon() -> Markup {
    html! {
        span class="file-icon" {
            (PreEscaped(generated::ARROW_UP))
        }
    }
}

// UI Icons (inline SVG rendering)

/// Copy to clipboard icon
pub fn copy_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_COPY)) } }
}

/// Check mark icon (copy success)
pub fn check_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_CHECK)) } }
}

/// Git tag icon
pub fn tag_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_TAG)) } }
}

/// Git branch icon
pub fn branch_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_BRANCH)) } }
}

/// Git commit icon
pub fn commit_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_COMMIT)) } }
}

/// Clock/history icon
pub fn clock_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_CLOCK)) } }
}

/// Arrow right icon
pub fn arrow_right_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_ARROW_RIGHT)) } }
}

/// Caret down icon (dropdown)
pub fn caret_down_icon() -> Markup {
    html! { span class="ui-icon caret" { (PreEscaped(generated::UI_CARET_DOWN)) } }
}

/// Eye icon (preview mode)
pub fn eye_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_EYE)) } }
}

/// Code icon
pub fn code_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_CODE)) } }
}

/// File with X icon (binary/unsupported)
pub fn file_x_icon() -> Markup {
    html! { span class="ui-icon" { (PreEscaped(generated::UI_FILE_X)) } }
}

/// Returns raw SVG string for copy icon (for JS manipulation)
pub fn copy_icon_svg() -> &'static str {
    generated::UI_COPY
}

/// Returns raw SVG string for check icon (for JS manipulation)
pub fn check_icon_svg() -> &'static str {
    generated::UI_CHECK
}

/// Returns SVG string for file path
///
/// Maps file extensions and names to appropriate icons. Falls back to generic
/// file icon for unrecognized types.
pub fn get_icon_svg(path: &str) -> &'static str {
    // Handle directories
    if path.ends_with('/') {
        return get_folder_icon(path);
    }

    let path_lower = path.to_lowercase();
    let file_name = Path::new(&path_lower)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Check special filenames first
    if let Some(icon) = match_filename(file_name) {
        return icon;
    }

    // Check extension
    if let Some(ext) = Path::new(&path_lower).extension().and_then(|e| e.to_str())
        && let Some(icon) = match_extension(ext)
    {
        return icon;
    }

    // Default file icon
    generated::_FILE
}

/// Maps folder names to specific folder icons
fn get_folder_icon(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();
    let name = path_lower
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");

    match name {
        // Core structure
        "src" | "source" => generated::FOLDER_SRC,
        "lib" | "libs" | "library" => generated::FOLDER_LIB,
        "dist" | "build" | "out" | "output" => generated::FOLDER_DIST,
        "public" | "static" => generated::FOLDER_PUBLIC,
        "priv" | "private" => generated::FOLDER_PRIVATE,

        // Git/VCS
        ".git" => generated::FOLDER_GIT,
        ".github" => generated::FOLDER_GITHUB,
        ".gitlab" => generated::FOLDER_GITLAB,

        // Documentation
        "docs" | "doc" | "documentation" => generated::FOLDER_DOCS,
        "examples" | "example" => generated::FOLDER_EXAMPLES,

        // Testing
        "test" | "tests" | "__tests__" | "spec" | "specs" => generated::FOLDER_TESTS,
        "mocks" | "__mocks__" | "mock" => generated::FOLDER_MOCKS,

        // Web/Frontend
        "components" | "component" => generated::FOLDER_COMPONENTS,
        "layouts" | "layout" => generated::FOLDER_LAYOUTS,
        "views" | "view" | "pages" | "page" => generated::FOLDER_VIEWS,
        "templates" | "template" => generated::FOLDER_TEMPLATES,
        "styles" | "style" | "css" => generated::FOLDER_STYLES,
        "themes" | "theme" => generated::FOLDER_THEMES,
        "assets" => generated::FOLDER_ASSETS,
        "images" | "img" | "image" => generated::FOLDER_IMAGES,
        "fonts" | "font" => generated::FOLDER_FONTS,
        "js" | "javascript" => generated::FOLDER_JAVASCRIPT,

        // Backend
        "api" | "apis" => generated::FOLDER_API,
        "routes" | "route" | "router" | "routing" => generated::FOLDER_ROUTES,
        "controllers" | "controller" => generated::FOLDER_CONTROLLERS,
        "middleware" | "middlewares" => generated::FOLDER_MIDDLEWARE,
        "server" | "servers" => generated::FOLDER_SERVER,
        "client" | "clients" => generated::FOLDER_CLIENT,
        "functions" | "function" | "fn" => generated::FOLDER_FUNCTIONS,

        // Data
        "database" | "db" | "databases" => generated::FOLDER_DATABASE,
        "graphql" | "gql" => generated::FOLDER_GRAPHQL,
        "prisma" => generated::FOLDER_PRISMA,

        // Config
        "config" | "configs" | "configuration" => generated::FOLDER_CONFIG,
        ".vscode" => generated::FOLDER_VSCODE,
        ".idea" => generated::FOLDER_INTELLIJ,

        // Utils/Shared
        "utils" | "util" | "utilities" | "helpers" | "helper" => generated::FOLDER_UTILS,
        "shared" | "common" => generated::FOLDER_SHARED,
        "core" => generated::FOLDER_CORE,
        "types" | "type" | "typings" => generated::FOLDER_TYPES,
        "hooks" | "hook" => generated::FOLDER_HOOKS,
        "plugins" | "plugin" => generated::FOLDER_PLUGINS,

        // CI/CD & DevOps
        "docker" => generated::FOLDER_DOCKER,
        ".devcontainer" | "devcontainer" => generated::FOLDER_DEVCONTAINER,
        ".husky" | "husky" => generated::FOLDER_HUSKY,

        // Build tools
        "node_modules" => generated::FOLDER_NODE,
        ".cargo" | "target" => generated::FOLDER_CARGO,
        ".turbo" | "turbo" => generated::FOLDER_TURBO,
        ".yarn" | "yarn" => generated::FOLDER_YARN,

        // Frameworks
        ".next" | "next" => generated::FOLDER_NEXT,
        ".nuxt" | "nuxt" => generated::FOLDER_NUXT,
        ".tauri" | "tauri" | "src-tauri" => generated::FOLDER_TAURI,
        ".xcode" | "xcode" => generated::FOLDER_XCODE,

        // Language specific
        "include" | "includes" => generated::FOLDER_INCLUDE,
        "locales" | "locale" | "i18n" | "translations" => generated::FOLDER_LOCALES,
        "packages" | "package" => generated::FOLDER_PACKAGES,
        "scripts" | "script" => generated::FOLDER_SCRIPTS,
        "workflows" | ".github/workflows" => generated::FOLDER_WORKFLOWS,
        "security" => generated::FOLDER_SECURITY,
        "temp" | "tmp" | ".temp" | ".tmp" => generated::FOLDER_TEMP,
        "vercel" | ".vercel" => generated::FOLDER_VERCEL,
        "nix" | ".nix" => generated::FOLDER_NIX,
        ".pre-commit" | "pre-commit" => generated::FOLDER_PRE_COMMIT,
        "xmake" | ".xmake" => generated::FOLDER_XMAKE,

        _ => generated::_FOLDER,
    }
}

/// Maps special filenames to icons
fn match_filename(name: &str) -> Option<&'static str> {
    Some(match name {
        // TypeScript definition files
        n if n.ends_with(".d.ts") => generated::TYPESCRIPT_DEF,

        // README/Docs
        n if n.starts_with("readme") => generated::README,
        n if n.starts_with("license") || n.starts_with("licence") => generated::LICENSE,
        n if n.starts_with("changelog") => generated::CHANGELOG,
        n if n.starts_with("contributing") => generated::CONTRIBUTING,
        "code_of_conduct.md" | "code-of-conduct.md" => generated::CODE_OF_CONDUCT,
        "codeowners" => generated::CODEOWNERS,
        "security.md" | "security.txt" => generated::SECURITY,
        "todo.md" | "todo.txt" | "todo" => generated::TODO,

        // Git
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".gitkeep" => generated::GIT,
        "cliff.toml" | ".cliff.toml" => generated::GIT_CLIFF,

        // Docker
        "dockerfile" | ".dockerfile" => generated::DOCKER,
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
            generated::DOCKER_COMPOSE
        }
        ".dockerignore" => generated::DOCKER_IGNORE,

        // Cargo/Rust
        "cargo.toml" => generated::CARGO,
        "cargo.lock" => generated::CARGO_LOCK,
        "rustfmt.toml" | ".rustfmt.toml" | "clippy.toml" | ".clippy.toml" => generated::RUST_CONFIG,

        // JavaScript/TypeScript/Node
        "package.json" => generated::PACKAGE_JSON,
        "package-lock.json" | "npm-shrinkwrap.json" => generated::NPM_LOCK,
        ".npmrc" | ".npmignore" => generated::NPM,
        "yarn.lock" => generated::YARN_LOCK,
        ".yarnrc" | ".yarnrc.yml" => generated::YARN,
        "pnpm-lock.yaml" | ".pnpmfile.cjs" => generated::PNPM_LOCK,
        ".pnpmrc" | "pnpm-workspace.yaml" => generated::PNPM,
        "bun.lockb" | "bun.lock" => generated::BUN_LOCK,
        "bunfig.toml" => generated::BUN,
        "deno.json" | "deno.jsonc" => generated::DENO,
        "deno.lock" => generated::DENO_LOCK,
        "tsconfig.json" | "jsconfig.json" => generated::TYPESCRIPT_CONFIG,
        ".prettierrc"
        | ".prettierrc.json"
        | ".prettierrc.yaml"
        | ".prettierrc.yml"
        | "prettier.config.js"
        | "prettier.config.mjs"
        | "prettier.config.ts" => generated::PRETTIER,
        ".prettierignore" => generated::PRETTIER_IGNORE,
        ".eslintrc" | ".eslintrc.json" | ".eslintrc.yaml" | ".eslintrc.yml" | ".eslintrc.js"
        | ".eslintrc.cjs" | "eslint.config.js" | "eslint.config.mjs" | "eslint.config.ts" => {
            generated::ESLINT
        }
        ".eslintignore" => generated::ESLINT_IGNORE,
        "biome.json" | "biome.jsonc" => generated::BIOME,
        ".babelrc" | ".babelrc.json" | "babel.config.js" | "babel.config.json" => generated::BABEL,
        "webpack.config.js" | "webpack.config.ts" | "webpack.config.mjs" => generated::WEBPACK,
        "vite.config.js" | "vite.config.ts" | "vite.config.mjs" => generated::VITE,
        "vitest.config.js" | "vitest.config.ts" => generated::VITEST,
        "rollup.config.js" | "rollup.config.ts" | "rollup.config.mjs" => generated::ROLLUP,
        "esbuild.config.js" | "esbuild.config.mjs" => generated::ESBUILD,
        "jest.config.js" | "jest.config.ts" | "jest.config.json" => generated::JEST,
        "playwright.config.js" | "playwright.config.ts" => generated::PLAYWRIGHT,
        "cypress.config.js" | "cypress.config.ts" => generated::CYPRESS,
        "tailwind.config.js" | "tailwind.config.ts" | "tailwind.config.mjs" => generated::TAILWIND,
        "postcss.config.js" | "postcss.config.mjs" | "postcss.config.ts" => generated::POSTCSS,
        ".stylelintrc" | ".stylelintrc.json" | "stylelint.config.js" => generated::STYLELINT,
        ".browserslistrc" | "browserslist" => generated::BROWSERSLIST,
        "nodemon.json" => generated::NODEMON,
        ".nvmrc" | ".node-version" => generated::JAVASCRIPT,
        "turbo.json" => generated::TURBO,
        "nx.json" => generated::NX,
        ".lintstagedrc" | "lint-staged.config.js" => generated::LINT_STAGED,
        "commitlint.config.js" | ".commitlintrc" | ".commitlintrc.json" => generated::COMMITLINT,
        ".huskyrc" | ".huskyrc.json" => generated::HUSKY,
        ".release-it.json" | "release-it.json" => generated::SEMANTIC_RELEASE,

        // Python
        "requirements.txt" | "requirements.in" | "pyproject.toml" | "setup.py" | "setup.cfg"
        | "pipfile" => generated::PYTHON_CONFIG,
        "pipfile.lock" | "poetry.lock" => generated::POETRY_LOCK,
        "uv.lock" => generated::UV,
        "ruff.toml" | ".ruff.toml" => generated::RUFF,
        ".python-version" => generated::PYTHON,

        // Ruby
        "gemfile" => generated::RUBY_GEM,
        "gemfile.lock" => generated::RUBY_GEM_LOCK,
        "rakefile" | ".ruby-version" | ".ruby-gemset" => generated::RUBY,

        // Go
        "go.mod" => generated::GO_MOD,
        "go.sum" | "go.work" => generated::GO,

        // Java/JVM
        "pom.xml" | "build.gradle" | "build.gradle.kts" | "settings.gradle" | "gradlew"
        | "gradlew.bat" => generated::GRADLE,

        // PHP
        "composer.json" | "composer.lock" => generated::PHP,

        // .NET
        "*.csproj" | "*.sln" | "nuget.config" => generated::CSHARP,

        // Build tools
        "makefile" | "gnumakefile" | "makefile.am" => generated::MAKEFILE,
        "cmakelists.txt" | "cmake.toml" => generated::CMAKE,
        "justfile" | ".justfile" => generated::JUST,
        "taskfile.yml" | "taskfile.yaml" => generated::TASKFILE,
        "meson.build" | "meson_options.txt" => generated::MESON,
        "build.ninja" => generated::NINJA,
        "xmake.lua" => generated::XMAKE,
        "premake5.lua" | "premake4.lua" => generated::PREMAKE,
        "bazel" | "build.bazel" | "workspace" | "workspace.bazel" => generated::BAZEL,

        // Editor/IDE configs
        ".editorconfig" => generated::EDITORCONFIG,
        ".vimrc" | "_vimrc" | ".gvimrc" => generated::VIM,
        ".vscodeignore" => generated::VSCODE_IGNORE,
        "settings.json" | "launch.json" | "tasks.json" | "extensions.json" => generated::VSCODE,
        "*.sublime-project" | "*.sublime-workspace" => generated::SUBLIME,

        // Environment
        ".env" | ".env.local" | ".env.development" | ".env.production" | ".env.example"
        | ".env.test" | ".env.staging" => generated::ENV,
        ".envrc" => generated::ENVRC,

        // CI/CD
        "circle.yml" | ".circleci/config.yml" => generated::CIRCLE_CI,
        ".gitlab-ci.yml" => generated::GITLAB,
        "netlify.toml" => generated::NETLIFY,
        "vercel.json" | "now.json" => generated::VERCEL,
        "heroku.yml" | "procfile" => generated::HEROKU,
        "firebase.json" | ".firebaserc" => generated::FIREBASE,
        "wrangler.toml" | "wrangler.json" => generated::WRANGLER,
        "serverless.yml" | "serverless.yaml" | "serverless.json" => generated::SERVERLESS,
        ".travis.yml" | "appveyor.yml" | "jenkinsfile" | "fly.toml" | "render.yaml"
        | "railway.json" | "railway.toml" | "pulumi.yaml" | "pulumi.yml" => generated::CONFIG,

        // Container/K8s
        "kubernetes.yaml" | "kubernetes.yml" | "k8s.yaml" | "k8s.yml" => generated::HELM,
        "helm.yaml" | "chart.yaml" | "values.yaml" => generated::HELM,
        "skaffold.yaml" => generated::DOCKER,
        "terraform.tfvars" => generated::TERRAFORM,

        // Frameworks
        "next.config.js" | "next.config.mjs" | "next.config.ts" => generated::NEXT,
        "nuxt.config.js" | "nuxt.config.ts" => generated::NUXT,
        "svelte.config.js" | "svelte.config.ts" => generated::SVELTE_CONFIG,
        "astro.config.js" | "astro.config.mjs" | "astro.config.ts" => generated::ASTRO_CONFIG,
        "remix.config.js" | "remix.config.ts" => generated::REMIX,
        "gatsby-config.js" | "gatsby-config.ts" => generated::GATSBY,
        "vue.config.js" => generated::VUE_CONFIG,
        "angular.json" => generated::ANGULAR,
        "tauri.conf.json" => generated::TAURI,
        "electron-builder.yml" | "electron-builder.json" => generated::JAVASCRIPT,

        // Linting/Formatting
        ".cspell.json" | "cspell.json" => generated::CSPELL,
        ".semgrep.yml" | "semgrep.yml" => generated::SEMGREP,

        // Database/ORM
        "prisma.schema" | "schema.prisma" => generated::PRISMA,
        "drizzle.config.ts" | "drizzle.config.js" => generated::DRIZZLE_ORM,

        // Misc configs
        "devcontainer.json" | ".devcontainer.json" => generated::DEVCONTAINER,
        "dependabot.yml" | "dependabot.yaml" => generated::DEPENDABOT,
        "renovate.json" | ".renovaterc" | ".renovaterc.json" => generated::RENOVATE,
        ".pre-commit-config.yaml" => generated::PRE_COMMIT,
        "codecov.yml" | ".codecov.yml" => generated::CONFIG,
        "hugo.toml" | "hugo.yaml" | "hugo.json" => generated::HUGO,
        ".storybook" => generated::STORYBOOK,
        ".nxignore" => generated::NX_IGNORE,
        "nginx.conf" => generated::NGINX,
        ".cursorignore" | ".cursorrules" => generated::CURSOR_IGNORE,
        "cursor.json" | ".cursor" => generated::CURSOR,
        "favicon.ico" | "favicon.svg" | "favicon.png" => generated::FAVICON,

        // Xcode
        "*.xcodeproj" | "*.xcworkspace" | "project.pbxproj" => generated::XCODE,

        // Nix
        "flake.nix" | "default.nix" | "shell.nix" => generated::NIX,
        "flake.lock" => generated::NIX_LOCK,

        _ => return None,
    })
}

/// Maps file extensions to icons
fn match_extension(ext: &str) -> Option<&'static str> {
    Some(match ext {
        // Rust
        "rs" => generated::RUST,

        // C/C++
        "c" => generated::C,
        "h" => generated::C_HEADER,
        "cpp" | "cc" | "cxx" | "c++" => generated::CPP,
        "hpp" | "hh" | "hxx" | "h++" => generated::CPP_HEADER,

        // C#/.NET
        "cs" | "razor" | "cshtml" => generated::CSHARP,
        "fs" | "fsx" | "fsi" => generated::FSHARP,
        "xaml" => generated::XAML,

        // Web
        "html" | "htm" | "xhtml" => generated::HTML,
        "css" => generated::CSS,
        "scss" | "sass" => generated::SASS,
        "less" => generated::LESS,
        "vue" => generated::VUE,
        "svelte" => generated::SVELTE,
        "astro" => generated::ASTRO,

        // JavaScript/TypeScript
        "js" | "mjs" | "cjs" => generated::JAVASCRIPT,
        "jsx" => generated::JAVASCRIPT_REACT,
        "ts" | "mts" | "cts" => generated::TYPESCRIPT,
        "tsx" => generated::TYPESCRIPT_REACT,

        // Templating
        "ejs" => generated::EJS,
        "hbs" | "handlebars" => generated::HANDLEBARS,
        "twig" => generated::TWIG,
        "mustache" | "slim" | "eta" => generated::HTML,
        "jinja" | "jinja2" | "j2" => generated::JINJA,

        // Data formats
        "json" | "jsonc" | "json5" => generated::JSON,
        "yaml" | "yml" => generated::YAML,
        "toml" => generated::TOML,
        "xml" | "xsl" | "xslt" => generated::XML,
        "csv" | "tsv" => generated::CSV,
        "properties" => generated::PROPERTIES,
        "kdl" => generated::KDL,
        "dhall" => generated::DHALL,

        // Markdown/Documentation
        "md" | "markdown" => generated::MARKDOWN,
        "mdx" => generated::MARKDOWN_MDX,
        "rst" | "rest" => generated::TEXT,
        "org" => generated::ORG,
        "tex" | "latex" | "sty" | "cls" => generated::LATEX,
        "typ" => generated::TYPST,

        // Python
        "py" | "pyi" | "pyw" => generated::PYTHON,
        "pyc" | "pyo" | "pyd" => generated::PYTHON_COMPILED,
        "ipynb" => generated::JUPYTER,

        // Ruby
        "rb" | "rake" | "gemspec" | "erb" | "rhtml" => generated::RUBY,

        // Go
        "go" => generated::GO,
        "tmpl" => generated::GO_TEMPLATE,

        // Java/JVM
        "java" => generated::JAVA,
        "jar" | "war" | "ear" => generated::JAVA_JAR,
        "kt" | "kts" => generated::KOTLIN,
        "scala" | "sc" | "sbt" => generated::SCALA,
        "clj" | "cljs" | "cljc" | "edn" => generated::CLOJURE,
        "groovy" | "gvy" | "gy" | "gsh" => generated::GROOVY,

        // Shell/Scripting
        "sh" | "bash" | "zsh" | "fish" => generated::BASH,
        "ps1" | "psm1" | "psd1" => generated::POWERSHELL,

        // PHP
        "php" | "php3" | "php4" | "php5" | "phtml" | "blade.php" => generated::PHP,

        // Mobile
        "swift" => generated::SWIFT,
        "dart" => generated::DART,
        "flutter" => generated::FLUTTER,

        // Functional languages
        "ex" | "exs" => generated::ELIXIR,
        "erl" | "hrl" => generated::ERLANG,
        "hs" | "lhs" => generated::HASKELL,
        "ml" | "mli" => generated::OCAML,
        "elm" => generated::ELM,
        "rkt" | "ss" => generated::RACKET,
        "scm" => generated::SCHEME,
        "lisp" | "cl" | "el" => generated::LISP,
        "gleam" => generated::GLEAM,

        // Systems languages
        "zig" => generated::ZIG,
        "nim" | "nims" => generated::NIM,
        "v" => generated::V,
        "odin" => generated::ODIN,
        "cr" => generated::CRYSTAL,
        "d" => generated::D,
        "hx" => generated::HAXE,

        // Scientific/Data
        "r" | "rmd" | "rdata" | "rds" => generated::R,
        "jl" => generated::JULIA,
        "m" | "mat" => generated::MATLAB,
        "do" | "ado" => generated::STATA,
        "f" | "f90" | "f95" | "for" | "f77" => generated::FORTRAN,

        // Legacy/Other
        "pl" | "pm" | "t" => generated::PERL,
        "lua" | "luau" => generated::LUA,
        "asm" | "s" | "S" => generated::ASSEMBLY,
        "cob" | "cbl" | "cpy" => generated::COBOL,
        "pro" | "P" => generated::PROLOG,

        // Hardware/Embedded
        "sv" | "svh" | "vh" | "vhd" | "vhdl" => generated::VERILOG,
        "ino" | "pde" => generated::ARDUINO,
        "cu" | "cuh" => generated::CUDA,

        // Blockchain
        "sol" | "vy" => generated::SOLIDITY,

        // Shaders
        "glsl" | "vert" | "frag" | "hlsl" | "wgsl" => generated::SHADER,

        // Config files
        "ini" | "cfg" | "conf" => generated::CONFIG,
        "env" => generated::ENV,
        "lock" => generated::LOCK,

        // Images
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tiff" | "tif" | "psd" | "ai"
        | "eps" => generated::IMAGE,
        "svg" => generated::SVG,

        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" => generated::FONT,

        // Audio/Video
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" => generated::AUDIO,
        "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv" | "wmv" | "m4v" => generated::VIDEO,
        "mid" | "midi" => generated::MIDI,

        // 3D/CAD
        "obj" | "fbx" | "gltf" | "glb" | "stl" | "dae" | "blend" => generated::_3D,

        // Documents
        "pdf" => generated::PDF,
        "txt" | "text" | "rtf" => generated::TEXT,
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => generated::BINARY,
        "diff" | "patch" => generated::DIFF,
        "log" => generated::LOG,

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "tbz2" => generated::ZIP,

        // Binary/Executable
        "exe" | "dll" | "so" | "dylib" | "bin" | "msi" | "app" => generated::BINARY,
        "wasm" | "wat" => generated::WEB_ASSEMBLY,

        // Database
        "sql" => generated::DATABASE,
        "prisma" => generated::PRISMA,
        "graphql" | "gql" => generated::GRAPHQL,

        // Infrastructure
        "tf" | "tfvars" | "tfstate" | "hcl" => generated::TERRAFORM,

        // Certificates/Keys
        "pem" | "crt" | "cer" | "der" => generated::CERTIFICATE,
        "key" | "pub" => generated::KEY,

        // HTTP/API
        "http" => generated::HTTP,
        "proto" => generated::PROTO,

        // Diagrams
        "mmd" | "mermaid" => generated::MERMAID,
        "drawio" => generated::IMAGE,

        // Nix
        "nix" => generated::NIX,

        _ => return None,
    })
}

/// Checks if file path is a README file
pub fn is_readme(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase().starts_with("readme"))
        .unwrap_or(false)
}

/// Checks if file path is a markdown file
pub fn is_markdown(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            lower == "md" || lower == "markdown"
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_file_icon() {
        let svg = get_icon_svg("main.rs");
        assert!(svg.contains("svg"), "Should return SVG content");
        assert!(svg.contains("#ef9f76"), "Rust icon uses peach color");
    }

    #[test]
    fn test_folder_icon() {
        let svg = get_icon_svg("src/");
        assert!(svg.contains("svg"), "Should return SVG content");
    }

    #[test]
    fn test_readme_icon() {
        let svg = get_icon_svg("README.md");
        assert!(svg.contains("svg"), "Should return SVG content");
    }

    #[test]
    fn test_unknown_extension_fallback() {
        let svg = get_icon_svg("unknown.xyz123");
        assert_eq!(
            svg,
            generated::_FILE,
            "Unknown extension uses default file icon"
        );
    }

    #[test]
    fn test_is_readme() {
        assert!(is_readme("README.md"));
        assert!(is_readme("readme.txt"));
        assert!(is_readme("README"));
        assert!(!is_readme("CONTRIBUTING.md"));
    }

    #[test]
    fn test_is_markdown() {
        assert!(is_markdown("file.md"));
        assert!(is_markdown("file.markdown"));
        assert!(!is_markdown("file.txt"));
    }
}
