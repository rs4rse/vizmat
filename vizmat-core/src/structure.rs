use bevy::prelude::*;
use ccmat_core::{atomic_number_from_symbol, math::Vector3, CrystalBuilder, FracCoord};

// Structure to represent an atom from XYZ file
// `#` is a macro. no inheritance. close to python decorator. injecting on top of something.
// traits are like interfaces.
#[derive(Debug, Clone)]
pub struct Site {
    pub element: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone)]
pub(crate) struct Lattice {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

impl Lattice {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Lattice { a, b, c }
    }

    pub fn a(&self) -> Vec3 {
        self.a
    }

    pub fn b(&self) -> Vec3 {
        self.b
    }

    pub fn c(&self) -> Vec3 {
        self.c
    }
}

// Structure to hold our crystal data
#[derive(Resource, Clone)]
pub(crate) struct Crystal {
    pub(crate) inner: ccmat_core::Crystal,
}

impl Crystal {
    // I can cast because IMO, the accuracy in vitualizer rendering is not essential.
    // XXX: but such re-allocation is not ideal.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn lattice(&self) -> Lattice {
        let latt = self.inner.lattice();
        let [ax, ay, az] = latt.a().map(|i| f64::from(i) as f32);
        let [bx, by, bz] = latt.b().map(|i| f64::from(i) as f32);
        let [cx, cy, cz] = latt.c().map(|i| f64::from(i) as f32);
        Lattice::new(
            Vec3::new(ax, ay, az),
            Vec3::new(bx, by, bz),
            Vec3::new(cx, cy, cz),
        )
    }

    pub(crate) fn positions(&self) -> Vec<[f32; 3]> {
        self.inner
            .cartesian_positions()
            .iter()
            .map(|p| p.map(|s| f64::from(s) as f32))
            .collect()
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
            })
            .collect()
    }

    pub(crate) fn set_sites(&mut self, sites: &[Site]) {
        let lattice = self.inner.lattice();
        let sites = sites
            .iter()
            .map(|s| {
                ccmat_core::Site::new(
                    Vector3([
                        FracCoord::from(f64::from(s.x)),
                        FracCoord::from(f64::from(s.y)),
                        FracCoord::from(f64::from(s.z)),
                    ]),
                    atomic_number_from_symbol(&s.element).expect("not a valid symbol"),
                )
            })
            .collect::<Vec<_>>();
        self.inner = CrystalBuilder::new()
            .with_lattice(&lattice)
            .with_sites(sites)
            .build_uncheck();
    }
}

// XXX: entity is the id point to the thing consist of components

// Component to mark atom entities
#[derive(Component)]
pub struct AtomEntity;

// Event to update the structure with new atom positions
#[derive(Event, Clone)]
pub struct UpdateStructure {
    pub sites: Vec<Site>,
}

// System to handle incoming structure updates
pub fn update_crystal_system(
    crystal: Option<ResMut<Crystal>>,
    mut events: EventReader<UpdateStructure>,
) {
    if let Some(mut crystal) = crystal {
        for event in events.read() {
            crystal.set_sites(&event.sites);
        }
    }
}
