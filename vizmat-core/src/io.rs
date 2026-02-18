// io.rs
use bevy::prelude::*;
use std::path::PathBuf;

use crate::formats::{
    is_supported_extension, parse_structure_by_extension, SUPPORTED_EXTENSIONS_HELP,
};
use crate::structure::{Atom, Crystal};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileStatusKind {
    Info,
    Success,
    Error,
}

// System to load default crystal data
pub(crate) fn load_default_crystal(mut commands: Commands) {
    println!("Loading default water molecule structure");

    let crystal = Crystal {
        atoms: vec![
            Atom {
                element: "O".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                chain_id: None,
                res_name: None,
            },
            Atom {
                element: "H".to_string(),
                x: 0.757,
                y: 0.587,
                z: 0.0,
                chain_id: None,
                res_name: None,
            },
            Atom {
                element: "H".to_string(),
                x: -0.757,
                y: 0.587,
                z: 0.0,
                chain_id: None,
                res_name: None,
            },
        ],
        bonds: None,
    };

    commands.insert_resource(crystal);
}

// Resource to handle file drag and drop
#[derive(Resource)]
pub(crate) struct FileDragDrop {
    pub(crate) dragged_file: Option<PathBuf>,
    pub(crate) loaded_crystal: Option<Crystal>,
    pub(crate) status_message: String,
    pub(crate) status_kind: FileStatusKind,
}

impl Default for FileDragDrop {
    fn default() -> Self {
        Self {
            dragged_file: None,
            loaded_crystal: None,
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
                        Ok(crystal) => {
                            println!("Successfully loaded crystal from: {:?}", path);
                            let atom_count = crystal.atoms.len();
                            let file_bond_count = crystal.bonds.as_ref().map_or(0, Vec::len);
                            file_drag_drop.loaded_crystal = Some(crystal);
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
                            eprintln!("Failed to parse structure file: {}", e);
                            file_drag_drop.status_message = format!("Parse error: {e}");
                            file_drag_drop.status_kind = FileStatusKind::Error;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);
                    file_drag_drop.status_message = format!("Read error: {e}");
                    file_drag_drop.status_kind = FileStatusKind::Error;
                }
            }
        }
    }
}

// System to update crystal resource when new file is loaded
pub(crate) fn update_crystal_from_file(
    mut commands: Commands,
    file_drag_drop: Res<FileDragDrop>,
    current_crystal: Option<Res<Crystal>>,
) {
    if let Some(crystal) = &file_drag_drop.loaded_crystal {
        // Only update if this is a new crystal
        if let Some(current) = current_crystal {
            let current_bond_count = current.bonds.as_ref().map_or(0, Vec::len);
            let new_bond_count = crystal.bonds.as_ref().map_or(0, Vec::len);
            if current.atoms.len() != crystal.atoms.len() || current_bond_count != new_bond_count {
                commands.insert_resource(crystal.clone());
                if new_bond_count > 0 {
                    println!(
                        "Crystal updated with {} atoms and {} file bonds",
                        crystal.atoms.len(),
                        new_bond_count
                    );
                } else {
                    println!("Crystal updated with {} atoms", crystal.atoms.len());
                }
            }
        } else {
            commands.insert_resource(crystal.clone());
            let new_bond_count = crystal.bonds.as_ref().map_or(0, Vec::len);
            if new_bond_count > 0 {
                println!(
                    "Crystal loaded with {} atoms and {} file bonds",
                    crystal.atoms.len(),
                    new_bond_count
                );
            } else {
                println!("Crystal loaded with {} atoms", crystal.atoms.len());
            }
        }
    }
}
