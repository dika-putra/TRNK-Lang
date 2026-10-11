# AGENTS.md

Context for any AI agent (or human) working in this repository.

## What this project is

**TRU Language** (`.tru` files, crate name `tru_id_core`) is a small
Rust-hosted programming language: lexer → parser → tree-walk interpreter,
plus a transpiler that turns `.tru` source into Rust. It ships as a library
(`tru_id_core`) and two binaries:

- `tru_id` (`src/bin/main.rs`) — the CLI: `run`, `build`, `repl`, `check`, `new`, `version`.
- `tru_id_installer` (`src/bin/installer.rs`) — Windows installer that
  registers the `.tru` file extension.

`use` is a no-op in the interpreter, with one exception: `run_file` replaces
`use modules::<name>;` lines with the text of `modules/<name>.tru` next to the
entry file (plain include, shared global scope). `tru_id new` relies on this.

Everything else bolted onto the crate (database configs, JWT auth helpers,
"AI/ML" model utilities under `domain`/`aimodels`/`models`) is a secondary,
much less mature layer. Treat the language implementation as the actual
product; treat the rest as scaffolding, most of it unfinished (see "Known
gaps" below).

## Branch layout — read this first

**`main` contains no source code.** It only has README/LICENSE/CODE_OF_CONDUCT.
All the actual Rust and `.tru` source lives on the **`update_chglog_230826`**
branch, which has never been merged to `main`. Check out that branch (or a
feature branch based on it) before doing anything — don't expect to find
`src/` on `main`.

```bash
git checkout update_chglog_230826
```

## Build / run / test

```bash
cargo build --release        # builds lib + tru_id + tru_id_installer
cargo test --lib              # unit tests inside src/ (28 at last count)
./target/release/tru_id run examples/hello.tru
./target/release/tru_id repl
```

Needs Rust/cargo on PATH (`brew install rust` works fine on macOS; no
rustup required). `cargo test` (without `--lib`) also tries to build
`examples/*.rs`, several of which reference types that don't exist
(see below) — use `cargo test --lib` if you just want the language tests.

## Repo map

```
src/
├── lib.rs                      # crate root, re-exports, prelude modules
├── bin/{main,installer}.rs     # the two real binaries
├── tru_lang/                   # the actual language: lexer, parser, ast,
│                                 interpreter, compiler (.tru -> Rust), stdlib,
│                                 scaffold.rs + templates/ (`tru_id new` boilerplate)
├── trunk_lang_core.rs          # runtime primitives (string interning, pools,
│                                 perf metrics) used by tru_lang
├── trung_config.rs             # parses trunk.toml (an alternative, unused,
│                                 build-config format — see "Known gaps")
├── trung_build.rs              # a from-scratch build system for trunk.toml
├── utils/                      # date/uuid helpers, JWT/AES security helpers
├── secure/                     # auth::{Role, create_jwt, with_auth}, error::Error
├── domain/, aimodels/, models/, infrastructure/
│                                # mostly placeholders — many `pub mod X;`
│                                 declarations have no backing file (see below)
examples/                       # .tru sample programs + a few .rs demo files
install/                        # Windows installer scripts (.bat/.ps1)
lib/, src/lib/, src/module/     # stray / duplicated .tru files, not part of
                                 # the build (src/lib and src/module hold
                                 # byte-identical duplicates of the same 4 files)
```

Three competing config files exist: `Cargo.toml` (the one cargo actually
reads), `trunk.toml` (an aspirational alternate build config parsed by
`trung_config.rs`/`trung_build.rs` but not wired into any real workflow),
and `trn.toml`/`trunk_old.toml` (older, abandoned experiments). When in
doubt, `Cargo.toml` is ground truth.

## Known gaps — things that will trip you up

1. **The crate does not compile as-is.** Several `mod.rs` files declare
   `pub mod X;` for files that were never created (e.g. `utils::flowEngineUtils`,
   `aimodels::asist`, `domain::algorithmic_reasoner`, `models::quantization`,
   `secure::auth`/`secure::error`, and ~15 others). `secure::auth` and
   `secure::error` are the only two of these actually used elsewhere
   (`utils/secureUtils.rs` depends on them) — the rest are declared but
   never referenced anywhere in the codebase, i.e. dead scaffolding.
2. **`src/tru_lang/_rs_archive/`** holds the real, working Rust
   implementation of the language (lexer/parser/interpreter/compiler/stdlib +
   `trunk_lang_core.rs`), but `lib.rs` expects those modules directly under
   `src/tru_lang/` and `src/`. Someone started a self-hosting rewrite (the
   language's own stdlib/compiler written in `.tru` syntax: `src/tru_lang/*.tru`,
   `src/main.tru`, `src/trunk_lang_core.tru`) and archived the working Rust
   version in the process, leaving the crate unbuildable. The `.tru` rewrite
   is incomplete and isn't wired into the Cargo build at all.
3. **Parser has a condition/struct-literal ambiguity**: an `if`/`while`/`for`/
   `match` whose condition ends in a bare identifier immediately followed by
   `{` gets misparsed as a struct literal (`while i < n { ... }` tries to
   parse `n { ... }` as `n { field: val }`). Any such bug is fixed by
   restricting struct-literal parsing to contexts where `{}` isn't ambiguous
   (parens/brackets/call args), the way rustc itself does it — worth
   double-checking other condition-like parse sites for the same issue.
4. **`examples/performance_comparison.rs` and `examples/tru_compiler_example.rs`**
   reference types/functions that don't exist in the current source
   (`VectorOps`, `FastCache`, `get_tru_runtime`, etc.) — they're stale demos
   for code that was removed or never finished.
5. Function/type naming is inconsistent (`camelCase` file and function names
   like `helperUtils.rs`, `uniqueIdUUID()` next to idiomatic `snake_case`
   elsewhere) — this is pre-existing style debt, not a style guide to follow
   for new code.

## Conventions for changes here

- Prefer deleting dead/never-referenced `pub mod` declarations over writing
  empty stub files for them — nothing currently depends on most of them
  (verify with `grep -rn` before deleting; `secure::auth`/`secure::error`
  are real exceptions).
- When restoring or touching `tru_lang`, the Rust implementation in
  `_rs_archive/` (once moved into place) is the one to build on; the `.tru`
  self-hosting files are an unfinished parallel effort — don't try to merge
  behavior from both.
- Run `cargo build --release` and `cargo test --lib` before calling anything
  done; the crate has a history of silently-broken state (modules referencing
  files that don't exist won't show up except at compile time).
