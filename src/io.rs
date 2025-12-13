// io.rs
use bevy::prelude::*;
use std::path::PathBuf;

use crate::parse::parse_xyz_content;
use crate::structure::{Atom, Crystal};

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
            },
            Atom {
                element: "H".to_string(),
                x: 0.757,
                y: 0.587,
                z: 0.0,
            },
            Atom {
                element: "H".to_string(),
                x: -0.757,
                y: 0.587,
                z: 0.0,
            },
        ],
    };

    commands.insert_resource(crystal);
}

// Resource to handle file drag and drop
#[derive(Resource, Default)]
pub(crate) struct FileDragDrop {
    dragged_file: Option<PathBuf>,
    loaded_crystal: Option<Crystal>,
}

impl FileDragDrop {
    pub(crate) fn dragged_file(&self) -> Option<&PathBuf> {
        self.dragged_file.as_ref()
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
                    if extension == "xyz" {
                        file_drag_drop.dragged_file = Some(path_buf.clone());
                    } else {
                        println!("Unsupported file type. Please drop an XYZ file.");
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

// System to load crystal from dropped file
pub(crate) fn load_dropped_file(
    mut file_drag_drop: ResMut<FileDragDrop>,
    mut crystal_loaded: Local<bool>,
) {
    if let Some(ref path) = file_drag_drop.dragged_file {
        if !*crystal_loaded {
            match std::fs::read_to_string(path) {
                Ok(contents) => match parse_xyz_content(&contents) {
                    Ok(crystal) => {
                        println!("Successfully loaded crystal from: {:?}", path);
                        file_drag_drop.loaded_crystal = Some(crystal);
                        *crystal_loaded = true;
                    }
                    Err(e) => {
                        eprintln!("Failed to parse XYZ file: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);
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
            if current.atoms.len() != crystal.atoms.len() {
                commands.insert_resource(crystal.clone());
                println!("Crystal updated with {} atoms", crystal.atoms.len());
            }
        } else {
            commands.insert_resource(crystal.clone());
            println!("Crystal loaded with {} atoms", crystal.atoms.len());
        }
    }
}
