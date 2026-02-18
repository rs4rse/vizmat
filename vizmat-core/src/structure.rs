use bevy::prelude::*;
use std::collections::HashMap;

use crate::constants::get_covalent_radius;

// Structure to represent an atom from XYZ file
// `#` is a macro. no inheritance. close to python decorator. injecting on top of something.
// traits are like interfaces.
#[derive(Debug, Clone)]
pub struct Atom {
    pub element: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub chain_id: Option<String>,
    pub res_name: Option<String>,
}

// Structure to hold our crystal data
#[derive(Resource, Clone)]
pub struct Crystal {
    pub atoms: Vec<Atom>,
    pub bonds: Option<Vec<Bond>>,
}

#[derive(Resource, Clone, Copy)]
pub struct BondInferenceSettings {
    pub enabled: bool,
    // Applied value used by bond generation.
    pub tolerance_scale: f32,
    // Live slider value shown in the UI.
    pub ui_tolerance_scale: f32,
    pub last_ui_change_secs: f64,
}

impl Default for BondInferenceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tolerance_scale: 1.15,
            ui_tolerance_scale: 1.15,
            last_ui_change_secs: 0.0,
        }
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum AtomColorMode {
    #[default]
    Element,
    Chain,
    Residue,
    Ring,
    BondEnv,
    Functional,
}

// XXX: entity is the id point to the thing consist of components

// Component to mark atom entities
#[derive(Component)]
pub struct AtomEntity;

#[derive(Component, Debug, Clone, Copy)]
pub struct AtomIndex(pub usize);

// Component to mark bond entities.
#[derive(Component)]
pub struct BondEntity;

#[derive(Component, Debug, Clone, Copy)]
pub struct BondOrder(pub u8);

#[derive(Debug, Clone, Copy)]
pub struct Bond {
    pub a: usize,
    pub b: usize,
    pub order: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondSourceMode {
    Disabled,
    File,
    Inferred,
}

// Event to update the structure with new atom positions
#[derive(Event, Clone)]
pub struct UpdateStructure {
    pub atoms: Vec<Atom>,
}

// System to handle incoming structure updates
pub fn update_crystal_system(
    crystal: Option<ResMut<Crystal>>,
    mut events: EventReader<UpdateStructure>,
) {
    if let Some(mut crystal) = crystal {
        for event in events.read() {
            crystal.atoms.clone_from(&event.atoms);
        }
    }
}

#[inline]
fn bond_cutoff(a: &Atom, b: &Atom, tolerance_scale: f32) -> f32 {
    ((get_covalent_radius(&a.element) + get_covalent_radius(&b.element)) * tolerance_scale)
        .clamp(0.4, 2.4)
}

pub fn infer_bonds_grid(crystal: &Crystal, tolerance_scale: f32) -> Vec<Bond> {
    let atoms = &crystal.atoms;
    if atoms.len() < 2 {
        return Vec::new();
    }

    let mut max_radius = 0.0_f32;
    for atom in atoms {
        max_radius = max_radius.max(get_covalent_radius(&atom.element));
    }
    let cell_size = (max_radius * 2.0 * tolerance_scale).clamp(1.2, 3.0);

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    let mut bonds = Vec::with_capacity(atoms.len().saturating_mul(2));

    for (i, atom) in atoms.iter().enumerate() {
        let cell = (
            (atom.x / cell_size).floor() as i32,
            (atom.y / cell_size).floor() as i32,
            (atom.z / cell_size).floor() as i32,
        );

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                    if let Some(candidates) = grid.get(&neighbor_cell) {
                        for &j in candidates {
                            let other = &atoms[j];
                            let cutoff = bond_cutoff(atom, other, tolerance_scale);
                            let ddx = atom.x - other.x;
                            let ddy = atom.y - other.y;
                            let ddz = atom.z - other.z;
                            let dist_sq = ddx * ddx + ddy * ddy + ddz * ddz;
                            if dist_sq <= cutoff * cutoff {
                                bonds.push(Bond {
                                    a: j,
                                    b: i,
                                    order: 1,
                                });
                            }
                        }
                    }
                }
            }
        }

        grid.entry(cell).or_default().push(i);
    }

    bonds
}

impl Crystal {
    pub fn has_explicit_bonds(&self) -> bool {
        self.bonds.as_ref().is_some_and(|b| !b.is_empty())
    }
}

pub fn resolve_bonds(
    crystal: &Crystal,
    settings: &BondInferenceSettings,
) -> (Vec<Bond>, BondSourceMode) {
    if !settings.enabled {
        return (Vec::new(), BondSourceMode::Disabled);
    }
    if let Some(file_bonds) = crystal.bonds.as_ref().filter(|b| !b.is_empty()) {
        return (file_bonds.clone(), BondSourceMode::File);
    }
    (
        infer_bonds_grid(crystal, settings.tolerance_scale),
        BondSourceMode::Inferred,
    )
}
