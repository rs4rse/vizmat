use crate::structure::{Atom, Crystal, Lattice};
use anyhow::{Context, Result};
use bevy::math::Vec3;

// Function to parse XYZ file format from string content
pub(crate) fn parse_xyz_content(contents: &str) -> Result<Crystal> {
    let lines = contents.lines().collect::<Vec<&str>>();

    if lines.len() < 2 {
        return Err(anyhow::anyhow!("XYZ file too short"));
    }

    // First line should contain the number of atoms
    let num_atoms: usize = lines[0]
        .trim()
        .parse()
        .context("Failed to parse number of atoms")?;

    // Second line may contain comment or extended XYZ properties
    let _comment_line = lines[1].trim();

    // Parse extended XYZ properties if present (basic implementation)
    // For now, we'll focus on the basic XYZ format

    let mut atoms = Vec::new();

    for (i, line) in lines.iter().skip(2).enumerate() {
        if i >= num_atoms {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue; // Skip malformed lines
        }

        let atom = Atom {
            element: parts[0].to_string(),
            x: parts[1].parse().context("Failed to parse x coordinate")?,
            y: parts[2].parse().context("Failed to parse y coordinate")?,
            z: parts[3].parse().context("Failed to parse z coordinate")?,
        };

        atoms.push(atom);
    }

    // FIXME: use extxyz
    let lattice = Lattice::new(
        Vec3::new(5., 0., 0.),
        Vec3::new(0., 5., 0.),
        Vec3::new(0., 0., 5.),
    );

    Ok(Crystal { lattice, atoms })
}
