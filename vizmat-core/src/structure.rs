use bevy::prelude::*;

// Structure to represent an atom from XYZ file
// `#` is a macro. no inheritance. close to python decorator. injecting on top of something.
// traits are like interfaces.
#[derive(Debug, Clone)]
pub struct Atom {
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
pub struct Crystal {
    pub lattice: Lattice,
    pub atoms: Vec<Atom>,
}

// XXX: entity is the id point to the thing consist of components

// Component to mark atom entities
#[derive(Component)]
pub struct AtomEntity;

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
