use crate::elements::Element;
use color_eyre::eyre::{eyre, Report as Error, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;

/// A single atom with its element and Cartesian coordinates (Å).
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Atom {
    /// Element identity.
    pub element: Element,
    /// x coordinate in Å.
    pub x: f64,
    /// y coordinate in Å.
    pub y: f64,
    /// z coordinate in Å.
    pub z: f64,
}

impl Atom {
    fn from_line(line: &str) -> Result<Self, Error> {
        let mut parts = line.split_whitespace();
        let element = parts
            .next()
            .ok_or_else(|| eyre!("Missing element symbol"))?
            .parse::<Element>()
            .wrap_err("Unknown element symbol")?;
        let x = parts
            .next()
            .ok_or_else(|| eyre!("Missing x coordinate"))?
            .parse::<f64>()
            .wrap_err("x coordinate is not a valid float")?;
        let y = parts
            .next()
            .ok_or_else(|| eyre!("Missing y coordinate"))?
            .parse::<f64>()
            .wrap_err("y coordinate is not a valid float")?;
        let z = parts
            .next()
            .ok_or_else(|| eyre!("Missing z coordinate"))?
            .parse::<f64>()
            .wrap_err("z coordinate is not a valid float")?;
        Ok(Self { element, x, y, z })
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<4}{:14.8}{:14.8}{:14.8}",
            self.element, self.x, self.y, self.z
        )
    }
}

/// A molecule parsed from an XYZ file.
///
/// Serialized to JSON and injected into the Tera context as `Molecule`
/// when an xyz file is provided to `gedent gen`.
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Molecule {
    /// Comment line from the xyz file (line 2). `None` if the line is blank.
    pub description: Option<String>,
    /// All atoms in file order.
    pub atoms: Vec<Atom>,
}

impl Molecule {
    /// Parse a single XYZ block from a buffered reader.
    ///
    /// The format is:
    /// ```text
    /// <natoms>
    /// <description or blank>
    /// <element> <x> <y> <z>
    /// ...
    /// ```
    ///
    /// Unknown element symbols error at parse time. Leading blank lines before
    /// the atom count are skipped.
    pub fn from_reader(reader: impl BufRead) -> Result<Self, Error> {
        let lines: Vec<String> = reader
            .lines()
            .collect::<std::io::Result<Vec<_>>>()
            .wrap_err("Failed to read xyz content")?;

        let mut iter = lines.iter().map(String::as_str);

        // skip any leading blank lines, then read atom count
        let natoms: usize = iter
            .by_ref()
            .find(|l| !l.trim().is_empty())
            .ok_or_else(|| eyre!("xyz content is empty"))?
            .trim()
            .parse()
            .wrap_err("First non-blank line must be an integer atom count")?;

        // description is always the very next line, even if blank
        let description_line = iter
            .next()
            .ok_or_else(|| eyre!("xyz content is missing a description line"))?;
        let description = if description_line.trim().is_empty() {
            None
        } else {
            Some(description_line.to_string())
        };

        // read exactly natoms atom lines, skipping any blank lines
        let mut atoms = Vec::with_capacity(natoms);
        for i in 0..natoms {
            let line = iter
                .by_ref()
                .find(|l| !l.trim().is_empty())
                .ok_or_else(|| eyre!("Expected {} atoms but found only {}", natoms, i))?;
            atoms.push(Atom::from_line(line).wrap_err(format!(
                "Failed to parse atom {} from: \"{}\"",
                i + 1,
                line
            ))?);
        }

        Ok(Self { description, atoms })
    }

    /// Open an xyz file at `path` and parse it into a `Molecule`.
    pub fn from_xyz(path: &PathBuf) -> Result<Self, Error> {
        let file = std::fs::File::open(path)
            .wrap_err(format!("Failed to open xyz file {}", path.display()))?;
        Self::from_reader(std::io::BufReader::new(file))
            .wrap_err(format!("Failed to parse xyz file {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const CH4_XYZ: &str = "5\nsymmetry c1\n\
        C       -0.702728547      0.000000000     -1.996862306\n\
        H       -0.172294601     -0.951333822     -1.920672276\n\
        H        0.013819138      0.821859802     -1.939355658\n\
        H       -1.419276232      0.083844265     -1.177270525\n\
        H       -1.233162492      0.045629756     -2.950150766";

    fn ch4_atoms() -> Vec<Atom> {
        vec![
            Atom {
                element: Element::C,
                x: -0.702_728_547,
                y: 0.0,
                z: -1.996_862_306,
            },
            Atom {
                element: Element::H,
                x: -0.172_294_601,
                y: -0.951_333_822,
                z: -1.920_672_276,
            },
            Atom {
                element: Element::H,
                x: 0.013_819_138,
                y: 0.821_859_802,
                z: -1.939_355_658,
            },
            Atom {
                element: Element::H,
                x: -1.419_276_232,
                y: 0.083_844_265,
                z: -1.177_270_525,
            },
            Atom {
                element: Element::H,
                x: -1.233_162_492,
                y: 0.045_629_756,
                z: -2.950_150_766,
            },
        ]
    }

    #[test]
    fn xyz_parse_works() {
        let mol = Molecule::from_reader(Cursor::new(CH4_XYZ)).unwrap();
        assert_eq!(mol.description, Some("symmetry c1".to_string()));
        assert_eq!(mol.atoms, ch4_atoms());
    }

    #[test]
    fn xyz_parse_trailing_blank_lines() {
        let input = format!("{CH4_XYZ}\n\n\n");
        let mol = Molecule::from_reader(Cursor::new(input)).unwrap();
        assert_eq!(mol.atoms, ch4_atoms());
    }

    #[test]
    fn xyz_parse_insufficient_atoms_errors() {
        let input = "10\nsymmetry c1\nC  0.0  0.0  0.0";
        assert!(
            Molecule::from_reader(Cursor::new(input)).is_err(),
            "Expected error when atom count exceeds available lines"
        );
    }

    #[test]
    fn xyz_parse_empty_description() {
        let input = "1\n\nC  0.0  0.0  0.0";
        let mol = Molecule::from_reader(Cursor::new(input)).unwrap();
        assert_eq!(mol.description, None);
    }

    #[test]
    fn xyz_parse_unknown_element_errors() {
        let input = "1\n\nXX  0.0  0.0  0.0";
        assert!(
            Molecule::from_reader(Cursor::new(input)).is_err(),
            "Expected error for unknown element symbol"
        );
    }

    #[test]
    fn atom_display_format() {
        let atom = Atom {
            element: Element::C,
            x: 0.0,
            y: 1.5,
            z: -2.0,
        };
        let s = atom.to_string();
        // {:<4} element symbol + three {:14.8} coordinates
        assert!(s.starts_with("C   "));
        assert!(s.contains("  0.00000000"));
        assert!(s.contains("  1.50000000"));
        assert!(s.contains(" -2.00000000"));
    }

    #[test]
    fn xyz_parse_case_insensitive_elements() {
        let input = "1\n\nfe  0.0  0.0  0.0";
        let mol = Molecule::from_reader(Cursor::new(input)).unwrap();
        assert_eq!(mol.atoms[0].element, Element::Fe);
    }
}
