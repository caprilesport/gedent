# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                    # build
cargo build --release          # release build
cargo test                     # run all tests
cargo test <test_name>         # run a single test (e.g. `cargo test xyz_parse_works`)
cargo clippy                   # lint
cargo run -- <args>            # run with arguments (e.g. `cargo run -- --help`)
```

## Architecture

Four source files, each with a clear responsibility:

- **`src/main.rs`** — CLI definition (clap), subcommand dispatch, and the `generate_input` function that wires config + molecule + template into output files. Also owns `get_gedent_home()` (resolves `~/.config/gedent/`) and `setup_gedent()` (bootstraps config dir from embedded files).

- **`src/config.rs`** — `Config` struct backed by `gedent.toml`. Config lookup walks up from cwd (recursive `find()`), falling back to `~/.config/gedent/gedent.toml`. The `[gedent]` table holds tool settings (`default_extension`); `[parameters]` is a free-form TOML map injected directly into the Tera context.

- **`src/template.rs`** — `Template` struct wrapping a Tera template string. Templates live in `~/.config/gedent/templates/` and can have an optional `--@...--@` TOML header (currently supports `extension`). Registers two custom Tera functions: `print_molecule` (renders atoms as newline-joined string) and `split_molecule` (splits atoms at index, returns two-element array).

- **`src/molecule.rs`** — `Molecule` struct (`filename`, `description`, `atoms: Vec<String>`) parsed from XYZ files. `from_xyz()` supports multi-molecule XYZ files (returns `Vec<Molecule>`). `split(index)` splits a molecule into two fragments.

### Data flow for `gedent gen`

1. XYZ files → `Molecule::from_xyz()` → `Vec<Molecule>`
2. `Config::get()` → all `[parameters]` inserted into `tera::Context`
3. CLI flags override context values (method, basis_set, charge, mult, etc.)
4. `Template::get()` → parses header, loads Tera template string
5. For each molecule: clone context, insert `Molecule` object, call `template.render()`
6. Output filename = molecule filename (or template name if no molecules) + extension from template header or `default_extension`

### Embedded assets

Presets and default templates are embedded at compile time via `include_dir!` macros in `main.rs`, and extracted to disk by `setup_gedent()`. The default `gedent.toml` config is embedded via `include_str!`.
