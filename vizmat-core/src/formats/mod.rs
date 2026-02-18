use anyhow::Result;

use crate::structure::Crystal;

mod pdb;
mod sdf;
mod xyz;

pub(crate) const SUPPORTED_EXTENSIONS: &[&str] = &["xyz", "pdb", "sdf"];
pub(crate) const SUPPORTED_EXTENSIONS_HELP: &str = ".xyz, .pdb, or .sdf";

pub(crate) fn is_supported_extension(ext: &str) -> bool {
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    SUPPORTED_EXTENSIONS.contains(&normalized.as_str())
}

pub(crate) fn parse_structure_by_extension(ext: &str, contents: &str) -> Result<Crystal> {
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    match normalized.as_str() {
        "xyz" => xyz::parse_xyz_content(contents),
        "pdb" => pdb::parse_pdb_content(contents),
        "sdf" => sdf::parse_sdf_content(contents),
        _ => Err(anyhow::anyhow!("Unsupported file extension")),
    }
}
