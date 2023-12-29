// #![allow(dead_code, unused_variables, unused_imports)]

use anyhow::{anyhow, Context, Error, Result};
use std::path::PathBuf;

#[derive(Debug)]
struct Atom {
    element: String,
    coords: Vec<f32>,
}

#[derive(Debug)]
pub struct Molecule {
    atoms: Vec<Atom>,
}

impl Molecule {
    pub fn new() -> Molecule {
        return Molecule { atoms: Vec::new() };
    }

    pub fn from_xyz(xyz_path: &PathBuf) -> Result<Molecule, Error> {
        Ok(parse_xyz(xyz_path)?)
    }
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn parse_xyz(xyz_path: &PathBuf) -> Result<Molecule, Error> {
    let xyz_file = std::fs::read_to_string(xyz_path)?;
    let xyz_splitted_lines: Vec<_> = xyz_file
        .lines()
        .map(|line| {
            line.trim()
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect();
    let n: usize = xyz_splitted_lines[0][0].parse()?;
    let atoms = &xyz_splitted_lines[2..];

    let mut molecule = Molecule::new();
    // is there a way to remove this for loop? python old habits
    for line in atoms {
        let element = line[0].to_string();
        let coords: Result<Vec<f32>, Error> = line[1..]
            .iter()
            .map(|x| x.parse::<f32>().context("Cant parse float"))
            .collect();

        // this boilerplate is also kinda ugly, need to study more
        // to learn to deal with results in iterators
        let coords = match coords {
            Ok(coord) => coord,
            _ => {
                anyhow::bail!("Failed to parse a float in xyz coordinates, exiting...")
            }
        };
        molecule.atoms.push(Atom { element, coords });

        if !n.eq(&molecule.atoms.len()) {
            anyhow::bail!("Expected {} atoms, found {}.", n, &molecule.atoms.len())
        }
    }

    Ok(molecule)
}
