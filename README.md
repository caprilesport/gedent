# gedent

`gedent` is an input generator for computational chemistry workflows. 
It strives to be simple and as general as possible, while contained in the
boundaries of the quantum chemistry research. 

As the needs for molecular dynamics software is much more diverse, `gedent` does
not aim to provide specific capabilites for this kind of software for now. (In the future, maybe?)

`gedent` aims to provide a paradigm of configurations and templates for software 
such as [XTB](https://xtb-docs.readthedocs.io/en/latest/), [orca](https://www.faccts.de/orca/), 
[ADF](https://www.scm.com/), [Gaussian](https://gaussian.com/), [NWChem](https://www.nwchem-sw.org/) 
and similar chemistry software suites. 

Although it aims to support such software and was thought with this
use case in mind, it is a template CLI combined with a user defined configuration,
so if you find another use for it, feel free to open a pull request with 
features that support your needs, or clone the repo `=)`.

## Is it any good?

[Yes.](https://news.ycombinator.com/item?id=3067434)

## Installation

### Requirements

Before installing `gedent`, through `cargo`
you need to make sure you have
[Rust](https://www.rust-lang.org/tools/install) (version 1.65.0 or later)
and [Cargo](https://doc.rust-lang.org/cargo/),
the package manager for Rust,
installed.

If you dont already have rust and cargo installed, you can [install them with rustup](https://www.rust-lang.org/tools/install):

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

Do note that the config directory is not create this way, so if you're on linux, please create the 
~/.config/gedent directory and copy the appropriate files and directories there:

```bash
mkdir ~/.config/gedent
cp $PWD/templates ~/.config/gedent/
cp $PWD/presets ~/.config/gedent/
cp $PWD/gedent.toml ~/.config/gedent/
```

## Configuration

`gedent` offers support for a per-project configuration file, it searches previous 
directories (until the user home folder) and if no config is found (a gedent.toml file) it uses the 
default config location (`~/.config/gedent` in linux).

Pairing the user defined config file with the power of [TERA templates](https://keats.github.io/tera/)
gives rise to a rich system of input generation.

## Templates

## Examples

