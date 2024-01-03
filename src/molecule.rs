#![allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Context, Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Molecule {
    filename: String,
    annotations: String,
    atoms: Vec<String>,
}

impl Molecule {
    fn new() -> Molecule {
        return Molecule {
            filename: "".to_string(),
            annotations: "".to_string(),
            atoms: Vec::new(),
        };
    }

    pub fn split(&self, index: usize) -> Result<(Molecule, Molecule), Error> {
        if index >= self.atoms.len() {
            anyhow::bail!("Index given bigger than size of molecule, exiting...")
        }
        let mut molecule1 = self.clone();
        let mut molecule2 = self.clone();

        molecule1.atoms = self.atoms[0..index].to_vec();
        molecule1.filename.push_str("_split1");
        molecule2.atoms = self.atoms[index..].to_vec();
        molecule2.filename.push_str("_split2");
        Ok((molecule1, molecule2))
    }

    // returns a vec because we support a file with multiple xyz
    // the check for atom length got kinda ugly.. see if there is some smarter way to do this
    pub fn from_xyz(xyz_path: &PathBuf) -> Result<Vec<Molecule>, Error> {
        let xyz_file = std::fs::read_to_string(xyz_path)?;
        let mut xyz_lines = xyz_file.lines().peekable();
        let mut molecules: Vec<Molecule> = vec![];
        let mut mol = Molecule::new();
        let mut natoms = 0;

        loop {
            if xyz_lines.peek().is_none() {
                if mol.atoms.len() != natoms {
                    anyhow::bail!(
                        "Expected {} atoms found {}, exiting...",
                        natoms,
                        mol.atoms.len()
                    )
                }
                molecules.push(mol.clone());
                break;
            }

            if xyz_lines.peek().unwrap().parse::<i64>().is_ok() {
                if !mol.atoms.is_empty() {
                    if mol.atoms.len() != natoms {
                        anyhow::bail!(
                            "Expected {} atoms found {}, exiting...",
                            natoms,
                            mol.atoms.len()
                        )
                    }
                    natoms -= natoms; // set to 0 again
                    molecules.push(mol.clone());
                }

                natoms += xyz_lines.next().unwrap().parse::<usize>()?;
                mol.annotations = xyz_lines.next().unwrap_or("").to_string();
                mol.atoms.clear();
            } else {
                mol.atoms.push(xyz_lines.next().unwrap().to_string());
            }
        }

        Ok(molecules)
    }
}
