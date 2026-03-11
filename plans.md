# gedent plans

A living document of known issues, planned refactors, and future features.
Items are roughly ordered by priority / dependency.

---

## Refactors

### 10f. Software database
**Status:** done
Software-level metadata (default extension, compatible solvation models, etc.)
lives in `~/.config/gedent/software.toml`, extracted on first run, user-editable.
Contains software entries, method entries (`has_own_basis`, `has_own_dispersion`),
and compatibility rules (`[[compat]]`).

---

## Features

### 11. Multi-format molecule input
**Status:** not started
The `BufRead`-based parsing interface makes adding new formats straightforward —
each is just a new `from_<format>(reader: impl BufRead)` function returning
`Molecule`. Formats worth supporting:
- **Extended xyz** (highest priority) — per-atom properties (forces, partial
  charges, etc.) in key=value pairs on the comment line; popular in ML potentials
  and modern QC workflows
- **mol/SDF** — common interchange format from drawing tools
- **mol2** — common in docking and molecular dynamics
- **PDB** — needed for biomolecular calculations

Note: `Molecule.description` is xyz-specific. As more formats are added, consider
replacing it with `metadata: HashMap<String, String>` populated by each parser,
or dropping it entirely since it rarely contains anything useful.

### 14. Molecule connectivity graph
**Status:** not started
Depends on item 1. Build a bond graph from covalent radii + distance threshold
(petgraph is the natural crate). From this, several things become possible:
- Fragment detection (connected components) — automatically identify how many
  independent fragments are in the file
- Automatic charge/mult splitting for counterpoise or interaction energy inputs
- Foundation for atom selection (item 15)

### 15. Atom selection
**Status:** not started
Depends on item 14. A way to select subsets of atoms for use in templates.
Primary use case: hybrid hessian and TS calculations where you need a specific
set of atoms (the TS core) plus neighbors within N bonds. Exposed as Tera
functions:
- `select_within(molecule, seed_atoms, n_bonds)` — BFS from seed atoms,
  return all atoms within n bonds
- `select_element(molecule, symbol)` — all atoms of a given element
- `bonded_to(molecule, i)` — direct neighbors of atom i in the connectivity graph
- `format_atoms(molecule, indices)` — print a subset of atoms by index list;
  pairs naturally with `select_within` for writing active-region blocks
Selections produce atom index lists that other template functions can consume.

### 16. Multi-role molecule inputs (NEB and similar)
**Status:** not started
Distinct from the removed trajectory support. Some calculations require multiple
named xyz files with semantic roles: `--reactant mol_r.xyz --product mol_p.xyz`,
optionally `--ts mol_ts.xyz`. The Tera context would expose named molecules
(`Reactant`, `Product`, `TS`) rather than a flat list. Needed for NEB, IRC
endpoint verification, and linear transit inputs.

### 17. Pre-generation validation pipeline
**Status:** done
`Diagnostic { severity: Error | Warning, message }`, `validate()` pipeline,
charge/mult parity, superposed atoms (hard error < 0.5 Å; warning below covalent
radii threshold), missing template variables, compat rules, and method-vars
warnings all implemented in `src/validation.rs` and wired into `generate_input`.

### 18. `--dry-run` flag and context introspection
**Status:** done
- `--dry-run` — runs full validation, prints what would be generated, writes nothing.
- `--show-context` — dumps the full Tera context as JSON before validation.
Both flags live on `gedent gen`.

### 19. Method abstraction and compatibility database
**Status:** done
`software.toml` (user-editable, extracted on first run) holds software entries,
method entries (`has_own_basis`, `has_own_dispersion`), and `[[compat]]` rules.
`SoftwareDb::load()` in `src/software.rs`; falls back to an empty database if
the file is absent (non-fatal). Wired into `validate()` for compat and
method-vars checks.

### 20. Workflow layer
**Status:** not started
Depends on items 11–17 being reasonably solid. An opinionated layer on top of
templates for common multi-step sequences (e.g. `--workflow opt-sp` runs geometry
optimization then single-point). Workflows pre-populate context and pick the right
template automatically. Quality tiers: `quick` / `production` / `benchmark`.


---

## Quality

### 23. `config print --location` per-file diff
**Status:** done
`config print --location` shows per-file contributions (which keys each
`gedent.toml` in the cascade actually sets) followed by the fully merged result.
Implemented via `raw_contributions()` + `collect_chain_raw()` in `src/config.rs`.

### 21. Tests
**Status:** done
Unit tests: config cascade (7 tests), xyz parser, `build_context` (6 tests),
`render_inputs`, `parse_frontmatter`, validation pipeline (15 tests covering
charge/mult, superposed atoms, missing vars, compat rules, method-vars),
`parse_var` (8 tests). Total: 85 unit tests.

Integration tests (`tests/integration.rs`): 14 CLI tests covering gen --print,
file output, multiple files, --dry-run, --show-context, --var, validation errors,
config print, and template list/print. All hermetic via `GEDENT_HOME` env var.

### 24. Documentation
**Status:** done
- Rustdoc on all public types and functions in all source files
- README rewritten to reflect current state (config format, template authoring,
  Tera functions table, validation pipeline, --var, --dry-run, --show-context,
  shell completion)
