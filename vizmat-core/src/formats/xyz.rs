use anyhow::{Context, Result};

use crate::structure::{Atom, Crystal};

pub(super) fn parse_xyz_content(contents: &str) -> Result<Crystal> {
    let lines = contents.lines().collect::<Vec<&str>>();

    if lines.len() < 2 {
        return Err(anyhow::anyhow!("XYZ file too short"));
    }

    let num_atoms: usize = lines[0]
        .trim()
        .parse()
        .context("Failed to parse number of atoms")?;

    let _comment_line = lines[1].trim();

    let mut atoms = Vec::new();

    for (i, line) in lines.iter().skip(2).enumerate() {
        if i >= num_atoms {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let atom = Atom {
            element: parts[0].to_string(),
            x: parts[1].parse().context("Failed to parse x coordinate")?,
            y: parts[2].parse().context("Failed to parse y coordinate")?,
            z: parts[3].parse().context("Failed to parse z coordinate")?,
            chain_id: None,
            res_name: None,
        };

        atoms.push(atom);
    }

    Ok(Crystal { atoms, bonds: None })
}
