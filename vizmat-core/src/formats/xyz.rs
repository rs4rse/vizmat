use anyhow::{Context, Result};

use crate::structure::{Molecule, Site};

pub(super) fn parse_xyz_content(contents: &str) -> Result<Molecule> {
    let lines = contents.lines().collect::<Vec<&str>>();

    if lines.len() < 2 {
        return Err(anyhow::anyhow!("XYZ file too short"));
    }

    let num_atoms: usize = lines[0]
        .trim()
        .parse()
        .context("Failed to parse number of atoms")?;

    let _comment_line = lines[1].trim();

    let mut sites = Vec::new();

    for (i, line) in lines.iter().skip(2).enumerate() {
        if i >= num_atoms {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let atom = Site {
            element: parts[0].to_string(),
            x: parts[1].parse().context("Failed to parse x coordinate")?,
            y: parts[2].parse().context("Failed to parse y coordinate")?,
            z: parts[3].parse().context("Failed to parse z coordinate")?,
            chain_id: None,
            res_name: None,
        };

        sites.push(atom);
    }

    let mol = Molecule::new_from_sites(&sites);
    Ok(mol)
}
