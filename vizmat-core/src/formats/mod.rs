use std::io::Cursor;

use crate::structure::StructureView;
use anyhow::Result;

mod pdb;
mod sdf;

pub(crate) const SUPPORTED_EXTENSIONS: &[&str] = &["xyz", "pdb", "sdf"];
pub(crate) const SUPPORTED_EXTENSIONS_HELP: &str = ".xyz, .pdb, or .sdf";

pub(crate) fn is_supported_extension(ext: &str) -> bool {
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    SUPPORTED_EXTENSIONS.contains(&normalized.as_str())
}

pub(crate) fn parse_structure_by_extension(ext: &str, content: &str) -> Result<StructureView> {
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    match normalized.as_str() {
        "xyz" => {
            let mut rd = Cursor::new(content.as_bytes());
            let inner = ccmat_babel::parse(&mut rd, "xyz")?;
            let s = StructureView { inner, bonds: None };

            Ok(s)
        }
        "pdb" => pdb::parse_pdb_content(content),
        "sdf" => sdf::parse_sdf_content(content),
        _ => Err(anyhow::anyhow!("Unsupported file extension")),
    }
}
