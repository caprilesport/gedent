# gedent

`gedent` is an input generator for computational chemistry workflows.
It combines a cascading configuration system with [Tera](https://keats.github.io/tera/)
templates to generate input files for quantum chemistry software such as
[ORCA](https://www.faccts.de/orca/), [Gaussian](https://gaussian.com/),
[XTB](https://xtb-docs.readthedocs.io/en/latest/),
[ADF](https://www.scm.com/), [NWChem](https://www.nwchem-sw.org/), and others.

`gedent` stands for _gerador de entradas_ — Portuguese for "input generator". 🇧🇷

## Is it any good?

[Yes.](https://news.ycombinator.com/item?id=3067434)

---

## Installation

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### From [crates.io](https://crates.io/crates/gedent)

```bash
cargo install gedent
gedent --set-up       # create ~/.config/gedent/ with default templates and config
gedent --health       # verify the setup
```

### From source

```bash
git clone https://github.com/caprilesport/gedent.git
cd gedent
cargo build --release
# binary at target/release/gedent
```

---

## Configuration

gedent uses a **cascading config** system. When you run `gedent gen`, it
walks up from the current directory looking for `gedent.toml` files, then
merges them with the global `~/.config/gedent/gedent.toml`. Deeper files win
key-by-key — the global config sets defaults, project configs override them.

### Config file structure

```toml
[gedent]
default_extension = "inp"   # output file extension
software = "orca"           # default software (used for template disambiguation)

[model]
method = "pbe0"
basis_set = "def2-tzvp"
charge = 0
mult = 1
dispersion = "d3bj"
solvent = "water"
solvation_model = "smd"     # smd | cpcm | alpb | cosmo | ...

[resources]
nprocs = 8
mem = 3000                  # MB per core

[parameters]
# Arbitrary key-value pairs available in templates as Tera variables.
# Useful for software-specific or job-specific settings.
maxiter = 500
frozen_atoms = [1, 2, 3]
```

### Useful config commands

```bash
gedent config print                # show merged config
gedent config print --location     # show per-file contributions + merged result
gedent config edit                 # open nearest local gedent.toml in $EDITOR
gedent config edit --global        # open ~/.config/gedent/gedent.toml
gedent init                        # create a gedent.toml in the current directory
```

### One-off overrides with `--var`

Any context variable can be overridden for a single run without editing config:

```bash
gedent gen scan --var frozen_atoms="[20, 28]" --var nsteps=20 mol.xyz
gedent gen sp --method b3lyp --basis-set def2-svp mol.xyz
```

Values after `--var` are parsed as TOML literals, so integers, booleans, and
arrays work naturally.

---

## Templates

Templates live in `~/.config/gedent/templates/<software>/<jobtype>` and are
rendered with [Tera](https://keats.github.io/tera/), a Jinja2-like engine.

### Frontmatter

Each template starts with a Tera comment block that declares its metadata:

```
{#
software = "orca"
jobtype = "sp"
requires = ["method", "basis_set", "charge", "mult", "nprocs", "mem", "Molecule"]
description = "Single point energy"
#}
```

- `requires` — variables that must be present in context before rendering.
  gedent reports a clear error listing what is missing.
- `software` and `jobtype` — used by the template picker and the workflow layer.

### Available context variables

All keys from `[model]`, `[resources]`, and `[parameters]` are injected into
the Tera context. Key names match the TOML keys exactly:

| Variable          | Source            | Notes                                        |
|-------------------|-------------------|----------------------------------------------|
| `method`          | `[model]`         |                                              |
| `basis_set`       | `[model]`         |                                              |
| `charge`          | `[model]`         |                                              |
| `mult`            | `[model]`         |                                              |
| `dispersion`      | `[model]`         |                                              |
| `solvent`         | `[model]`         | also sets `solvation = true`                 |
| `solvation`       | derived           | `true` when `solvent` is set                 |
| `solvation_model` | `[model]`         |                                              |
| `nprocs`          | `[resources]`     |                                              |
| `mem`             | `[resources]`     |                                              |
| `name`            | molecule stem     | file stem of the input xyz file              |
| `Molecule`        | xyz file          | parsed molecule object (see below)           |
| anything else     | `[parameters]`    |                                              |

Variables are only present if they were set — use `{% if x is defined %}` before
referencing optional ones.

### The Molecule object

When an xyz file is provided, a `Molecule` is injected into context with:

- `Molecule.description` — comment line from the xyz file
- `Molecule.atoms` — list of `{ element, x, y, z }` atom objects

### Built-in Tera functions

gedent registers these functions in addition to
[Tera's built-ins](https://keats.github.io/tera/docs/#built-in-functions):

| Function | Arguments | Returns |
|---|---|---|
| `print_coords(molecule)` | `Molecule` | atom block (`element x y z` per line) |
| `natoms(molecule)` | `Molecule` | total atom count |
| `count_element(molecule, symbol)` | `Molecule`, string | count of atoms of that element |
| `element_list(molecule)` | `Molecule` | sorted unique element symbols |
| `atom_symbol(molecule, i)` | `Molecule`, 1-based index | element symbol of atom i |
| `atom_coords(molecule, i)` | `Molecule`, 1-based index | `[x, y, z]` array for atom i |
| `measure(molecule, atoms)` | `Molecule`, index array | distance (2), angle (3), or dihedral (4) in Å/° |

All index arguments are **1-based**.

### Template example

```
{#
software = "orca"
jobtype = "sp"
requires = ["method", "basis_set", "charge", "mult", "nprocs", "mem", "Molecule"]
description = "Single point energy"
#}
! {{ method }} {{ basis_set }}{% if dispersion is defined %} {{ dispersion }}{% endif %}{% if solvation %} SMD({{ solvent }}){% endif %}

%pal
 nprocs {{ nprocs }}
end

%maxcore {{ mem }}

*xyz {{ charge }} {{ mult }}
{{ print_coords(molecule = Molecule) }}
*
```

### Template commands

```bash
gedent template list               # list all available templates
gedent template print sp           # print template source
gedent template edit orca/opt      # open template in $EDITOR
gedent template new mytemplate     # create a new template from a preset
```

---

## Generating inputs

```bash
gedent gen sp mol.xyz              # generate sp.inp (or sp.<default_extension>)
gedent gen orca/sp *.xyz           # generate one file per xyz
gedent gen sp mol.xyz --print      # print to stdout instead of writing a file
gedent gen sp mol.xyz --dry-run    # validate and show what would be written, no output
gedent gen sp mol.xyz --show-context  # dump the full Tera context as JSON
gedent gen sp mol.xyz --ext gjf    # override output extension
gedent gen sp --software gaussian mol.xyz  # override software for template lookup
```

### Validation

Before rendering, gedent runs a validation pipeline and reports all issues at
once. Errors abort generation; warnings proceed with output.

Checks performed:
- **Charge and multiplicity** — electron count parity, physically impossible
  combinations
- **Superposed atoms** — error if any two atoms are closer than 0.5 Å; warning
  if closer than half the sum of their covalent radii
- **Missing template variables** — clear list of what `requires` but is absent
  from context
- **Solvation compatibility** — e.g. XTB in ORCA requires ALPB solvation
- **Composite method variables** — warning when `basis_set` or `dispersion` are
  set but the method (e.g. `pbeh-3c`) carries its own

---

## Shell completion

```bash
gedent --generate fish   # or bash, zsh
```

Pipe the output to the appropriate completion file for your shell. Template
names are completable in `gedent gen` and `gedent template` subcommands.

---

## Contributing

Contributions are welcome — bug reports, feature requests, and pull requests at
[github.com/caprilesport/gedent](https://github.com/caprilesport/gedent).

## Acknowledgments

Built on top of:

- [Tera](https://github.com/Keats/tera) — template engine
- [clap](https://github.com/clap-rs/clap) — CLI parsing
- [color-eyre](https://github.com/eyre-rs/color-eyre) — error reporting
- [serde](https://github.com/serde-rs/serde) + [toml](https://github.com/toml-rs/toml) — config parsing

## License

[MIT](LICENSE)
