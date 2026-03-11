# gedent plans

A living document of known issues, planned refactors, and future features.
Items are roughly ordered by priority / dependency.

---

## Refactors

### 10f. Software database
**Status:** not started
Software-level metadata (default extension, compatible solvation models, etc.)
lives in `~/.config/gedent/software.toml`, extracted on first run, user-editable.
Not embedded in the binary. See also item 19.

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
**Status:** partial — `Diagnostic` type, `validate()`, charge/mult, superposed atoms, and missing
vars all implemented in `src/validation.rs` and wired into `generate_input`
Rather than scattering ad-hoc `bail!` / `println!` calls, introduce a formal
validation layer: `Diagnostic { severity: Error | Warning, message }` returned
from a `validate(molecule, context) -> Vec<Diagnostic>` pipeline that runs before
rendering. Errors stop generation; warnings proceed with output. Specific warnings
should be suppressible via config.

Checks to implement in this pipeline:
- **Charge and multiplicity** — compute total electron count, validate parity and
  physical possibility of the provided charge/mult combination.
- **Superposed atoms** — any two atoms closer than ~0.5 Å is almost certainly a
  drawing error.
- **Unknown element symbols** — catch typos (`CA`, `ca` instead of `Ca`) before
  a bad input file is silently written.
- **Suspiciously long bonds** — atoms connected in the graph but with an
  unreasonable distance; indicates bad geometry or a unit mismatch (Bohr vs Å).
- **Unexpected fragment count** — warn if the connectivity graph shows a different
  number of fragments than the template expects (e.g. a single-molecule template
  receiving a system with 3 disconnected fragments).
- **Missing template variables** — check that all variables referenced in the
  template are present in context before rendering, and report a clear list of
  what is undefined rather than a cryptic Tera error.

### 18. `--dry-run` flag and context introspection
**Status:** not started
- `--dry-run` — run the full validation pipeline and print what would be generated
  without writing any files. Useful for debugging templates.
- `--show-context` (or `gedent gen --dry-run --verbose`) — dump the full Tera
  context so template authors can see exactly what variables are available and
  their types without guessing.

### 19. Method abstraction and compatibility database
**Status:** not started / design phase
Some methods are composite and have baked-in components (pbeh-3c, r2scan-3c,
HF-3c carry their own basis and dispersion; XTB has no basis set concept at all).
The flat `{{ method }}` / `{{ basis_set }}` / `{{ dispersion }}` variable model
breaks down for these — populating `{{ basis_set }}` for pbeh-3c is wrong, not
just empty.

Two sub-problems:

**1. Composite/semiempirical methods in context building**
A data-driven method + software database in `~/.config/gedent/software.toml`
(extracted on first run, user-editable, not embedded in binary):
```toml
[software.orca]
extension = "inp"
solvation_models = ["CPCM", "SMD", "COSMO"]

[software.xtb]
extension = "inp"
solvation_models = ["ALPB", "GBSA"]

[methods.pbeh-3c]
has_own_basis = true
has_own_dispersion = true
kind = "composite"

[methods.xtb]
kind = "semiempirical"
has_own_basis = true

[methods.pbe0]
kind = "dft"
requires_basis = true
```
The context builder uses these properties to decide what variables to populate.
Templates should use `{% if basis_set is defined %}` for conditionally present
fields. Alternatively, composite methods get their own templates entirely (a
pbeh-3c template simply has no `{{ basis_set }}`), with the workflow layer
responsible for picking the right template.

**2. Method × software × solvation compatibility validation**
Cross-product constraints like "XTB in ORCA must use ALPB, not CPCM" are
validation rules, not type-level constraints. These belong in the validation
pipeline (item 17) as `check_solvation_compatibility(method, software,
solvation_model)` with a `.suggestion()` via color_eyre pointing to the correct
solvation model.

This item is closely related to the workflow layer (item 20) — once method
metadata exists, workflows can use it to pre-validate and auto-configure
calculations rather than relying purely on user-provided variables.

### 20. Workflow layer
**Status:** not started
Depends on items 11–17 being reasonably solid. An opinionated layer on top of
templates for common multi-step sequences (e.g. `--workflow opt-sp` runs geometry
optimization then single-point). Workflows pre-populate context and pick the right
template automatically. Quality tiers: `quick` / `production` / `benchmark`.


---

## Quality

### 23. `config print --location` per-file diff
**Status:** not started
Currently `--location` lists the config chain paths and then dumps the fully
merged result, with no indication of where each value came from. A user seeing
`charge = 1` has no way to know if that came from the global config or a
`gedent.toml` five directories up. Enhance to show per-file contributions:

```
~/.config/gedent/gedent.toml
  method = "pbe0", basis_set = "def2-tzvp", charge = 0

~/projects/reaction/gedent.toml
  charge = 1

merged:
  method = "pbe0", basis_set = "def2-tzvp", charge = 1
```

Requires threading per-file key sets through the cascade (capture which keys
each `RawConfig` actually set before merging).

### 21. Tests
**Status:** partial
Added: config cascade (7 tests), xyz parser (4 tests including error cases),
`build_context` (6 tests covering overrides, fallthrough, solvent flag),
`render_with_molecule`, `parse_frontmatter` (2 tests), `missing_vars` (3 tests —
the core of item 17's pre-render check is already tested).

Still needed:
- Integration tests (invoke the CLI, check output files)
- Property-based tests for the xyz parser
- Tests for `generate_input` end-to-end

### 24. Documentation
**Status:** inadequate
- Add rustdoc to all public types and functions
- Expand the README with real usage examples and template authoring guide
- Document the config file format and lookup behaviour
- Document available Tera functions and their signatures
