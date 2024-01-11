use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Molecule {
    pub filename: String,
    pub annotations: String,
    pub atoms: Vec<String>,
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
        molecule1.filename.push_str("_split_1");
        molecule2.atoms = self.atoms[index..].to_vec();
        molecule2.filename.push_str("_split_2");
        println!("{:?}, {:?}", molecule1.atoms, molecule2.atoms);
        Ok((molecule1, molecule2))
    }

    // returns a vec because we support a file with multiple xyz
    // the check for atom length got kinda ugly.. see if there is some smarter way to do this
    pub fn from_xyz(mut xyz_path: PathBuf) -> Result<Vec<Molecule>, Error> {
        let xyz_file = std::fs::read_to_string(&xyz_path)?;
        xyz_path.set_extension("");
        let name = String::from(
            xyz_path
                .to_str()
                .ok_or(anyhow!("Cant convert path of xyz file to name"))?,
        );
        let mut xyz_lines = xyz_file.lines().peekable();
        let mut molecules: Vec<Molecule> = vec![];
        let mut mol = Molecule::new();
        mol.filename = name.clone();
        let mut natoms = 0;
        let mut counter = 0;

        loop {
            if xyz_lines.peek().is_none() {
                if mol.atoms.len() != natoms {
                    anyhow::bail!(
                        "Expected {} atoms found {}, exiting...",
                        natoms,
                        mol.atoms.len()
                    )
                }
                match counter {
                    0 => (),
                    _ => {
                        mol.filename = [name.clone(), counter.clone().to_string()].join("_");
                    }
                };
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
                    mol.filename = [name.clone(), counter.clone().to_string()].join("_");
                    molecules.push(mol.clone());
                    counter += 1;
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn xyz_parse_works() {
        let test_ch4 = "5
symmetry c1
C       -0.702728547      0.000000000     -1.996862306
H       -0.172294601     -0.951333822     -1.920672276
H        0.013819138      0.821859802     -1.939355658
H       -1.419276232      0.083844265     -1.177270525
H       -1.233162492      0.045629756     -2.950150766"
            .to_string();
        let test_ch4_h2o = "5
symmetry c1
C       -0.702728547      0.000000000     -1.996862306
H       -0.172294601     -0.951333822     -1.920672276
H        0.013819138      0.821859802     -1.939355658
H       -1.419276232      0.083844265     -1.177270525
H       -1.233162492      0.045629756     -2.950150766
3
symmetry c1
O       -1.537653553      0.000000000     -2.881263893
H       -1.537653553      0.759337000     -2.285220893
H       -1.537653553     -0.759337000     -2.285220893"
            .to_string();

        // create dummy files to load
        std::fs::write("./ch4.xyz", test_ch4).unwrap();
        std::fs::write("./ch4_h2o.xyz", test_ch4_h2o).unwrap();

        let ch4 = Molecule {
            filename: "./ch4".to_string(),
            annotations: "symmetry c1".to_string(),
            atoms: vec![
                "C       -0.702728547      0.000000000     -1.996862306".to_string(),
                "H       -0.172294601     -0.951333822     -1.920672276".to_string(),
                "H        0.013819138      0.821859802     -1.939355658".to_string(),
                "H       -1.419276232      0.083844265     -1.177270525".to_string(),
                "H       -1.233162492      0.045629756     -2.950150766".to_string(),
            ],
        };
        let h2o = Molecule {
            filename: "./ch4_h2o_1".to_string(),
            annotations: "symmetry c1".to_string(),
            atoms: vec![
                "O       -1.537653553      0.000000000     -2.881263893".to_string(),
                "H       -1.537653553      0.759337000     -2.285220893".to_string(),
                "H       -1.537653553     -0.759337000     -2.285220893".to_string(),
            ],
        };
        let mut ch4_2 = ch4.clone();
        ch4_2.filename = "./ch4_h2o_0".to_string();

        match Molecule::from_xyz(PathBuf::from("./ch4.xyz")) {
            Ok(mol) => assert_eq!(mol, vec![ch4]),
            Err(_) => core::panic!("Failes test ch4"),
        };

        match Molecule::from_xyz(PathBuf::from("./ch4_h2o.xyz")) {
            Ok(mol) => assert_eq!(mol, vec![ch4_2, h2o]),
            Err(_) => core::panic!("Failed test ch4 h2o"),
        };

        std::fs::remove_file("./ch4.xyz").unwrap();
        std::fs::remove_file("./ch4_h2o.xyz").unwrap();
    }

    #[test]
    fn molecule_split_works() {
        let ch4 = Molecule {
            filename: "./ch4".to_string(),
            annotations: "symmetry c1".to_string(),
            atoms: vec![
                "C       -0.702728547      0.000000000     -1.996862306".to_string(),
                "H       -0.172294601     -0.951333822     -1.920672276".to_string(),
                "H        0.013819138      0.821859802     -1.939355658".to_string(),
                "H       -1.419276232      0.083844265     -1.177270525".to_string(),
                "H       -1.233162492      0.045629756     -2.950150766".to_string(),
            ],
        };

        let ch3 = Molecule {
            filename: "./ch4_split_1".to_string(),
            annotations: "symmetry c1".to_string(),
            atoms: vec![
                "C       -0.702728547      0.000000000     -1.996862306".to_string(),
                "H       -0.172294601     -0.951333822     -1.920672276".to_string(),
                "H        0.013819138      0.821859802     -1.939355658".to_string(),
                "H       -1.419276232      0.083844265     -1.177270525".to_string(),
            ],
        };
        let h = Molecule {
            filename: "./ch4_split_2".to_string(),
            annotations: "symmetry c1".to_string(),
            atoms: vec!["H       -1.233162492      0.045629756     -2.950150766".to_string()],
        };
        match ch4.split(4) {
            Ok(mol) => assert_eq!(mol, (ch3, h), "Failed to split molecule"),
            Err(_) => core::panic!(),
        }
    }
}
