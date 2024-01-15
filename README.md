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

## Configuration

`gedent` offers support for a per-project configuration file, it searches previous 
directories (until the user home folder) and if no config is found (a gedent.toml file) it uses the 
default config location (`~/.config/gedent` in linux).

Pairing the user defined config file with the power of [TERA templates](https://keats.github.io/tera/)
gives rise to a rich system of input generation.

## Templates

## Examples

