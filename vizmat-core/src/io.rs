// io.rs
use bevy::prelude::*;
use ccmat_core::{atomic_number, sites_cart_coord, MoleculeBuilder};
use std::path::PathBuf;

use crate::formats::{
    is_supported_extension, parse_structure_by_extension, SUPPORTED_EXTENSIONS_HELP,
};
use crate::structure::StructureView;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileStatusKind {
    Info,
    Success,
    Error,
}

// System to load default structure data
pub(crate) fn load_default_structure(mut commands: Commands) {
    println!("Loading default water molecule structure");

    // TODO: this should be able to be initialized with Molecule::new directly
    let sites = sites_cart_coord![
        (0.0, 0.0, 0.0), atomic_number!(O);
        (0.757, 0.587, 0.0), atomic_number!(H);
        (-0.757, 0.587, 0.0), atomic_number!(H);
    ];
    let mol = MoleculeBuilder::new().with_sites(sites).build_uncheck();
    let s = StructureView {
        inner: ccmat_core::Structure::Molecule(mol),
        bonds: None,
    };

    commands.insert_resource(s);
}

// Resource to handle file drag and drop
#[derive(Resource)]
pub(crate) struct FileDragDrop {
    pub(crate) dragged_file: Option<PathBuf>,
    pub(crate) loaded_structure: Option<StructureView>,
    pub(crate) status_message: String,
    pub(crate) status_kind: FileStatusKind,
}

impl Default for FileDragDrop {
    fn default() -> Self {
        Self {
            dragged_file: None,
            loaded_structure: None,
            status_message: format!("Drop {} file", SUPPORTED_EXTENSIONS_HELP),
            status_kind: FileStatusKind::Info,
        }
    }
}

// System to handle file drag and drop events
pub(crate) fn handle_file_drag_drop(
    mut drag_drop_events: EventReader<bevy::window::FileDragAndDrop>,
    mut file_drag_drop: ResMut<FileDragDrop>,
) {
    for event in drag_drop_events.read() {
        match event {
            bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } => {
                println!("File dropped: {:?}", path_buf);

                if let Some(extension) = path_buf.extension() {
                    let ext = extension.to_string_lossy().to_ascii_lowercase();
                    if is_supported_extension(&ext) {
                        file_drag_drop.dragged_file = Some(path_buf.clone());
                        if let Some(name) = path_buf.file_name().and_then(|n| n.to_str()) {
                            file_drag_drop.status_message = format!("Loading: {name}");
                            file_drag_drop.status_kind = FileStatusKind::Info;
                        }
                    } else {
                        println!(
                            "Unsupported file type. Please drop a {} file.",
                            SUPPORTED_EXTENSIONS_HELP
                        );
                        file_drag_drop.status_message = format!(
                            "Unsupported file. Please drop {}",
                            SUPPORTED_EXTENSIONS_HELP
                        );
                        file_drag_drop.status_kind = FileStatusKind::Error;
                    }
                }
            }
            bevy::window::FileDragAndDrop::HoveredFile { path_buf, .. } => {
                println!("File hovered: {:?}", path_buf);
            }
            bevy::window::FileDragAndDrop::HoveredFileCanceled { .. } => {
                println!("File hover canceled");
            }
        }
    }
}

// XXX: this only works for non-wasm env
//
// System to load crystal from dropped file
pub(crate) fn load_dropped_file(
    mut file_drag_drop: ResMut<FileDragDrop>,
    mut last_loaded_path: Local<Option<PathBuf>>,
) {
    if let Some(path) = file_drag_drop.dragged_file.clone() {
        if last_loaded_path
            .as_ref()
            .is_none_or(|loaded_path| loaded_path != &path)
        {
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    let ext = path
                        .extension()
                        .map(|s| s.to_string_lossy().to_ascii_lowercase());
                    let parsed = match ext.as_deref() {
                        Some(ext) => parse_structure_by_extension(ext, &contents),
                        _ => Err(anyhow::anyhow!("Unsupported file extension")),
                    };
                    match parsed {
                        Ok(s) => {
                            println!("Successfully loaded structure from: {:?}", path.display());
                            let atom_count = s.nsites();
                            let file_bond_count = s.bonds.as_ref().map_or(0, Vec::len);
                            file_drag_drop.loaded_structure = Some(s);
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("structure");
                            file_drag_drop.status_message = if file_bond_count > 0 {
                                format!(
                                    "Loaded: {name} ({atom_count} atoms, {file_bond_count} file bonds)"
                                )
                            } else {
                                format!("Loaded: {name} ({atom_count} atoms)")
                            };
                            file_drag_drop.status_kind = FileStatusKind::Success;
                            *last_loaded_path = Some(path);
                        }
                        Err(e) => {
                            eprintln!("Failed to parse structure file: {e}");
                            file_drag_drop.status_message = format!("Parse error: {e}");
                            file_drag_drop.status_kind = FileStatusKind::Error;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read file: {e}");
                    file_drag_drop.status_message = format!("Read error: {e}");
                    file_drag_drop.status_kind = FileStatusKind::Error;
                }
            }
        }
    }
}

// System to update structure resource when new file is loaded
pub(crate) fn update_structure_from_file(
    mut commands: Commands,
    file_drag_drop: Res<FileDragDrop>,
    current_structure: Option<Res<StructureView>>,
) {
    if let Some(s) = &file_drag_drop.loaded_structure {
        // Only update if this is a new structure
        if let Some(current) = current_structure {
            let current_bond_count = current.bonds.as_ref().map_or(0, Vec::len);
            let new_bond_count = s.bonds.as_ref().map_or(0, Vec::len);
            if current.nsites() != s.nsites() || current_bond_count != new_bond_count {
                commands.insert_resource(s.clone());
                if new_bond_count > 0 {
                    println!(
                        "Structure updated with {} atoms and {} file bonds",
                        s.nsites(),
                        new_bond_count
                    );
                } else {
                    println!("Crystal updated with {} atoms", s.nsites());
                }
            }
        } else {
            commands.insert_resource(s.clone());
            let new_bond_count = s.bonds.as_ref().map_or(0, Vec::len);
            if new_bond_count > 0 {
                println!(
                    "Structure loaded with {} atoms and {} file bonds",
                    s.nsites(),
                    new_bond_count
                );
            } else {
                println!("Structure loaded with {} atoms", s.nsites());
            }
        }
    }
}
