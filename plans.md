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

### 4. Introduce `[chemistry]` section in config
**Status:** done
The current `[parameters]` section mixes two distinct concerns: first-class chemistry
parameters (method, basis_set, charge, mult, solvent, etc.) and user-defined free-form
template variables. Separate them:

```toml
[gedent]
default_extension = "inp"

[chemistry]          # typed, validated by gedent, maps to GenOptions
method = "pbe0"
basis_set = "def2-tzvp"
charge = 0
mult = 1
dispersion = "d3bj"
solvent = "water"
solvation_model = "cpcm"
nprocs = 4
mem = 8000

[parameters]         # free-form user context, passed through to Tera as-is
my_custom_var = "foo"
```

CLI flags override `[chemistry]` values. `[parameters]` entries are never validated,
just inserted into the Tera context verbatim. `hessian` stays CLI-only (job-type
flag, not a system property).

Future: `[chemistry]` may be sub-divided into `[solvation]`, `[relativistic]`, etc.
when the validation pipeline (item 17) can act on them as groups. Not now.

### 4b. Config cascade / inheritance + gedent init rework
**Status:** done
Depends on item 4. Instead of stopping at the first `gedent.toml` found walking up
from cwd, merge all configs in the chain. Deepest (closest to cwd) wins per key in
both `[chemistry]` and `[parameters]`. The global `~/.config/gedent/gedent.toml`
is the base.

Example:
```
~/.config/gedent/gedent.toml   ← base: method=pbe0, basis=def2-tzvp, nprocs=8
~/projects/reaction/
  gedent.toml                  ← override: charge=0, mult=1
  proton_transfer/
    gedent.toml                ← override: charge=1, mult=2
```

Running `gedent gen` from `proton_transfer/` sees charge=1, mult=2, method=pbe0,
basis=def2-tzvp, nprocs=8. This makes it natural to share a project-level config
and only override what changes per calculation directory.

**`gedent init` rework:** the current behavior (copy the full resolved config to
`./gedent.toml`) is counterproductive in a cascade model — you'd end up with a
redundant full copy that obscures what you actually intend to override locally.
Replace with explicit key scaffolding: `gedent init` accepts chemistry flags and
writes a minimal override file containing only those keys:

```
gedent init --charge 1 --mult 2
```

produces:
```toml
[chemistry]
charge = 1
mult = 2
```

Running `gedent init` with no flags creates an empty `gedent.toml` with bare section
headers as a blank slate. The cascade handles the rest.

### 4c. Improve abstractions / domain model — split `generate_input`
**Status:** done

### 4d. Split `[chemistry]` into `[model]` and `[resources]`
**Status:** not started
`[chemistry]` conflates two semantically distinct concerns:
- `[model]` — what the calculation *is*: `method`, `basis_set`, `charge`, `mult`,
  `dispersion`, `solvent`, `solvation_model`. Changes per-project, per-directory.
  Subject to method compatibility validation (item 19).
- `[resources]` — what machine it runs on: `nprocs`, `mem`. Future: `use_gpu`,
  `scratch_dir`. Changes per-machine or per-cluster, rarely per-calculation.

```toml
[model]
method = "pbe0"
basis_set = "def2-tzvp"
charge = 0
mult = 1
dispersion = "d3bj"
solvent = "water"
solvation_model = "cpcm"

[resources]
nprocs = 20
mem = 3000
```

The validation pipeline (item 17) acts on these groups differently: charge/mult
parity is a `[model]` concern, `nprocs` sanity is a `[resources]` concern.
Tera context variable names (`{{ nprocs }}`, `{{ method }}`) do not change —
only the config file structure and Rust structs change. Do before item 17.
Depends on items 4 and 4b. With `[chemistry]` as a typed struct and the cascade
in place, split `generate_input` into focused pieces:
- `build_context(chemistry: &ChemistryConfig, params: &ParamsMap, opts: &GenOptions) -> tera::Context`
  — pure function, no I/O, testable. Takes the merged chemistry config + CLI
  overrides and builds the base Tera context. This is also the hook point for the
  validation pipeline (item 17).
- `Template::render_with_molecule(context: &tera::Context, molecule: &Molecule, stem: &str) -> Result<String>`
  — molecule insertion lives in `Template`, where it belongs.
- `generate_input` becomes a thin coordinator: merge config, call `build_context`,
  loop over molecules, collect `Vec<Input>`.

### 5. Replace `anyhow` with `color_eyre`
**Status:** done
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
**Status:** done
Design settled. Implementation items:

**a. `software/jobtype` directory convention**
Templates live at `~/.config/gedent/templates/software/jobtype`, e.g.
`templates/orca/sp`, `templates/xtb/gfn2`. This is a storage convention only —
not the user-facing name in normal use.

**b. Tera comment frontmatter**
Each template declares its interface in a `{# ... #}` block (stripped at render
time, zero leakage into output):
```
{#
software = "orca"
jobtype = "sp"
requires = ["method", "basis_set", "charge", "mult", "nprocs", "mem"]
description = "Single point energy"
#}
! {{ method }} {{ basis_set }}
...
```
Only the first `{# ... #}` block is treated as frontmatter if it parses as valid
TOML. Templates without frontmatter still work — metadata is absent, validation
skips gracefully. `requires` is used by the validation pipeline (item 17) to
check for missing variables before rendering.

**c. `software` key in `[gedent]` config**
Optional. Used as tiebreaker when short-name lookup is ambiguous. Cascades
normally (local overrides parent) — setting it in a subdirectory `gedent.toml`
naturally scopes the default software for that subtree.

```toml
[gedent]
software = "orca"
default_extension = "inp"
```

**d. Short-name lookup with disambiguation**
```
gedent gen sp mol.xyz
  1. scan templates/*/sp
  2. one match  → use it
  3. no match   → error: no template named "sp"
  4. >1 match   → check gedent.software as tiebreaker
  5. still ambiguous → error: ambiguous: orca/sp, xtb/sp
                        hint: use full name or set software in gedent.toml
```
Full names (`gedent gen orca/sp`) always resolve directly, bypassing lookup.
Multi-software projects work naturally: `xtb/gfn2` and `crest/conformers` are
unambiguous by default; collisions (e.g. `orca/opt` vs `xtb/opt`) are resolved
by setting `software` in the project or subdirectory `gedent.toml`.

**e. Dynamic shell completion**
`gedent __complete templates` is a hidden subcommand that prints one completable
name per line, respecting the `software` config for tiebreaking. Short names are
used for unambiguous jobtypes; full `software/jobtype` names for collisions.
Shell completion scripts should call this endpoint for the `template_name`
argument of `gedent gen`. `gedent template list` now displays a borderless
two-column table (name | description) grouped by software using `comfy-table`.

**f. Software database**
Software-level metadata (default extension, compatible solvation models, etc.)
lives in `~/.config/gedent/software.toml`, extracted by `--set-up`. User-editable
so users can add custom software or update entries when software adds new features
(e.g. XTB gaining CPCM support). Not embedded in the binary. See also item 19.

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
A data-driven method + software database in `~/.config/gedent/software.toml`
(extracted by `--set-up`, user-editable, not embedded in binary):
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
Depends on items 10–17 being reasonably solid. An opinionated layer on top of
templates for common multi-step sequences (e.g. `--workflow opt-sp` runs geometry
optimization then single-point). Workflows pre-populate context and pick the right
template automatically. Quality tiers: `quick` / `production` / `benchmark`.

---

## Quality

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

### 22. Logging
**Status:** not started
Replace ad-hoc `println!` progress output with structured logging.
Stack: `log` crate (facade) + `env_logger` (backend) + `clap_verbosity_flag`
(CLI flags). `clap_verbosity_flag` adds `-v`/`-vv`/`-q` to the top-level CLI
and maps directly to `log::LevelFilter` — no glue code needed.

Default level: warn (silent on success). With `-v`: info (show "Writing foo.inp",
"Created gedent.toml." etc.). With `-vv`: debug (config chain, context keys,
template resolution steps). Error output stays on stderr via color_eyre.

Migration: replace `println!` progress calls with `log::info!`, add
`env_logger::Builder::new().filter_level(verbosity.log_level_filter()).init()`
in `main`. `tracing` is intentionally not used — it is designed for async and
distributed systems and is overkill for a synchronous CLI.

### 24. Documentation
**Status:** inadequate
- Add rustdoc to all public types and functions
- Expand the README with real usage examples and template authoring guide
- Document the config file format and lookup behaviour
- Document available Tera functions and their signatures

