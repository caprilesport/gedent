use color_eyre::eyre::{eyre, Report as Error, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Atom {
    pub symbol: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Atom {
    fn from_line(line: &str) -> Result<Self, Error> {
        let mut parts = line.split_whitespace();
        let symbol = parts
            .next()
            .ok_or_else(|| eyre!("Missing element symbol"))?
            .to_string();
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
        Ok(Self { symbol, x, y, z })
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<4}{:14.8}{:14.8}{:14.8}",
            self.symbol, self.x, self.y, self.z
        )
    }
}

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Molecule {
    pub description: Option<String>,
    pub atoms: Vec<Atom>,
}

impl Molecule {
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
                symbol: "C".to_string(),
                x: -0.702_728_547,
                y: 0.0,
                z: -1.996_862_306,
            },
            Atom {
                symbol: "H".to_string(),
                x: -0.172_294_601,
                y: -0.951_333_822,
                z: -1.920_672_276,
            },
            Atom {
                symbol: "H".to_string(),
                x: 0.013_819_138,
                y: 0.821_859_802,
                z: -1.939_355_658,
            },
            Atom {
                symbol: "H".to_string(),
                x: -1.419_276_232,
                y: 0.083_844_265,
                z: -1.177_270_525,
            },
            Atom {
                symbol: "H".to_string(),
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
}
