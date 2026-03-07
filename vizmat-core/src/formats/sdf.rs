use anyhow::{Context, Result};
use ccmat_core::{atomic_number_from_symbol, math::Vector3, Angstrom, MoleculeBuilder};

use crate::structure::{Site, StructureView};

pub(super) fn parse_sdf_content(content: &str) -> Result<StructureView> {
    let first_record = content.split("$$$$").next().unwrap_or(content);
    let lines: Vec<&str> = first_record.lines().collect();

    let Some((counts_line_index, counts_line)) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.contains("V2000") || line.contains("V3000"))
    else {
        return Err(anyhow::anyhow!(
            "SDF file does not contain a MOL counts line"
        ));
    };

    if counts_line.contains("V3000") {
        return Err(anyhow::anyhow!("SDF V3000 is not supported yet"));
    }

    let nsites = parse_count_field(counts_line, 0..3)
        .or_else(|_| parse_count_tokens(counts_line, 0))
        .context("Failed to parse SDF atom count")?;
    let bond_count = parse_count_field(counts_line, 3..6)
        .or_else(|_| parse_count_tokens(counts_line, 1))
        .context("Failed to parse SDF bond count")?;

    let atom_block_start = counts_line_index + 1;
    let mut sites = Vec::with_capacity(nsites);

    for atom_idx in 0..nsites {
        let line = lines
            .get(atom_block_start + atom_idx)
            .copied()
            .context("SDF atom block is shorter than declared atom count")?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(anyhow::anyhow!("Malformed SDF atom line: {line}"));
        }

        sites.push(Site {
            x: parts[0]
                .parse()
                .context("Failed to parse SDF x coordinate")?,
            y: parts[1]
                .parse()
                .context("Failed to parse SDF y coordinate")?,
            z: parts[2]
                .parse()
                .context("Failed to parse SDF z coordinate")?,
            element: parts[3].to_string(),
            chain_id: None,
            res_name: None,
        });
    }

    let bond_block_start = atom_block_start + nsites;
    let mut bonds = Vec::with_capacity(bond_count);
    for bond_idx in 0..bond_count {
        let line = lines
            .get(bond_block_start + bond_idx)
            .copied()
            .context("SDF bond block is shorter than declared bond count")?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Malformed SDF bond line: {line}"));
        }

        let a_raw = parts[0]
            .parse::<usize>()
            .context("Failed to parse SDF bond atom index A")?;
        let b_raw = parts[1]
            .parse::<usize>()
            .context("Failed to parse SDF bond atom index B")?;
        let order = parts[2]
            .parse::<u8>()
            .context("Failed to parse SDF bond order")?;

        if a_raw == 0 || b_raw == 0 {
            continue;
        }
        let a = a_raw - 1;
        let b = b_raw - 1;
        if a >= nsites || b >= nsites || a == b {
            continue;
        }

        bonds.push(crate::structure::Bond { a, b, order });
    }

    if sites.is_empty() {
        return Err(anyhow::anyhow!("SDF file contains no atoms"));
    }

    let sites = sites
        .iter()
        .map(|s| {
            // TODO: I should not rely directly on ccmat_core API call.
            ccmat_core::SiteCartesian::new(
                Vector3([
                    Angstrom::from(f64::from(s.x)),
                    Angstrom::from(f64::from(s.y)),
                    Angstrom::from(f64::from(s.z)),
                ]),
                atomic_number_from_symbol(&s.element).expect("not a valid symbol"),
            )
        })
        .collect::<Vec<_>>();
    let mol = MoleculeBuilder::new().with_sites(sites).build_uncheck();
    let bonds = if bond_count > 0 { Some(bonds) } else { None };
    let s = StructureView {
        inner: ccmat_core::Structure::Molecule(mol),
        bonds,
        chain_ids: None,
        residues: None,
    };

    Ok(s)
}

fn parse_count_field(line: &str, range: std::ops::Range<usize>) -> Result<usize> {
    let value = line
        .get(range)
        .context("Missing count field in SDF counts line")?
        .trim()
        .parse::<usize>()
        .context("Invalid SDF count field")?;
    Ok(value)
}

fn parse_count_tokens(line: &str, idx: usize) -> Result<usize> {
    let token = line
        .split_whitespace()
        .nth(idx)
        .context("Missing tokenized count in SDF counts line")?;
    let value = token
        .parse::<usize>()
        .context("Invalid tokenized SDF count")?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::parse_sdf_content;

    #[test]
    fn parses_simple_v2000_sdf() {
        let sdf = "\
Naproxen
  generated

  3  2  0  0  0  0            999 V2000
   -0.7500    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.7500    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
    1.5000    1.2000    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  2  0  0  0  0
  2  3  1  0  0  0  0
M  END
$$$$
";
        let mol = parse_sdf_content(sdf).expect("expected sdf parse success");
        assert_eq!(mol.nsites(), 3);
        assert_eq!(mol.sites()[0].element, "C");
        assert_eq!(mol.sites()[1].element, "O");
        let bonds = mol.bonds.expect("expected parsed sdf bonds");
        assert_eq!(bonds.len(), 2);
        assert_eq!(bonds[0].order, 2);
        assert_eq!(bonds[1].order, 1);
    }

    #[test]
    fn treats_zero_bond_count_as_no_file_bonds() {
        let sdf = "\
SingleAtom
  generated

  1  0  0  0  0  0            999 V2000
    0.0000    0.0000    0.0000 He  0  0  0  0  0  0  0  0  0  0  0  0
M  END
$$$$
";
        let mol = parse_sdf_content(sdf).expect("expected sdf parse success");
        assert_eq!(mol.nsites(), 1);
        assert!(
            mol.bonds.is_none(),
            "zero-bond SDF should be treated as no explicit bonds"
        );
    }
}
