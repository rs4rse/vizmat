use bevy::prelude::*;
use std::collections::HashMap;

use crate::constants::get_covalent_radius;

// Structure to represent an atom from XYZ file
// `#` is a macro. no inheritance. close to python decorator. injecting on top of something.
// traits are like interfaces.
#[derive(Debug, Clone)]
pub struct Site {
    pub element: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub chain_id: Option<String>,
    pub res_name: Option<String>,
}

/// Structure view, with extra bonds information to be plot in canvas.
#[derive(Resource, Clone)]
pub(crate) struct StructureView {
    pub(crate) inner: ccmat_core::Structure,
    pub bonds: Option<Vec<Bond>>,
    // TODO: cache sites because sites() call do too much allocation.
}

impl StructureView {
    // raw positions of view in visualizer is always cartesian.
    pub(crate) fn positions(&self) -> Vec<[f32; 3]> {
        match &self.inner {
            ccmat_core::Structure::Crystal(inner) => inner
                .positions()
                .iter()
                .map(|p| p.map(|s| f64::from(s) as f32))
                .collect(),
            ccmat_core::Structure::Molecule(inner) => inner
                .positions()
                .iter()
                .map(|p| p.map(|s| f64::from(s) as f32))
                .collect(),
        }
    }

    pub(crate) fn elements(&self) -> Vec<String> {
        self.inner.species().iter().map(|p| p.symbol()).collect()
    }

    pub(crate) fn sites(&self) -> Vec<Site> {
        self.positions()
            .iter()
            .zip(self.elements())
            .map(|(p, e)| Site {
                element: e,
                x: p[0],
                y: p[1],
                z: p[2],
                chain_id: None,
                res_name: None,
            })
            .collect()
    }

    pub(crate) fn nsites(&self) -> usize {
        self.positions().len()
    }

    // pub(crate) fn set_bonds(&mut self, bonds: &[Bond]) {
    //     self.bonds = Some(bonds.to_vec());
    // }

    // pub(crate) fn set_sites(&mut self, sites: &[Site]) {
    //     // XXX: override the inner with reallocation, performance not good
    //     let sites = sites
    //         .iter()
    //         .map(|s| {
    //             // TODO: I should not rely directly on ccmat_core API call.
    //             ccmat_core::SiteCartesian::new(
    //                 Vector3([
    //                     Angstrom::from(f64::from(s.x)),
    //                     Angstrom::from(f64::from(s.y)),
    //                     Angstrom::from(f64::from(s.z)),
    //                 ]),
    //                 atomic_number_from_symbol(&s.element).expect("not a valid symbol"),
    //             )
    //         })
    //         .collect::<Vec<_>>();
    //     self.inner = MoleculeBuilder::new().with_sites(sites).build_uncheck();
    // }

    pub fn has_explicit_bonds(&self) -> bool {
        self.bonds.as_ref().is_some_and(|b| !b.is_empty())
    }
}

// // Structure to hold our crystal data
// #[derive(Resource, Clone)]
// pub struct Crystal {
//     pub atoms: Vec<Site>,
//     pub bonds: Option<Vec<Bond>>,
// }

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
    pub inner: ccmat_core::Structure,
}

// System to handle incoming structure updates
pub fn update_structure_system(
    sv: Option<ResMut<StructureView>>,
    mut events: EventReader<UpdateStructure>,
) {
    if let Some(mut sv) = sv {
        for event in events.read() {
            sv.inner.clone_from(&event.inner);
        }
    }
}

#[inline]
fn bond_cutoff(a: &Site, b: &Site, tolerance_scale: f32) -> f32 {
    ((get_covalent_radius(&a.element) + get_covalent_radius(&b.element)) * tolerance_scale)
        .clamp(0.4, 2.4)
}

pub fn infer_bonds_grid(sv: &StructureView, tolerance_scale: f32) -> Vec<Bond> {
    let sites = &sv.sites();
    if sites.len() < 2 {
        return Vec::new();
    }

    let mut max_radius = 0.0_f32;
    for site in sites {
        max_radius = max_radius.max(get_covalent_radius(&site.element));
    }
    let cell_size = (max_radius * 2.0 * tolerance_scale).clamp(1.2, 3.0);

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    let mut bonds = Vec::with_capacity(sites.len().saturating_mul(2));

    for (i, site) in sites.iter().enumerate() {
        let cell = (
            (site.x / cell_size).floor() as i32,
            (site.y / cell_size).floor() as i32,
            (site.z / cell_size).floor() as i32,
        );

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                    if let Some(candidates) = grid.get(&neighbor_cell) {
                        for &j in candidates {
                            let other = &sites[j];
                            let cutoff = bond_cutoff(site, other, tolerance_scale);
                            let ddx = site.x - other.x;
                            let ddy = site.y - other.y;
                            let ddz = site.z - other.z;
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

pub fn resolve_bonds(
    sv: &StructureView,
    settings: &BondInferenceSettings,
) -> (Vec<Bond>, BondSourceMode) {
    if !settings.enabled {
        return (Vec::new(), BondSourceMode::Disabled);
    }
    if let Some(file_bonds) = sv.bonds.as_ref().filter(|b| !b.is_empty()) {
        return (file_bonds.clone(), BondSourceMode::File);
    }
    (
        infer_bonds_grid(sv, settings.tolerance_scale),
        BondSourceMode::Inferred,
    )
}
