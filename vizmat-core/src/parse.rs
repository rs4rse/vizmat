use crate::structure::Crystal;
use anyhow::{Context, Result};
use ccmat_core::{
    atomic_number_from_symbol, lattice_angstrom, math::Vector3, CrystalBuilder, Site,
};

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

    let mut sites = Vec::new();

    for (i, line) in lines.iter().skip(2).enumerate() {
        if i >= num_atoms {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue; // Skip malformed lines
        }

        let (x, y, z): (f64, f64, f64) = (
            parts[1].parse().context("Failed to parse x coordinate")?,
            parts[2].parse().context("Failed to parse y coordinate")?,
            parts[3].parse().context("Failed to parse z coordinate")?,
        );
        let position = Vector3([x.into(), y.into(), z.into()]);
        let sym = parts[0].to_string();
        let site = Site::new(position, atomic_number_from_symbol(&sym)?);

        sites.push(site);
    }

    // FIXME: use extxyz
    let lattice = lattice_angstrom![a = (5., 0., 0.), b = (0., 5., 0.), c = (0., 0., 5.),];
    let crystal = CrystalBuilder::new()
        .with_lattice(&lattice)
        .with_sites(sites)
        .build()?;

    Ok(Crystal { inner: crystal })
}
