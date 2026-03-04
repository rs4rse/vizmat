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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Resource, Clone)]
pub struct BondCache {
    pub is_dirty: bool,
    pub bonds: Vec<Bond>,
    pub source: BondSourceMode,
    pub valid: bool,
    pub atom_count: usize,
    pub file_bond_count: usize,
    pub has_file_bonds: bool,
    pub infer_tolerance_scale: f32,
    pub settings_enabled: bool,
}

impl Default for BondCache {
    fn default() -> Self {
        Self {
            is_dirty: true,
            bonds: Vec::new(),
            source: BondSourceMode::Disabled,
            valid: false,
            atom_count: 0,
            file_bond_count: 0,
            has_file_bonds: false,
            infer_tolerance_scale: 0.0,
            settings_enabled: true,
        }
    }
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

pub fn mark_bond_cache_dirty(
    crystal: Option<Res<Crystal>>,
    bond_settings: Res<BondInferenceSettings>,
    mut bond_cache: ResMut<BondCache>,
) {
    let Some(crystal) = crystal else {
        return;
    };

    if crystal.is_changed() || bond_settings.is_changed() {
        bond_cache.is_dirty = true;
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

impl BondCache {
    fn can_reuse(&self, crystal: &Crystal, settings: &BondInferenceSettings) -> bool {
        if !self.valid || self.is_dirty {
            return false;
        }

        if self.settings_enabled != settings.enabled {
            return false;
        }

        if self.atom_count != crystal.atoms.len() {
            return false;
        }

        if !settings.enabled {
            return self.source == BondSourceMode::Disabled;
        }

        let has_file_bonds = crystal.bonds.as_ref().is_some_and(|b| !b.is_empty());
        if has_file_bonds != self.has_file_bonds {
            return false;
        }

        if has_file_bonds {
            return self.source == BondSourceMode::File
                && self.file_bond_count == crystal.bonds.as_ref().map_or(0, Vec::len);
        }

        self.source == BondSourceMode::Inferred
            && (self.infer_tolerance_scale - settings.tolerance_scale).abs() <= f32::EPSILON
    }

    pub fn resolve_bonds_cached<'a>(
        &'a mut self,
        crystal: &Crystal,
        settings: &BondInferenceSettings,
    ) -> (&'a [Bond], BondSourceMode) {
        if self.can_reuse(crystal, settings) {
            return (&self.bonds, self.source);
        }

        let (next_bonds, source) = resolve_bonds(crystal, settings);
        self.bonds = next_bonds;
        self.source = source;
        self.atom_count = crystal.atoms.len();
        self.file_bond_count = crystal.bonds.as_ref().map_or(0, Vec::len);
        self.has_file_bonds = crystal.has_explicit_bonds();
        self.infer_tolerance_scale = settings.tolerance_scale;
        self.settings_enabled = settings.enabled;
        self.is_dirty = false;
        self.valid = true;

        (&self.bonds, self.source)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::formats::parse_structure_by_extension;
    use crate::structure::{BondCache, BondInferenceSettings, BondSourceMode, Crystal};

    fn asset_file(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../vizmat-app/assets/structures")
            .join(path)
    }

    fn load_structure(path: &str) -> Crystal {
        let full_path = asset_file(path);
        let ext = full_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let contents =
            std::fs::read_to_string(&full_path).expect("expected structure asset to be readable");
        parse_structure_by_extension(ext, &contents).expect("expected structure parse success")
    }

    #[test]
    fn inferred_bonds_cached_6vxx() {
        let crystal = load_structure("proteins/6VXX.pdb");
        let settings = BondInferenceSettings::default();
        let mut cache = BondCache::default();

        let (first_bonds, first_source) = cache.resolve_bonds_cached(&crystal, &settings);
        assert_eq!(first_source, BondSourceMode::Inferred);
        let first_bonds = first_bonds.to_vec();

        let (second_bonds, second_source) = cache.resolve_bonds_cached(&crystal, &settings);
        assert_eq!(second_source, BondSourceMode::Inferred);
        assert_eq!(first_bonds, second_bonds);
    }
}
