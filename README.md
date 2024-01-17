# gedent

`gedent` is an input generator for computational chemistry workflows. 
It strives to be simple and as general as possible, while contained in the
boundaries of the quantum chemistry research. 

`gedent` stands for *gerador de entradas*, which is the portugues translation for 
input generator. 🇧🇷

`gedent` aims to provide a paradigm of configurations and templates for software 
such as [XTB](https://xtb-docs.readthedocs.io/en/latest/), 
[orca](https://www.faccts.de/orca/), 
[ADF](https://www.scm.com/), 
[Gaussian](https://gaussian.com/), 
[NWChem](https://www.nwchem-sw.org/) 
and similar chemistry software suites. 
 Although it aims to support such software and was thought with this
use case in mind, it is a template CLI combined with a user defined configuration,
so if you find another use for it, feel free to open a pull request with 
features that support your needs.

## Is it any good?

[Yes.](https://news.ycombinator.com/item?id=3067434)

## Installation

### Requirements

Before installing `gedent`, ensure that you have the following prerequisites:

- [Rust](https://www.rust-lang.org/tools/install) (version 1.65.0 or later)
- [Cargo](https://doc.rust-lang.org/cargo/), the Rust package manager

If Rust and Cargo are not already installed, you can conveniently set them up using the following one-liner:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Installing from [crates.io](https://crates.io/crates/gedent)

Once Rust and Cargo are in place, `gedent` can be installed from [crates.io](https://crates.io/) using Cargo:

```bash
cargo install gedent
```

This command downloads necessary dependencies, compiles the `gedent` binary, and installs it on your system.

### Directly from [GitHub](https://github.com/caprilesport/gedent)

Alternatively,
you can install `gedent` directly from the GitHub repository
using Cargo by running:

```bash
cargo install --git=https://github.com/caprilesport/gedent.git
```

### By cloning the GitHub repository

You can also build `gedent` from source by cloning the GitHub repository
and running `cargo build`:

```bash
git clone https://github.com/caprilesport/gedent.git
cd gedent
cargo build --release
```

After building,
the binary will be located at `target/release/gedent`.


## Configuration

`gedent` supports per-project configuration files. If no config file (`gedent.toml`) is found in the current directory or its parent directories, the tool defaults to using the configuration at `~/.config/gedent` on Linux.

If you want to modify the parameters in the config just for the current folder (or for any subfolder in it), you can clone the current in-use configuration file by using:

```bash
gedent init
```

Pairing the user defined config file with the power of [TERA templates](https://keats.github.io/tera/)
gives rise to a rich system of input generation.

Here is an example configuration for `gedent`:

```toml
[gedent]
default_extension = "gjf"

[parameters]
charge = 1
mult = 1
solvation = true
solvent = "dmso"
random_field = "any valid toml data"
other_named_key = [100, 38, 29]
```

The `[gedent]` block contains default settings, and the `[parameters]` section allows user-defined configurations accessed by templates.

A default config file is provided by gedent with some example defaults for the templates that are shipped with the program.


## Templates basics

In `gedent`, templates play a central role in generating the inputs. The `template` subcommand facilitates template management.

### Getting started

For a comprehensive guide on template capabilities, refer to the [Tera templates documentation](https://keats.github.io/tera/docs/#getting-started).It is heavily based on the [Jinja2](https://jinja.palletsprojects.com/en/3.1.x/)
and [Django](https://docs.djangoproject.com/en/5.0/ref/templates/language/) 
template languages, so if you know
any of these you will feel right at home.

### Creating new templates

To create new templates, you can add a base template in the `presets` directory, then call gedent:

```bash
gedent template new "new_template_name"
```

If no preset name is provided, a fuzzy dialogue box assists in selecting a preset for your new template. The template is then opened in your default editor for modification.

`gedent` ships with default template presets for popular chemistry software, but users are encouraged to create custom base presets.
Right now, we ship the following basic template presets (if your favorite software is not supported, please [open a pull-request](https://github.com/caprilesport/gedent/issues/new)):
- [orca](https://www.faccts.de/orca/)
- [ADF](https://www.scm.com/) 
- [Gaussian](https://gaussian.com/) 
- [NWChem](https://www.nwchem-sw.org/) 

#### Template Metadata

Templates support a metadata header using the `--@` delimiter. Presently, the only supported metadata is the `extension` directive, setting the default file extension. Future releases plan to support templates with more than one XYZ file per template.

Example template metadata:

```toml
--@
extension = "inp"
--@
```

The metadata uses [TOML](https://toml.io/en/) syntax.


### Template rendering

`gedent` renders templates using the `.xyz` format, and users can leverage [openbabel](https://openbabel.org/) for format conversion if needed. Generating an input file is straightforward:

```bash
gedent gen `name_of_template` example.xyz
```

Wildcard support is available, allowing commands like `gedent gen orca/opt *.xyz`. Use `gedent template list` to view all available templates.

### The molecule object

`gedent` parses an XYZ file into a `Molecule` object, which includes the following fields: `filename`, `description`, and `atoms`. 
On top of the already built-in tera 
[functions](https://keats.github.io/tera/docs/#built-in-functions) and [filters](https://keats.github.io/tera/docs/#built-in-filters) two additional functions that receive a Molecule are provided, `print_molecule` and `split_molecule`.

Example `Molecule` object fields:

```bash
filename: example
description: Sample XYZ file
atoms: ["O  0.0  0.0  0.0", "H  0.0  1.0  0.0", "H  1.0  0.0  0.0"]
```

### Workflow example

#### Orca optimization

Let's walk through an example using `gedent` to generate an input file for optimizing a water molecule with [Orca](https://www.faccts.de/orca/).

1. **Create a New Template:**
   ```bash
   gedent template new opt orca
   ```
   This command generates a new template named `opt` based on the `orca` preset and opens the file in your default editor.

2. **Edit the Template:**
   With the generated template open in your editor, modify it to fit the optimization scenario. For example:

   ```bash
   --@
   extension = "inp"
   --@
   ! {{ functional }} {{ basis_set }}
   ! Opt freq

   %pal
    nprocs {{ nprocs }}
   end

   %maxcore {{ memory }}

   *xyz {{ charge }} {{ mult }}
   {{ print_molecule(molecule = Molecule) }}
   *
   ```

3. **Review Configuration:**
   Ensure the configuration parameters match the requirements. Check the configuration using:
   ```bash
   gedent config print
   ```


   Which will print something like this:

   ```toml
   charge = 1
   basis_set = "def2svp"
   functional = "BP86"
   dft_type = "GGA"
   memory = 3000
   mult = 1
   nprocs = 8
   solvation = false
   solvent = "water"
   start_hessian = false
   ```

4. **Adjust Configuration:**
   If needed, modify the configuration using:
   ```bash
   gedent config set
   ```

5. **Generate Optimization Input:**
   ```bash
   gedent gen opt h2o.xyz
   ```
   This generates the optimization input based on the template and specified XYZ coordinates.


   Which yields: 

   ```bash
   ! BP86 def2svp
   ! Opt freq D3BJ

   %pal
    nprocs 8
   end

   %maxcore 3000

   *xyz 0 1
   O       -0.981036882      0.000000000     -2.282900972
   H       -0.981036882      0.759337000     -1.686857972
   H       -0.981036882     -0.759337000     -1.686857972
   *
   ```

## Example templates

To understand how to create inputs with `gedent`, explore the shipped templates in the [templates directory](./templates) or the [presets](./presets). These examples serve as valuable references for creating your custom templates.


## Interactive Help

For quick access to command summaries and options, utilize the `--help` flag. For example:

```bash
gedent --help
gedent template --help
gedent gen --help
```

## Contributing

Contributions to `gedent` are welcome!
If you find a bug,
have a feature request,
or want to contribute code,
please [open an issue](https://github.com/caprilesport/gedent/issues/new)
or [a pull request](https://github.com/caprilesport/gedent/pulls).

## Acknowledgments

`gedent` was built on top of the following amazing crates:

- [TERA](https://github.com/Keats/tera)
- [Dialoguer](https://github.com/console-rs/dialoguer)

## License

`gedent` is released under the [MIT License](LICENSE).


