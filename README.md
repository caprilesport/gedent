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

Before installing `gedent`
you need to make sure you have
[Rust](https://www.rust-lang.org/tools/install) (version 1.65.0 or later)
and [Cargo](https://doc.rust-lang.org/cargo/),
the package manager for Rust,
installed.

If you dont already have rust and cargo installed, you can
[install them with rustup](https://www.rust-lang.org/tools/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### From [crates.io](https://crates.io/crates/gedent) 

Once you have Rust and Cargo installed,
you can install `gedent` from [crates.io](https://crates.io/) using Cargo:

```bash
cargo install gedent
```

This will download the necessary dependencies,
compile the `gedent` binary,
and install it in your system.

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

`gedent` offers support for a per-project configuration file, it searches previous 
directories and if no config file - a 
a gedent.toml file - is found, it uses the 
default config location (`~/.config/gedent` in linux).

Pairing the user defined config file with the power of [TERA templates](https://keats.github.io/tera/)
gives rise to a rich system of input generation.

The config file accepts any keys, and is composed of a `[gedent]` block, which as of now
only supports a default extension, and a user defined `[parameters]` section, which can be 
accessed by templates. A default config 
file is provided by gedent with some example defaults for the templates that are 
shipped with the program.

## Templates tutorial

### Getting started

To understand the full functionalities of the templates, please visit
[the tera templates documentation](https://keats.github.io/tera/docs/#getting-started)
, which offers a comprehensive guide on the capabilities of the tera template language. 
It is heavily based on the [Jinja2](https://jinja.palletsprojects.com/en/3.1.x/)
and [Django](https://docs.djangoproject.com/en/5.0/ref/templates/language/) 
template languages, so if you know
any of these you will feel right at home.

### Creating new templates

To create new templates, you can add a base template in the `presets` directory, then call gedent:
```bash
gedent template new "new_template_name"
```

If you call it without the preset name, a fuzzy dialogue box will open for 
you to select what preset to use for your new template. It is then opened 
in your default to editor for you to modify it.

Right now, we ship the following basic template presets with `gedent`:
- [orca](https://www.faccts.de/orca/)
- [ADF](https://www.scm.com/) 
- [Gaussian](https://gaussian.com/) 
- [NWChem](https://www.nwchem-sw.org/) 

Although these are shipped by default, you are encouraged to create your own base presets. 

The only gedent-specific features in the templates is the metadata header.
On any template file if you use the special delimiter `--@` enclosing the template metada,
which can be placed anywhere in the input. 
Right now, the only supported metadata is the `extension`
directive, where it sets the default extension for the file, but there are plans to support templates
with more than 1 xyz file per template, for exemple.

An example of the template metada looks like this:

```toml
--@
extension = "inp"
--@
```

It is provided in [TOML](https://toml.io/en/) style syntax.

### Basic tera template usage

A Tera template is just a text file where variables and expressions 
get replaced with values when it is rendered. 
The syntax is based on Jinja2 and Django templates.

There are 3 kinds of delimiters and those cannot be changed:

- `{{` and `}}` for expressions
- `{%` and `%}` for statements
- `{#` and `#}` for comments

### Available functions

On top of the already built-in tera 
[functions](https://keats.github.io/tera/docs/#built-in-functions) and [filters](https://keats.github.io/tera/docs/#built-in-filters)
there are two more available functions as of now:
`print_molecule(molecule: Molecule)` and `split_molecule(molecule: Molecule, index: int)`.

`print_molecule`

## Example templates

[shipped templates](./templates)

## Contributing

Contributions to `gedent` are welcome!
If you find a bug,
have a feature request,
or want to contribute code,
please [open an issue](https://github.com/caprilesport/gedent/issues/new)
or [a pull request](https://github.com/caprilesport/gedent/pulls).

## License

`gedent` is released under the [MIT License](LICENSE).
