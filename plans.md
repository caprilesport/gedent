# gedent plans

A living document of known issues, planned refactors, and future features.
Items are roughly ordered by priority / dependency.

---

## Refactors

### 1. Parse atoms into a real struct
**Status:** done
Atoms are currently stored as raw strings (`"C  -0.702  0.000  -1.996"`).
Parse them into `Atom { symbol: String, x: f64, y: f64, z: f64 }` at load time.
This is the foundational change that unlocks almost everything else: atom counting,
geometric measurements, connectivity, electron counting, charge/mult validation,
and better error messages.

### 2. Fix and harden the XYZ parser
**Status:** done
Current parser breaks on trailing blank lines, doesn't validate atom line format,
and doesn't handle Windows line endings (CRLF). Fix all of these. Also simplify
the loop — most of the complexity comes from the multi-xyz trajectory feature
(see item 3).

### 3. Drop multi-xyz trajectory support for now
**Status:** done
In practice a single xyz file per invocation is the common case. The multi-molecule
loop logic and the `_0`/`_1` filename suffixing is the main source of parser
complexity. Remove it. Can be re-added later on top of a cleaner base.

### 4. Improve abstractions / domain model
**Status:** not started
Depends on items 1–3. Once `Atom` is a real type and the parser is clean, revisit
the `Molecule` struct and `generate_input` to make the domain model feel right.
`generate_input` currently does context-building, rendering, and output in one
function — split these apart.

### 5. Replace `anyhow` with `color_eyre`
**Status:** not started
`color_eyre` (built on `eyre`, a fork of `anyhow` with the same API) adds three
things that matter for a CLI:
- Colorized, structured error output in the terminal
- `.suggestion()` and `.note()` via the `Section` trait — attach human-facing
  hints directly to errors (e.g. "Did you forget to run `gedent --set-up`?" or
  "Charge/mult parity is wrong — try mult = 2 for a radical"). This pairs
  directly with the validation pipeline.
- `color_eyre::install()` in `main` improves panic output during development.

Migration cost is low: `anyhow::Error` → `eyre::Report`, `anyhow::bail!` →
`eyre::bail!`, `anyhow::anyhow!` → `eyre::eyre!`, `.context()` unchanged.
Do this before the validation pipeline so warnings and diagnostics can use
`.suggestion()` from the start.

### 6. Remove the template header (`--@...--@`)
**Status:** done
The bespoke TOML-in-comment header format is hard to document, non-standard, and
the only thing it carries right now is `extension`. Replace with an `--ext` CLI
flag. Per-software extension defaults can live in a config section or a software
enum later.

### 7. Remove `split_molecule` template function
**Status:** done
Too niche and premature given the current state of the codebase. Remove now,
revisit after the domain model is solid.

### 8. Remove `dialoguer` and make all CLI arguments required
**Status:** done
Interactive fuzzy-select fallbacks (for template name, software, config key) are
band-aid UX over missing required arguments. Remove `dialoguer` entirely and make
the affected arguments non-optional. Cleaner API surface, fully scriptable, no
hidden TTY dependency.

### 9. Replace `include_dir!` setup with xtask
**Status:** not started
Embedding presets/templates/config at compile time via `include_dir!`/`include_str!`
is the wrong layer for what is essentially a runtime installation step. Move setup
to an xtask crate (or a simple install subcommand backed by xtask). This also
makes it easier to update defaults without recompiling.

### 10. Template organization
**Status:** no solution yet
A flat directory that the user must know the path of is bad UX. Options include
organizing by software, by calculation type, or adding lightweight metadata.
Revisit once the domain model and template format are stable.

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

### 12. Basic Tera template functions
**Status:** not started
Depends on item 1 (real `Atom` type). Useful functions:
- `natoms(molecule)` — total atom count
- `count_element(molecule, symbol)` — count atoms of a given element
- `element_list(molecule)` — unique elements present
- `center_of_mass(molecule)` — may be useful for some inputs
- `atom_symbol(molecule, i)` / `atom_coords(molecule, i)` — indexed accessors,
  useful for writing frozen atom blocks or geometry constraints
- `nuclear_repulsion(molecule)` — occasionally required in input files as a
  reference energy

### 13. Geometric measurements in templates
**Status:** not started
Depends on item 1. Expose as Tera functions so templates can embed computed
geometry directly in input files:
- `distance(molecule, i, j)` — bond length between atoms i and j
- `angle(molecule, i, j, k)` — valence angle
- `dihedral(molecule, i, j, k, l)` — torsion angle
Useful for constrained optimizations, scan inputs, and anything that needs
explicit internal coordinates.

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
Depends on item 11. A way to select subsets of atoms for use in templates.
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
**Status:** not started
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
A data-driven method database (TOML, shipped with gedent and user-extensible)
that describes method properties:
```toml
[methods.pbeh-3c]
has_own_basis = true
has_own_dispersion = true
kind = "composite"

[methods.xtb]
kind = "semiempirical"
has_own_basis = true
compatible_solvation = ["ALPB"]

[methods.pbe0]
kind = "dft"
requires_basis = true
compatible_solvation = ["CPCM", "SMD", "COSMO"]
```
The context builder uses these properties to decide what variables to populate.
Templates should use `{% if basis_set is defined %}` for conditionally present
fields. Alternatively, composite methods get their own templates entirely (a
pbeh-3c template simply has no `{{ basis_set }}`), with the workflow layer
responsible for picking the right template.

**2. Method × software × solvation compatibility validation**
Cross-product constraints like "XTB in ORCA must use ALPB, not CPCM" are
validation rules, not type-level constraints. These belong in the validation
pipeline (item 15) as `check_solvation_compatibility(method, software,
solvation_model)` with a `.suggestion()` via color_eyre pointing to the correct
solvation model.

This item is closely related to the workflow layer (item 18) — once method
metadata exists, workflows can use it to pre-validate and auto-configure
calculations rather than relying purely on user-provided variables.

### 20. Workflow layer
**Status:** not started
Depends on items 10–17 being reasonably solid. An opinionated layer on top of
templates for common multi-step sequences (e.g. `--workflow opt-sp` runs geometry
optimization then single-point). Workflows pre-populate context and pick the right
template automatically. Quality tiers: `quick` / `production` / `benchmark`.

---

## Quality

### 21. Tests
**Status:** inadequate
Current tests only cover happy paths for individual units. Needed:
- Error case coverage for the xyz parser (blank lines, malformed atoms, CRLF)
- Tests for `generate_input`
- Integration tests (invoke the CLI, check output files)
- Property-based tests for the parser once the atom struct is in place

### 22. Documentation
**Status:** inadequate
- Add rustdoc to all public types and functions
- Expand the README with real usage examples and template authoring guide
- Document the config file format and lookup behaviour
- Document available Tera functions and their signatures

