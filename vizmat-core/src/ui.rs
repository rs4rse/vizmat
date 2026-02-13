#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
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

#[derive(Component)]
pub(crate) struct MoleculeRoot;

#[derive(Component)]
pub(crate) struct GizmoAxisRoot;

#[derive(Component)]
pub(crate) struct GizmoCamera;

// Component for UI text
#[derive(Component)]
pub(crate) struct FileUploadText;

// Component for load default button
#[derive(Component)]
pub(crate) struct LoadDefaultButton;

#[derive(Component)]
pub(crate) struct ThemeToggleButton;

#[derive(Component)]
pub(crate) struct ThemeToggleIcon;

#[derive(Component)]
pub(crate) struct HudTopBar;

#[derive(Component)]
pub(crate) struct HudBottomBar;

#[derive(Component)]
pub(crate) struct HudButton;

#[derive(Component)]
pub(crate) struct HudButtonLabel;

#[derive(Component)]
pub(crate) struct HudHelpText;

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    Dark,
    Light,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct UiTheme {
    pub(crate) mode: ThemeMode,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
        }
    }
}

#[derive(Clone, Copy)]
struct ThemePalette {
    scene_bg: Color,
    bar_bg: Color,
    bar_bg_alt: Color,
    button_bg: Color,
    button_hover: Color,
    button_pressed: Color,
    border: Color,
    text: Color,
    text_muted: Color,
}

fn theme_palette(mode: ThemeMode) -> ThemePalette {
    match mode {
        ThemeMode::Dark => ThemePalette {
            scene_bg: Color::srgb(0.02, 0.03, 0.05),
            bar_bg: Color::srgba(0.07, 0.09, 0.12, 0.95),
            bar_bg_alt: Color::srgba(0.07, 0.09, 0.12, 0.90),
            button_bg: Color::srgb(0.15, 0.15, 0.15),
            button_hover: Color::srgb(0.20, 0.20, 0.20),
            button_pressed: Color::srgb(0.25, 0.25, 0.25),
            border: Color::srgb(0.30, 0.30, 0.30),
            text: Color::WHITE,
            text_muted: Color::srgb(0.86, 0.90, 0.95),
        },
        ThemeMode::Light => ThemePalette {
            scene_bg: Color::srgb(0.96, 0.97, 0.99),
            bar_bg: Color::srgba(0.94, 0.95, 0.97, 0.98),
            bar_bg_alt: Color::srgba(0.92, 0.93, 0.96, 0.96),
            button_bg: Color::srgb(0.95, 0.95, 0.97),
            button_hover: Color::srgb(0.90, 0.91, 0.94),
            button_pressed: Color::srgb(0.84, 0.86, 0.90),
            border: Color::srgb(0.74, 0.76, 0.81),
            text: Color::srgb(0.12, 0.14, 0.18),
            text_muted: Color::srgb(0.18, 0.22, 0.30),
        },
    }
}

fn themed_button_bg(mode: ThemeMode, interaction: Interaction) -> Color {
    let p = theme_palette(mode);
    match interaction {
        Interaction::Pressed => p.button_pressed,
        Interaction::Hovered => p.button_hover,
        Interaction::None => p.button_bg,
    }
}

#[derive(SystemParam)]
pub(crate) struct HudThemeParams<'w, 's> {
    bg: ParamSet<'w, 's, HudBgQueries<'w, 's>>,
    text: ParamSet<'w, 's, HudTextQueries<'w, 's>>,
}

type HudBgQueries<'w, 's> = (
    Query<'w, 's, &'static mut BackgroundColor, With<HudTopBar>>,
    Query<'w, 's, &'static mut BackgroundColor, With<HudBottomBar>>,
    Query<
        'w,
        's,
        (
            &'static Interaction,
            &'static mut BackgroundColor,
            &'static mut BorderColor,
        ),
        With<HudButton>,
    >,
);

type HudTextQueries<'w, 's> = (
    Query<'w, 's, &'static mut TextColor, With<HudButtonLabel>>,
    Query<'w, 's, &'static mut TextColor, (With<FileUploadText>, Without<HudButtonLabel>)>,
    Query<'w, 's, &'static mut TextColor, With<HudHelpText>>,
);

type ThemeToggleInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static Children,
    ),
    (Changed<Interaction>, With<ThemeToggleButton>),
>;

type MainCameraTransformProjectionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static Projection),
    (With<Camera3d>, Without<MoleculeRoot>),
>;

// System to set up file upload UI
pub(crate) fn setup_file_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(UiTheme::default());
    let p = theme_palette(ThemeMode::Dark);
    let icon_font: Handle<Font> = asset_server.load("fonts/fa-solid-900.ttf");
    commands.insert_resource(ClearColor(p.scene_bg));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(50.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(p.bar_bg),
            HudTopBar,
        ))
        .with_children(|top| {
            top.spawn((
                Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|row| {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    LoadDefaultButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Load Default"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text),
                        HudButtonLabel,
                    ));
                });

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    ResetCameraButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Reset View"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text),
                        HudButtonLabel,
                    ));
                });

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    LightAttachmentButton { attached: false },
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Light: Static"),
                        TextFont {
                            font: default(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text),
                        HudButtonLabel,
                    ));
                });
            });

            top.spawn((
                Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|right| {
                right.spawn((
                    Text::new("Drop XYZ file"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(p.text),
                    FileUploadText,
                ));

                right
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderRadius::MAX,
                        BorderColor(p.border),
                        BackgroundColor(p.button_bg),
                        ThemeToggleButton,
                        HudButton,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("\u{f186}"),
                            TextFont {
                                font: icon_font.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(p.text),
                            HudButtonLabel,
                            ThemeToggleIcon,
                        ));
                    });
            });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(p.bar_bg_alt),
            HudBottomBar,
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new(
                    "Doom-like: W/A/S/D move  Shift sprint  Q/E rotate  LMB rotate  RMB pan  Wheel zoom",
                ),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(p.text_muted),
                HudHelpText,
            ));
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
            **text = "Drop XYZ file".to_string();
        }
    }
}

pub(crate) fn toggle_theme_button(
    mut theme: ResMut<UiTheme>,
    mut interaction_query: ThemeToggleInteractionQuery<'_, '_>,
    mut texts: Query<&mut Text, With<ThemeToggleIcon>>,
) {
    for (interaction, mut color, children) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                theme.mode = match theme.mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                for child in children.iter() {
                    if let Ok(mut text) = texts.get_mut(child) {
                        text.0 = match theme.mode {
                            ThemeMode::Dark => "\u{f186}".into(),
                            ThemeMode::Light => "\u{f185}".into(),
                        };
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));
            }
            Interaction::None => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));
            }
        }
    }
}

pub(crate) fn apply_theme_to_hud(
    theme: Res<UiTheme>,
    mut clear_color: ResMut<ClearColor>,
    mut themed: HudThemeParams<'_, '_>,
) {
    if !theme.is_changed() {
        return;
    }
    let p = theme_palette(theme.mode);
    clear_color.0 = p.scene_bg;
    for mut bg in &mut themed.bg.p0() {
        *bg = BackgroundColor(p.bar_bg);
    }
    for mut bg in &mut themed.bg.p1() {
        *bg = BackgroundColor(p.bar_bg_alt);
    }
    for (interaction, mut bg, mut border) in &mut themed.bg.p2() {
        *bg = BackgroundColor(themed_button_bg(theme.mode, *interaction));
        *border = BorderColor(p.border);
    }
    for mut color in &mut themed.text.p0() {
        *color = TextColor(p.text);
    }
    if let Ok(mut color) = themed.text.p1().single_mut() {
        *color = TextColor(p.text);
    }
    for mut color in &mut themed.text.p2() {
        *color = TextColor(p.text_muted);
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

fn crystal_center_and_extents(crystal: &Crystal) -> Option<(Vec3, Vec3)> {
    let first = crystal.atoms.first()?;
    let mut min = Vec3::new(first.x, first.y, first.z);
    let mut max = min;

    for atom in &crystal.atoms {
        let p = Vec3::new(atom.x, atom.y, atom.z);
        min = min.min(p);
        max = max.max(p);
    }

    Some(((min + max) * 0.5, max - min))
}

fn fit_distance_for_extents(extents: Vec3, projection: &Projection) -> f32 {
    let radius = (extents.length() * 0.5).max(0.5);
    match projection {
        Projection::Perspective(perspective) => {
            let fov_y = perspective.fov.max(0.1);
            let aspect = perspective.aspect_ratio.max(0.1);
            let fov_x = 2.0 * ((fov_y * 0.5).tan() * aspect).atan();
            let limiting_fov = fov_x.min(fov_y);
            (radius / (limiting_fov * 0.5).tan()) * 1.2
        }
        Projection::Orthographic(_) => radius * 2.5,
        _ => radius * 2.5,
    }
}

fn apply_initial_camera_reset(transform: &mut Transform, rig: &mut CameraRig) {
    transform.translation = rig.initial_translation;
    transform.rotation = rig.initial_rotation;
    transform.scale = rig.initial_scale;
    rig.target = rig.initial_target;
    rig.distance = (rig.initial_translation - rig.initial_target)
        .length()
        .max(0.5);
}

fn apply_framed_camera_reset(
    transform: &mut Transform,
    projection: &Projection,
    rig: &mut CameraRig,
    crystal: Option<&Crystal>,
) {
    let Some(crystal) = crystal else {
        apply_initial_camera_reset(transform, rig);
        return;
    };
    let Some((center, extents)) = crystal_center_and_extents(crystal) else {
        apply_initial_camera_reset(transform, rig);
        return;
    };

    // Keep the default viewing direction, but place the camera far enough to frame the model.
    let mut view_dir = (rig.initial_translation - rig.initial_target).normalize_or_zero();
    if view_dir.length_squared() < f32::EPSILON {
        view_dir = Vec3::new(1.0, 1.0, 1.0).normalize();
    }
    let distance = fit_distance_for_extents(extents, projection).max(0.5);

    rig.target = center;
    rig.distance = distance;
    transform.translation = center + view_dir * distance;
    transform.look_at(center, Vec3::Y);
    transform.scale = Vec3::ONE;
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
    theme: Res<UiTheme>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                // Load default water molecule
                if crystal.is_none() {
                    crate::io::load_default_crystal(commands.reborrow());
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));
            }
            Interaction::None => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));
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
    molecule_root: Query<Entity, With<MoleculeRoot>>,
) {
    if let Some(crystal) = crystal {
        if crystal.is_changed() {
            // Clear existing atoms
            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn new atoms
            if let Ok(root_entity) = molecule_root.single() {
                spawn_atoms(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &crystal,
                    root_entity,
                );
            }

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
    root_entity: Entity,
) {
    // Create a sphere mesh for atoms
    let sphere_mesh = meshes.add(Sphere::new(1.0));

    // Create materials for different elements
    let mut element_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();

    commands.entity(root_entity).with_children(|parent| {
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
            parent.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(atom.x, atom.y, atom.z)
                    .with_scale(Vec3::splat(get_element_size(&atom.element))),
                AtomEntity,
            ));
        }
    });
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

    commands.spawn((
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        MoleculeRoot,
    ));

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
                GizmoCamera,
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
    // HUD buttons are created in `setup_file_ui`.
    let _ = &mut commands;
}

pub(crate) fn spawn_axis(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gizmo_camera: Query<Entity, With<GizmoCamera>>,
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
    let Ok(gizmo_camera_entity) = gizmo_camera.single() else {
        return;
    };

    commands
        .entity(gizmo_camera_entity)
        .with_children(|parent| {
            parent
                .spawn((
                    // Keep gizmo at a fixed distance in front of the gizmo camera,
                    // so it stays the same apparent size and position on screen.
                    Transform::from_xyz(0.0, 0.0, -6.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    LAYER_GIZMO,
                    GizmoAxisRoot,
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
        });
}

pub(crate) fn sync_gizmo_axis_rotation(
    molecule_query: Query<&Transform, (With<MoleculeRoot>, Changed<Transform>)>,
    mut gizmo_query: Query<&mut Transform, (With<GizmoAxisRoot>, Without<MoleculeRoot>)>,
) {
    if let (Ok(molecule), Ok(mut gizmo)) = (molecule_query.single(), gizmo_query.single_mut()) {
        gizmo.rotation = molecule.rotation;
    }
}

// System to refresh atoms when Crystal resource changes
pub fn refresh_atoms_system(
    mut commands: Commands,
    crystal: Option<Res<Crystal>>,
    atom_entities: Query<Entity, With<AtomEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    molecule_root: Query<Entity, With<MoleculeRoot>>,
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

    if let Some(crystal) = crystal {
        if let Ok(root_entity) = molecule_root.single() {
            spawn_atoms(
                &mut commands,
                &mut meshes,
                &mut materials,
                &crystal,
                root_entity,
            );
        }
    }
}

// Simple camera controls
#[allow(clippy::too_many_arguments)]
pub(crate) fn camera_controls(
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<MoleculeRoot>)>,
    mut molecule_query: Query<&mut Transform, (With<MoleculeRoot>, Without<MainCamera>)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_rig: ResMut<CameraRig>,
) {
    if let (Ok(mut transform), Ok(mut molecule_transform)) =
        (camera_query.single_mut(), molecule_query.single_mut())
    {
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
            let yaw_delta = -mouse_delta.x * sensitivity;
            let pitch_delta = -mouse_delta.y * sensitivity;
            let yaw_rotation = Quat::from_rotation_y(yaw_delta);
            let right_axis = transform.right().normalize_or_zero();
            let pitch_rotation = if right_axis.length_squared() > 0.0 {
                Quat::from_axis_angle(right_axis, pitch_delta)
            } else {
                Quat::IDENTITY
            };
            molecule_transform.rotation =
                yaw_rotation * pitch_rotation * molecule_transform.rotation;
        }
        if keyboard.pressed(KeyCode::KeyQ) || keyboard.pressed(KeyCode::KeyE) {
            let mut yaw = 0.0;
            if keyboard.pressed(KeyCode::KeyQ) {
                yaw += 1.0;
            }
            if keyboard.pressed(KeyCode::KeyE) {
                yaw -= 1.0;
            }
            let rotate_speed = 1.8;
            molecule_transform.rotation =
                Quat::from_rotation_y(yaw * rotate_speed * time.delta_secs())
                    * molecule_transform.rotation;
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            pan_request = mouse_delta;
        }

        for wheel in mouse_wheel_events.read() {
            zoom_change -= wheel.y * 0.002;
        }

        // Keep camera offset updated relative to target.
        let mut offset = transform.translation - camera_rig.target;
        if offset.length_squared() < f32::EPSILON {
            offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
        }

        // FPS-style keyboard movement of the camera perspective.
        let distance = offset.length().max(MIN_DISTANCE);
        let forward = (-offset).normalize_or_zero();
        let mut right = forward.cross(Vec3::Y).normalize_or_zero();
        if right.length_squared() < f32::EPSILON {
            right = Vec3::X;
        }
        let mut move_dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) {
            move_dir += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            move_dir -= forward;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            move_dir += right;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            move_dir -= right;
        }
        if move_dir.length_squared() > 0.0 {
            let sprint =
                keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
            let move_speed = if sprint { 10.0 } else { 5.0 };
            let step = move_dir.normalize() * move_speed * time.delta_secs();
            camera_rig.target += step;
        }

        if pan_request != Vec2::ZERO {
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
    theme: Res<UiTheme>,
) {
    for (interaction, mut background, mut button_state, children) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));

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
                            "Light: Follow".into()
                        } else {
                            "Light: Static".into()
                        };
                    }
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));
            }
            Interaction::None => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));
            }
        }
    }
}

pub(crate) fn auto_reset_view_on_crystal_change(
    crystal: Option<Res<Crystal>>,
    camera_entity: Option<Res<MainCameraEntity>>,
    mut camera_query: MainCameraTransformProjectionQuery<'_, '_>,
    mut camera_rig: Option<ResMut<CameraRig>>,
    mut molecule_query: Query<&mut Transform, (With<MoleculeRoot>, Without<Camera3d>)>,
) {
    let Some(crystal) = crystal else {
        return;
    };
    if !crystal.is_changed() {
        return;
    }
    let (Some(camera_entity), Some(rig)) = (camera_entity.as_deref(), camera_rig.as_deref_mut())
    else {
        return;
    };

    if let Ok((mut transform, projection)) = camera_query.get_mut(camera_entity.0) {
        apply_framed_camera_reset(&mut transform, projection, rig, Some(&crystal));
    }

    if let Ok(mut molecule_transform) = molecule_query.single_mut() {
        molecule_transform.rotation = Quat::IDENTITY;
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
    mut camera_query: MainCameraTransformProjectionQuery<'_, '_>,
    mut camera_rig: Option<ResMut<CameraRig>>,
    mut molecule_query: Query<&mut Transform, (With<MoleculeRoot>, Without<Camera3d>)>,
    crystal: Option<Res<Crystal>>,
    theme: Res<UiTheme>,
) {
    for (interaction, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));

                if let (Some(camera_entity), Some(rig)) =
                    (camera_entity.as_deref(), camera_rig.as_deref_mut())
                {
                    if let Ok((mut transform, projection)) = camera_query.get_mut(camera_entity.0) {
                        apply_framed_camera_reset(
                            &mut transform,
                            projection,
                            rig,
                            crystal.as_deref(),
                        );
                    }
                }
                if let Ok(mut molecule_transform) = molecule_query.single_mut() {
                    molecule_transform.rotation = Quat::IDENTITY;
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));
            }
            Interaction::None => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));
            }
        }
    }
}
