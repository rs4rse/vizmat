use crate::structure::{Atom, Crystal};
use anyhow::{Context, Result};

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
            chain_id: None,
            res_name: None,
        };

        atoms.push(atom);
    }

    Ok(Crystal { atoms })
}

// Function to parse PDB file format from string content.
// Reads ATOM/HETATM coordinate records and extracts element + xyz.
pub(crate) fn parse_pdb_content(contents: &str) -> Result<Crystal> {
    let mut atoms = Vec::new();

    for line in contents.lines() {
        if !(line.starts_with("ATOM  ") || line.starts_with("HETATM")) {
            continue;
        }
        if line.len() < 54 {
            continue;
        }

        let x: f32 = line[30..38]
            .trim()
            .parse()
            .context("Failed to parse PDB x coordinate")?;
        let y: f32 = line[38..46]
            .trim()
            .parse()
            .context("Failed to parse PDB y coordinate")?;
        let z: f32 = line[46..54]
            .trim()
            .parse()
            .context("Failed to parse PDB z coordinate")?;

        let atom_name = if line.len() >= 16 {
            Some(line[12..16].trim().to_string())
        } else {
            None
        };
        let res_name = if line.len() >= 20 {
            let v = line[17..20].trim();
            (!v.is_empty()).then(|| v.to_string())
        } else {
            None
        };
        let chain_id = if line.len() >= 22 {
            let v = line[21..22].trim();
            (!v.is_empty()).then(|| v.to_string())
        } else {
            None
        };
        let mut element = if line.len() >= 78 {
            line[76..78].trim().to_string()
        } else {
            String::new()
        };

        if element.is_empty() {
            element = atom_name
                .as_deref()
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect::<String>();
        }

        if element.is_empty() {
            continue;
        }

        atoms.push(Atom {
            element,
            x,
            y,
            z,
            chain_id,
            res_name,
        });
    }

    if atoms.is_empty() {
        return Err(anyhow::anyhow!(
            "PDB file contains no ATOM/HETATM coordinates"
        ));
    }

    Ok(Crystal { atoms })
}
