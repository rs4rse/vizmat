use anyhow::{Context, Result};

use crate::structure::{Molecule, Site};

pub(super) fn parse_pdb_content(contents: &str) -> Result<Molecule> {
    let mut sites = Vec::new();

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

        sites.push(Site {
            element,
            x,
            y,
            z,
            chain_id,
            res_name,
        });
    }

    if sites.is_empty() {
        return Err(anyhow::anyhow!(
            "PDB file contains no ATOM/HETATM coordinates"
        ));
    }

    let mol = Molecule::new_from_sites(&sites);

    Ok(mol)
}

#[cfg(test)]
mod tests {
    use super::parse_pdb_content;

    #[test]
    fn parses_atom_and_hetatm_records() {
        let pdb = "\
HEADER    TEST PDB
ATOM      1  N   MET A   1      11.104  13.207   8.447  1.00 20.00           N
ATOM      2  CA  MET A   1      12.560  13.482   8.615  1.00 20.00           C
HETATM    3  O   HOH B 101       9.301  10.200   7.100  1.00 10.00           O
END
";

        let mol = parse_pdb_content(pdb).expect("expected pdb parse success");
        assert_eq!(mol.sites().len(), 3);

        assert_eq!(mol.sites()[0].element, "N");
        assert_eq!(mol.sites()[0].chain_id.as_deref(), Some("A"));
        assert_eq!(mol.sites()[0].res_name.as_deref(), Some("MET"));

        assert_eq!(mol.sites()[2].element, "O");
        assert_eq!(mol.sites()[2].chain_id.as_deref(), Some("B"));
        assert_eq!(mol.sites()[2].res_name.as_deref(), Some("HOH"));
    }

    #[test]
    fn falls_back_to_atom_name_when_element_column_missing() {
        let pdb = "\
ATOM      1  CL  LIG A   1       1.000   2.000   3.000  1.00 20.00
END
";

        let mol = parse_pdb_content(pdb).expect("expected pdb parse success");
        assert_eq!(mol.sites().len(), 1);
        assert_eq!(mol.sites()[0].element, "CL");
        assert_eq!(mol.sites()[0].chain_id.as_deref(), Some("A"));
        assert_eq!(mol.sites()[0].res_name.as_deref(), Some("LIG"));
    }

    #[test]
    fn ignores_non_atom_and_too_short_records_then_errors_if_empty() {
        let pdb = "\
HEADER    NOTHING USEFUL
ATOM
REMARK    text
END
";

        match parse_pdb_content(pdb) {
            Ok(_) => panic!("expected empty-atoms error"),
            Err(err) => assert!(
                err.to_string().contains("no ATOM/HETATM coordinates"),
                "unexpected error: {err}"
            ),
        }
    }

    #[test]
    fn errors_on_invalid_coordinates() {
        let pdb = "\
ATOM      1  N   MET A   1      XX.104  13.207   8.447  1.00 20.00           N
END
";

        match parse_pdb_content(pdb) {
            Ok(_) => panic!("expected coordinate parse error"),
            Err(err) => assert!(
                err.to_string().contains("PDB x coordinate"),
                "unexpected error: {err}"
            ),
        }
    }
}
