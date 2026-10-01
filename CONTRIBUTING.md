# Contributing

## Getting set up

```bash
git checkout update_chglog_230826   # main has no source, see AGENTS.md
cargo build --release               # currently fails — see "First task" below
```

Read [AGENTS.md](AGENTS.md) first — it covers the branch layout, repo
structure, and the specific things that currently don't work.

## Project state

This is an early, unfinished project. The language core (lexer/parser/
interpreter/compiler for `.tru` files) is the real substance; most of the
"AI/ML util library" scaffolding around it (`domain/`, `aimodels/`,
`models/`, parts of `secure/`) is placeholders — declared modules with no
file behind them, or files with no caller. Don't assume something works
because a module name suggests it should; check `AGENTS.md`'s "Known gaps"
list and grep for actual usage before building on top of anything.

## Starter tasks

Pick one, roughly in order of how foundational it is:

1. **Make it compile.** `cargo build --release` currently fails with ~30
   errors: missing module files, the working `tru_lang` implementation
   sitting in `src/tru_lang/_rs_archive/` instead of where `lib.rs` expects
   it, a couple of outdated `chrono`/`rand` API calls. This blocks
   everything else — good first PR.
2. **Write `.tru` test programs and find parser/interpreter bugs.** The
   condition/struct-literal ambiguity in `AGENTS.md` is one example of a
   bug that breaks any loop or `if` ending in a bare identifier. Write
   small `.tru` programs (structs, nested loops, match, recursion, string
   methods) and run them with `tru_id run` — anything that errors
   unexpectedly is a real bug.
3. **Fix the stale example files.** `examples/performance_comparison.rs`
   and `examples/tru_compiler_example.rs` reference removed/nonexistent
   types. Either update them to match current APIs or delete them if
   they no longer demonstrate anything real.
4. **Decide the scope of the "AI/ML util" layer.** `domain/`, `aimodels/`,
   `models/` are mostly empty declarations. Before adding real code there,
   it's worth deciding (as a maintainer discussion, not a silent PR) whether
   this crate is "a language" or "a language + an AI app toolkit" — the
   README currently promises the latter but almost none of it exists.
5. **Clean up duplicated/dead files.** `src/lib/` and `src/module/` hold
   byte-identical copies of the same four `.tru` files; `trunk_old.toml` is
   an empty scratch file; `trn.toml` and `trunk.toml` are two different
   unused alternate build configs. Consolidate or remove once you've
   confirmed nothing references them.

## Before opening a PR

- `cargo build --release` and `cargo test --lib` must pass.
- If you touch the parser or interpreter, run the example programs in
  `examples/*.tru` manually — there's no integration test suite yet
  covering the CLI end to end.
