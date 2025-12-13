#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::render::view::RenderLayers;

use crate::constants::{get_element_color, get_element_size};
use crate::structure::{AtomEntity, Crystal};

const LAYER_GIZMO: RenderLayers = RenderLayers::layer(1);
const LAYER_CANVAS: RenderLayers = RenderLayers::layer(0);

#[derive(Component)]
pub(crate) struct MainCamera;

// Component for UI text
#[derive(Component)]
pub(crate) struct FileUploadText;

// Component for load default button
#[derive(Component)]
pub(crate) struct LoadDefaultButton;

// System to set up file upload UI
pub(crate) fn setup_file_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Drag and drop an XYZ file here to visualize"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        FileUploadText,
    ));

    // Add button to load default structure
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(18.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    LoadDefaultButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Load Default Structure"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

// System to update file upload UI
pub(crate) fn update_file_ui(
    file_drag_drop: Res<crate::io::FileDragDrop>,
    mut text_query: Query<&mut Text, With<FileUploadText>>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        if let Some(path) = file_drag_drop.dragged_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Update the text content
                **text = format!("Loaded: {file_name}");
            }
        } else {
            **text = "Drag and drop an XYZ file here to visualize".to_string();
        }
    }
}

/// Button that resets the camera to its original position/orientation.
#[derive(Component)]
pub(crate) struct ResetCameraButton;

#[derive(Component)]
pub(crate) struct LightAttachmentButton {
    attached: bool,
}

/// Marks an entity that spawned the main camera.
#[derive(Resource)]
pub(crate) struct MainCameraEntity(pub Entity);

/// Marks the primary directional light used for shading.
#[derive(Resource)]
pub(crate) struct MainLightEntity(pub Entity);

/// Stores camera orbit information and the original configuration so it can be restored.
#[derive(Resource)]
pub(crate) struct CameraRig {
    target: Vec3,
    distance: f32,
    initial_target: Vec3,
    initial_translation: Vec3,
    initial_rotation: Quat,
    initial_scale: Vec3,
}

// System to clear existing atoms when new crystal is loaded
#[allow(dead_code)]
pub fn clear_old_atoms(mut commands: Commands, atom_query: Query<Entity, With<AtomEntity>>) {
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }
}

// System to handle button click to load default structure
#[allow(clippy::type_complexity)]
pub(crate) fn handle_load_default_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<LoadDefaultButton>),
    >,
    mut commands: Commands,
    crystal: Option<Res<Crystal>>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.35, 0.35, 0.35));
                // Load default water molecule
                if crystal.is_none() {
                    crate::io::load_default_crystal(commands.reborrow());
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.25, 0.25, 0.25));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.15, 0.15, 0.15));
            }
        }
    }
}

// System to respawn atoms when crystal changes
pub(crate) fn update_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crystal: Option<Res<Crystal>>,
    atom_query: Query<Entity, With<AtomEntity>>,
) {
    if let Some(crystal) = crystal {
        if crystal.is_changed() {
            // Clear existing atoms
            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn new atoms
            spawn_atoms(&mut commands, &mut meshes, &mut materials, &crystal);

            println!("Scene updated with new crystal structure");
        }
    }
}

// Helper function to spawn atoms
fn spawn_atoms(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    crystal: &Crystal,
) {
    // Create a sphere mesh for atoms
    let sphere_mesh = meshes.add(Sphere::new(1.0));

    // Create materials for different elements
    let mut element_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();

    // Spawn atoms as 3D spheres
    for atom in &crystal.atoms {
        // Get or create material for this element
        let material = element_materials
            .entry(atom.element.clone())
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: get_element_color(&atom.element),
                    metallic: 0.0,
                    ..default()
                })
            })
            .clone();

        // Spawn the atom as a sphere
        commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(atom.x, atom.y, atom.z)
                .with_scale(Vec3::splat(get_element_size(&atom.element))),
            AtomEntity,
        ));
    }
}

// System to set up the camera
pub fn setup_cameras(mut commands: Commands, windows: Query<&Window>) {
    let window = windows.single().unwrap();
    let viewport_size = UVec2::new(200, 200);
    let bottom_left_y = window.physical_height() - viewport_size.y - 10;
    let viewport_position = UVec2::new(10, bottom_left_y);

    let camera_transform = Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
    let initial_translation = camera_transform.translation;
    let initial_rotation = camera_transform.rotation;
    let initial_scale = camera_transform.scale;
    let initial_target = Vec3::ZERO;

    // Spawn cameras
    let camera_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                ..default()
            },
            IsDefaultUiCamera,
            Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            LAYER_CANVAS,
            MainCamera,
        ))
        .with_children(|parent| {
            // GIZMO CAMERA
            parent.spawn((
                Camera3d { ..default() },
                Camera {
                    order: 1,
                    viewport: Some(Viewport {
                        physical_position: viewport_position,
                        physical_size: viewport_size,
                        ..default()
                    }),
                    ..default()
                },
                Transform::default(),
                GlobalTransform::default(),
                LAYER_GIZMO,
            ));
        })
        .id();

    commands.insert_resource(MainCameraEntity(camera_entity));
    commands.insert_resource(CameraRig {
        target: initial_target,
        distance: initial_translation.distance(initial_target),
        initial_target,
        initial_translation,
        initial_rotation,
        initial_scale,
    });
}

pub(crate) fn setup_light(mut commands: Commands, camera: Res<MainCameraEntity>) {
    let light_entity = commands
        .spawn((
            DirectionalLight { ..default() },
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
            ChildOf(camera.0),
        ))
        .id();

    commands.insert_resource(MainLightEntity(light_entity));
}

// Setup minimal UI with toggle buttons
pub fn setup_buttons(mut commands: Commands) {
    // buttons at top-left
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    LightAttachmentButton { attached: false },
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("light not follow cam"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    ResetCameraButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Reset Camera"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub(crate) fn spawn_axis(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut axis = |color: Color,
                    (x, y, z): (f32, f32, f32),
                    (x_, y_, z_): (f32, f32, f32)|
     -> (
        (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform),
        RenderLayers,
    ) {
        let mesh = meshes.add(Mesh::from(Cuboid::new(x, y, z)));
        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        });
        (
            (
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(x_, y_, z_),
            ),
            LAYER_GIZMO, // visible only to axis camera
        )
    };

    let scale = 2.0;
    commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            LAYER_GIZMO,
        ))
        .with_children(|p| {
            p.spawn(axis(
                Srgba::RED.into(),
                (scale * 1., scale * 0.1, scale * 0.1),
                (scale * 1. / 2., 0., 0.),
            )); // +X
            p.spawn(axis(
                Srgba::GREEN.into(),
                (scale * 0.1, scale * 1., scale * 0.1),
                (0., scale * 1. / 2., 0.),
            )); // +Y
            p.spawn(axis(
                Srgba::BLUE.into(),
                (scale * 0.1, scale * 0.1, scale * 1.),
                (0., 0., scale * 1. / 2.),
            )); // +Z
        });
}

// System to refresh atoms when Crystal resource changes
pub fn refresh_atoms_system(
    mut commands: Commands,
    crystal: Option<Res<Crystal>>,
    atom_entities: Query<Entity, With<AtomEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Only run when Crystal resource changes
    if let Some(ref crystal) = crystal {
        if !crystal.is_changed() {
            return;
        }
    }

    // Despawn all existing atoms
    for entity in atom_entities.iter() {
        commands.entity(entity).despawn();
    }

    // Respawn with new positions
    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));
    let mut element_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();

    if let Some(crystal) = crystal {
        for atom in &crystal.atoms {
            // Get or create material for this element
            let material = element_materials
                .entry(atom.element.clone())
                .or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: get_element_color(&atom.element),
                        metallic: 0.0,
                        ..default()
                    })
                })
                .clone();

            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(material),
                Transform {
                    translation: Vec3::new(atom.x, atom.y, atom.z),
                    scale: Vec3::splat(get_element_size(&atom.element)),
                    ..default()
                },
                AtomEntity,
            ));
        }
    }
}

// Simple camera controls
#[allow(clippy::too_many_arguments)]
pub(crate) fn camera_controls(
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_rig: ResMut<CameraRig>,
) {
    if let Ok(mut transform) = camera_query.single_mut() {
        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        let mut zoom_change = 0.0;
        let mut pan_request = Vec2::ZERO;

        const MIN_DISTANCE: f32 = 0.2;
        const MAX_DISTANCE: f32 = 200.0;

        let mut mouse_delta = Vec2::ZERO;
        for motion in mouse_motion_events.read() {
            mouse_delta += motion.delta;
        }

        if mouse_buttons.pressed(MouseButton::Left) {
            let sensitivity = 0.005;
            yaw_delta -= mouse_delta.x * sensitivity;
            pitch_delta -= mouse_delta.y * sensitivity;
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            pan_request = mouse_delta;
        }

        for wheel in mouse_wheel_events.read() {
            zoom_change -= wheel.y * 0.2;
        }

        // Keep camera offset updated relative to target.
        let mut offset = transform.translation - camera_rig.target;
        if offset.length_squared() < f32::EPSILON {
            offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
        }

        if yaw_delta != 0.0 || pitch_delta != 0.0 {
            let rotation = Quat::from_euler(EulerRot::XYZ, pitch_delta, yaw_delta, 0.0);
            offset = rotation * offset;
        }

        if pan_request != Vec2::ZERO {
            let distance = offset.length().max(MIN_DISTANCE);
            let forward = (-offset).normalize_or_zero();
            let mut right = forward.cross(Vec3::Y).normalize_or_zero();
            if right.length_squared() < f32::EPSILON {
                right = Vec3::X;
            }
            let up = right.cross(forward).normalize_or_zero();
            let pan_speed = 0.002 * distance;
            let pan_offset = (-pan_request.x * right + pan_request.y * up) * pan_speed;
            camera_rig.target += pan_offset;
        }

        let mut distance = offset.length().max(MIN_DISTANCE);
        if zoom_change != 0.0 {
            let factor = (1.0 + zoom_change).clamp(0.2, 5.0);
            distance = (distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
        }

        let direction = offset.normalize_or_zero();
        offset = if direction.length_squared() > 0.0 {
            direction * distance
        } else {
            Vec3::new(0.0, 0.0, distance)
        };

        transform.translation = camera_rig.target + offset;
        transform.look_at(camera_rig.target, Vec3::Y);
        transform.scale = Vec3::ONE;
        camera_rig.distance = distance;
    }
}

#[allow(clippy::type_complexity)]
pub fn toggle_light_attachment(
    mut commands: Commands,
    light: Res<MainLightEntity>,
    camera: Res<MainCameraEntity>,
    mut interactions: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut LightAttachmentButton,
            &Children,
        ),
        (Changed<Interaction>, With<LightAttachmentButton>),
    >,
    q_trans: Query<&GlobalTransform>,
    mut texts: Query<&mut Text>,
) {
    for (interaction, mut background, mut button_state, children) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(Color::srgb(0.25, 0.25, 0.25));

                let old_state = button_state.attached;
                let new_state = !button_state.attached;

                button_state.attached = new_state;

                if old_state {
                    let light_trans = q_trans
                        .get(light.0)
                        .expect("light must have global transform when disattach");
                    let camera_trans = q_trans
                        .get(camera.0)
                        .expect("camera must have global transform");
                    let local = light_trans.reparented_to(camera_trans);
                    commands
                        .entity(light.0)
                        .insert(local)
                        .insert(ChildOf(camera.0));
                    info!("Light attached to camera");
                } else {
                    let glb_trans = q_trans
                        .get(light.0)
                        .expect("light must have global transform when disattach");
                    commands
                        .entity(light.0)
                        // preserve the world transform when detach
                        .insert(Transform::from(*glb_trans))
                        .remove::<ChildOf>();
                    info!("Light detached from camera");
                }

                // Update the text inside the button
                for child in children.iter() {
                    if let Ok(mut text) = texts.get_mut(child) {
                        text.0 = if new_state {
                            "light follow cam".into()
                        } else {
                            "light not follow cam".into()
                        };
                    }
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
            Interaction::None => {
                *background = BackgroundColor(Color::srgb(0.15, 0.15, 0.15));
            }
        }
    }
}

// Handle reset button interaction.
#[allow(clippy::type_complexity)]
pub fn reset_camera_button_interaction(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ResetCameraButton>),
    >,
    camera_entity: Option<Res<MainCameraEntity>>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    mut camera_rig: Option<ResMut<CameraRig>>,
) {
    for (interaction, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(Color::srgb(0.25, 0.25, 0.25));

                if let (Some(camera_entity), Some(rig)) =
                    (camera_entity.as_deref(), camera_rig.as_deref_mut())
                {
                    if let Ok(mut transform) = camera_query.get_mut(camera_entity.0) {
                        transform.translation = rig.initial_translation;
                        transform.rotation = rig.initial_rotation;
                        transform.scale = rig.initial_scale;
                        rig.target = rig.initial_target;
                        rig.distance = (rig.initial_translation - rig.initial_target)
                            .length()
                            .max(0.5);
                    }
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
            Interaction::None => {
                *background = BackgroundColor(Color::srgb(0.15, 0.15, 0.15));
            }
        }
    }
}
