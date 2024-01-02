use anyhow::{anyhow, Context, Error, Result};
use std::path::PathBuf;

// compiler warns that i dont "need" the atom type as im not actually doing anything with it
// but for know i think it makes the code more readable (molecules are made of atoms)
#[derive(Debug, Clone)]
struct Atom {
    _element: String,
    _coords: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Molecule {
    atoms: Vec<Atom>,
}

impl Molecule {
    fn new() -> Molecule {
        return Molecule { atoms: Vec::new() };
    }

    // returns a vec because we support a file with multiple xyz
    pub fn from_xyz(xyz_path: &PathBuf) -> Result<Vec<Molecule>, Error> {
        Ok(parse_xyz(xyz_path)?)
    }

    pub fn split(&self, index: usize) -> Result<(Molecule, Molecule), Error> {
        if index >= self.atoms.len() {
            anyhow::bail!("Index given bigger than size of molecule, exiting...")
        }
        let mut molecule1 = Molecule::new();
        let mut molecule2 = Molecule::new();

        molecule1.atoms = self.atoms[0..index].to_vec();
        molecule2.atoms = self.atoms[index..].to_vec();
        Ok((molecule1, molecule2))
    }
}

fn parse_atom(line: &str) -> Result<Atom, Error> {
    let line_split = line
        .trim()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let element = line_split[0];
    let coords: Result<Vec<f64>, Error> = line_split[1..]
        .iter()
        .map(|x| x.parse::<f64>().context("Cant parse float"))
        .collect();
    let atom = Atom {
        _element: element.to_string(),
        _coords: coords?,
    };
    Ok(atom)
}

fn is_natoms(peeked: &&str) -> bool {
    return peeked.parse::<i64>().is_ok();
}

// the check for atom length got kinda ugly.. see if there is some smarter way to do this
fn parse_xyz(xyz_path: &PathBuf) -> Result<Vec<Molecule>, Error> {
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

        if is_natoms(xyz_lines.peek().unwrap()) {
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
            let _comment = xyz_lines.next().unwrap();
            mol.atoms.clear();
        } else {
            mol.atoms.push(parse_atom(xyz_lines.next().unwrap())?);
        }
    }

    Ok(molecules)
}
