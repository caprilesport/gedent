# gedent

Gedent is an input generator for computational chemistry workflows. 
It strives to be simple and as general as possible, while contained in the
boundaries of the quantum chemistry research. 

As the needs for molecular dynamics software is much more diverse, gedent does
not aim to provide capabilites for this kind of software.

gedent aims to provide a paradigm of configurations and templates for software 
such as [XTB](), [orca](), 
[ADF](), [Gaussian](), [NWChem]() and similar chemistry software
suites. Although it aims to support such software and was though with this
use case in mind, it is a template CLI combined with a user defined configuration,
so if you find another use for it, feel free to open a pull request with 
functionalities that support your needs.

## Installation

## Configuration

gedent offers support for a per-project configuration file, it searches previous 
directories (until the user home folder) and if no config is found it uses the 
default config location (~/.config/gedent).
The config file accepts any keys, and is user defined. A default config 
file is provided by gedent with sane defaults for XTB and orca (for now).

Pairing the user defined config file with the power of [TERA templates]()
gives rise to a rich system of input generation.

## Templates

## Examples

