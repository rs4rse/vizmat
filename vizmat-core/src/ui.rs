#![allow(clippy::needless_pass_by_value)]

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::render::view::RenderLayers;
use bevy::ui::RelativeCursorPosition;

use crate::constants::{get_element_color, get_element_size, get_residue_class_color};
use crate::formats::SUPPORTED_EXTENSIONS_HELP;
#[cfg(not(target_arch = "wasm32"))]
use crate::formats::{parse_structure_by_extension, SUPPORTED_EXTENSIONS};
use crate::io::FileStatusKind;
use crate::structure::{
    resolve_bonds, AtomColorMode, AtomEntity, AtomIndex, BondEntity, BondInferenceSettings,
    BondOrder, Crystal,
};

const LAYER_GIZMO: RenderLayers = RenderLayers::layer(1);
const LAYER_CANVAS: RenderLayers = RenderLayers::layer(0);
const GIZMO_VIEWPORT_SIZE_PX: u32 = 200;
const GIZMO_VIEWPORT_MARGIN_PX: u32 = 10;

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
pub(crate) struct OpenFileButton;

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

#[derive(Component)]
pub(crate) struct HudLegendText;

#[derive(Component)]
pub(crate) struct BondToleranceSliderTrack;

#[derive(Component)]
pub(crate) struct BondToleranceText;

#[derive(Component)]
pub(crate) struct BondToggleButton;

#[derive(Component)]
pub(crate) struct BondToggleLabel;

#[derive(Component)]
pub(crate) struct BondToleranceFill;

#[derive(Component)]
pub(crate) struct ColorModeButton;

#[derive(Component)]
pub(crate) struct ColorModeLabel;

#[derive(Component)]
pub(crate) struct BondOrderLegendContainer;

#[derive(Component)]
pub(crate) struct BondOrderLegendText;

#[derive(Component)]
pub(crate) struct AtomHoverPanel;

#[derive(Component)]
pub(crate) struct AtomHoverText;

#[derive(Resource, Default, Clone)]
pub(crate) struct AtomHoverCache {
    degree: Vec<usize>,
    ring_atoms: Vec<bool>,
}

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct SelectedAtom {
    pub(crate) index: Option<usize>,
}

#[derive(Component)]
pub(crate) struct AtomSelectionHighlight;

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

#[derive(Resource, Clone)]
pub(crate) struct ColorModeAvailability {
    modes: Vec<AtomColorMode>,
}

impl Default for ColorModeAvailability {
    fn default() -> Self {
        Self {
            modes: vec![AtomColorMode::Element],
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
    slider_fill: Color,
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
            slider_fill: Color::srgb(0.45, 0.72, 0.98),
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
            slider_fill: Color::srgb(0.10, 0.38, 0.90),
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

fn chain_color(chain_id: &str) -> Color {
    let mut hash: u32 = 2166136261;
    for b in chain_id.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    let hue = (hash % 360) as f32;
    Color::hsl(hue, 0.70, 0.55)
}

fn color_mode_label(mode: AtomColorMode) -> &'static str {
    match mode {
        AtomColorMode::Element => "Color: Element",
        AtomColorMode::Chain => "Color: Chain",
        AtomColorMode::Residue => "Color: Residue",
        AtomColorMode::Ring => "Color: Ring",
        AtomColorMode::BondEnv => "Color: Bond Env",
        AtomColorMode::Functional => "Color: Functional",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FunctionalGroupClass {
    Other,
    Carbonyl,
    Hydroxyl,
    Amine,
    Amide,
    Phosphate,
    Halogen,
    AromaticLike,
}

fn functional_group_color(group: FunctionalGroupClass) -> Color {
    match group {
        FunctionalGroupClass::Carbonyl => Color::srgb(0.88, 0.34, 0.30),
        FunctionalGroupClass::Hydroxyl => Color::srgb(0.22, 0.72, 0.95),
        FunctionalGroupClass::Amine => Color::srgb(0.35, 0.52, 0.98),
        FunctionalGroupClass::Amide => Color::srgb(0.55, 0.40, 0.92),
        FunctionalGroupClass::Phosphate => Color::srgb(0.95, 0.66, 0.24),
        FunctionalGroupClass::Halogen => Color::srgb(0.27, 0.80, 0.58),
        FunctionalGroupClass::AromaticLike => Color::srgb(0.94, 0.78, 0.28),
        FunctionalGroupClass::Other => Color::srgb(0.62, 0.64, 0.68),
    }
}

fn functional_group_key(group: FunctionalGroupClass) -> &'static str {
    match group {
        FunctionalGroupClass::Carbonyl => "carbonyl",
        FunctionalGroupClass::Hydroxyl => "hydroxyl",
        FunctionalGroupClass::Amine => "amine",
        FunctionalGroupClass::Amide => "amide",
        FunctionalGroupClass::Phosphate => "phosphate",
        FunctionalGroupClass::Halogen => "halogen",
        FunctionalGroupClass::AromaticLike => "aromatic",
        FunctionalGroupClass::Other => "other",
    }
}

fn build_bond_adjacency(
    atom_count: usize,
    bonds: &[crate::structure::Bond],
) -> Vec<Vec<(usize, u8)>> {
    let mut adjacency = vec![Vec::new(); atom_count];
    for bond in bonds {
        if bond.a >= atom_count || bond.b >= atom_count || bond.a == bond.b {
            continue;
        }
        adjacency[bond.a].push((bond.b, bond.order));
        adjacency[bond.b].push((bond.a, bond.order));
    }
    adjacency
}

fn is_carbonyl_carbon(
    atom_index: usize,
    elements: &[String],
    adjacency: &[Vec<(usize, u8)>],
) -> bool {
    if elements.get(atom_index).map(String::as_str) != Some("C") {
        return false;
    }
    adjacency[atom_index]
        .iter()
        .any(|(n, order)| *order >= 2 && elements.get(*n).map(String::as_str) == Some("O"))
}

fn compute_functional_groups(
    crystal: &Crystal,
    bonds: &[crate::structure::Bond],
    ring_atoms: &[bool],
    bond_env_atoms: &[bool],
) -> Vec<FunctionalGroupClass> {
    let atom_count = crystal.atoms.len();
    let elements = crystal
        .atoms
        .iter()
        .map(|atom| atom.element.to_ascii_uppercase())
        .collect::<Vec<_>>();
    let adjacency = build_bond_adjacency(atom_count, bonds);
    let carbonyl_carbons = (0..atom_count)
        .map(|idx| is_carbonyl_carbon(idx, &elements, &adjacency))
        .collect::<Vec<_>>();

    (0..atom_count)
        .map(|idx| {
            let el = elements[idx].as_str();
            if matches!(el, "F" | "CL" | "BR" | "I") {
                return FunctionalGroupClass::Halogen;
            }
            if el == "P" {
                return FunctionalGroupClass::Phosphate;
            }
            if el == "C" && carbonyl_carbons[idx] {
                return FunctionalGroupClass::Carbonyl;
            }
            if el == "O" {
                let bonded_to_p = adjacency[idx]
                    .iter()
                    .any(|(n, _)| elements.get(*n).map(String::as_str) == Some("P"));
                if bonded_to_p {
                    return FunctionalGroupClass::Phosphate;
                }
                let bonded_to_h = adjacency[idx].iter().any(|(n, order)| {
                    *order == 1 && elements.get(*n).map(String::as_str) == Some("H")
                });
                if bonded_to_h {
                    return FunctionalGroupClass::Hydroxyl;
                }
            }
            if el == "N" {
                let amide_like = adjacency[idx].iter().any(|(n, order)| {
                    *order == 1 && carbonyl_carbons.get(*n).copied().unwrap_or(false)
                });
                if amide_like {
                    return FunctionalGroupClass::Amide;
                }
                return FunctionalGroupClass::Amine;
            }
            if ring_atoms.get(idx).copied().unwrap_or(false)
                && bond_env_atoms.get(idx).copied().unwrap_or(false)
            {
                return FunctionalGroupClass::AromaticLike;
            }
            FunctionalGroupClass::Other
        })
        .collect()
}

fn next_mode(modes: &[AtomColorMode], current: AtomColorMode) -> AtomColorMode {
    if modes.is_empty() {
        return AtomColorMode::Element;
    }
    let pos = modes.iter().position(|mode| *mode == current).unwrap_or(0);
    modes[(pos + 1) % modes.len()]
}

fn compute_ring_atoms(atom_count: usize, bonds: &[crate::structure::Bond]) -> Vec<bool> {
    if atom_count == 0 {
        return Vec::new();
    }
    let mut degree = vec![0_usize; atom_count];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); atom_count];
    for bond in bonds {
        if bond.a >= atom_count || bond.b >= atom_count || bond.a == bond.b {
            continue;
        }
        degree[bond.a] += 1;
        degree[bond.b] += 1;
        adj[bond.a].push(bond.b);
        adj[bond.b].push(bond.a);
    }

    let mut removed = vec![false; atom_count];
    let mut queue = VecDeque::new();
    for (idx, deg) in degree.iter().enumerate() {
        if *deg <= 1 {
            queue.push_back(idx);
        }
    }

    while let Some(node) = queue.pop_front() {
        if removed[node] {
            continue;
        }
        removed[node] = true;
        for &n in &adj[node] {
            if removed[n] {
                continue;
            }
            degree[n] = degree[n].saturating_sub(1);
            if degree[n] == 1 {
                queue.push_back(n);
            }
        }
    }

    removed.into_iter().map(|is_removed| !is_removed).collect()
}

fn compute_bond_env_atoms(atom_count: usize, bonds: &[crate::structure::Bond]) -> Vec<bool> {
    let mut marks = vec![false; atom_count];
    for bond in bonds {
        if bond.order >= 2 {
            if bond.a < atom_count {
                marks[bond.a] = true;
            }
            if bond.b < atom_count {
                marks[bond.b] = true;
            }
        }
    }
    marks
}

fn compute_atom_degree(atom_count: usize, bonds: &[crate::structure::Bond]) -> Vec<usize> {
    let mut degree = vec![0_usize; atom_count];
    for bond in bonds {
        if bond.a < atom_count {
            degree[bond.a] = degree[bond.a].saturating_add(1);
        }
        if bond.b < atom_count {
            degree[bond.b] = degree[bond.b].saturating_add(1);
        }
    }
    degree
}

fn count_unique_non_empty(values: impl Iterator<Item = Option<String>>) -> usize {
    let mut set = HashSet::new();
    for value in values.flatten() {
        if !value.is_empty() {
            set.insert(value);
        }
    }
    set.len()
}

fn compute_available_color_modes(
    crystal: Option<&Crystal>,
    bond_settings: &BondInferenceSettings,
) -> Vec<AtomColorMode> {
    let mut modes = vec![AtomColorMode::Element];
    let Some(crystal) = crystal else {
        return modes;
    };

    let chain_count = count_unique_non_empty(crystal.atoms.iter().map(|a| a.chain_id.clone()));
    if chain_count > 1 {
        modes.push(AtomColorMode::Chain);
    }

    let residue_count = count_unique_non_empty(crystal.atoms.iter().map(|a| a.res_name.clone()));
    if residue_count > 1 {
        modes.push(AtomColorMode::Residue);
    }

    let (bonds, _) = resolve_bonds(crystal, bond_settings);
    if !bonds.is_empty() {
        let ring_atoms = compute_ring_atoms(crystal.atoms.len(), &bonds);
        let ring_count = ring_atoms.iter().filter(|&&v| v).count();
        if ring_count > 0 && ring_count < crystal.atoms.len() {
            modes.push(AtomColorMode::Ring);
        }

        let bond_env_atoms = compute_bond_env_atoms(crystal.atoms.len(), &bonds);
        let env_count = bond_env_atoms.iter().filter(|&&v| v).count();
        if env_count > 0 && env_count < crystal.atoms.len() {
            modes.push(AtomColorMode::BondEnv);
        }

        let functional_groups =
            compute_functional_groups(crystal, &bonds, &ring_atoms, &bond_env_atoms);
        let unique_groups = functional_groups.iter().copied().collect::<HashSet<_>>();
        if unique_groups.len() > 1 {
            modes.push(AtomColorMode::Functional);
        }
    }

    modes
}

pub(crate) fn update_atom_hover_cache(
    crystal: Option<Res<Crystal>>,
    bond_settings: Res<BondInferenceSettings>,
    mut cache: ResMut<AtomHoverCache>,
) {
    let Some(crystal) = crystal else {
        cache.degree.clear();
        cache.ring_atoms.clear();
        return;
    };

    if !crystal.is_changed() && !bond_settings.is_changed() {
        return;
    }

    let (bonds, _) = resolve_bonds(&crystal, &bond_settings);
    cache.degree = compute_atom_degree(crystal.atoms.len(), &bonds);
    cache.ring_atoms = compute_ring_atoms(crystal.atoms.len(), &bonds);
}

fn pick_atom_under_cursor(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    atom_query: &Query<(&GlobalTransform, &AtomIndex), With<AtomEntity>>,
) -> Option<(usize, f32)> {
    let cursor = window.cursor_position()?;
    let mut best: Option<(usize, f32)> = None;
    for (atom_transform, atom_index) in atom_query {
        let Ok(screen) = camera.world_to_viewport(camera_transform, atom_transform.translation())
        else {
            continue;
        };
        let d2 = screen.distance_squared(cursor);
        if best.is_none_or(|(_, prev)| d2 < prev) {
            best = Some((atom_index.0, d2));
        }
    }
    best
}

fn format_atom_info(
    atom_idx: usize,
    crystal: &Crystal,
    cache: &AtomHoverCache,
    selected: bool,
) -> String {
    let Some(atom) = crystal.atoms.get(atom_idx) else {
        return String::new();
    };
    let degree = cache.degree.get(atom_idx).copied().unwrap_or(0);
    let ring = if cache.ring_atoms.get(atom_idx).copied().unwrap_or(false) {
        "yes"
    } else {
        "no"
    };
    let mut lines = Vec::new();
    if selected {
        lines.push("Selected atom".to_string());
    }
    lines.push(format!("Element: {}", atom.element));
    lines.push(format!("Index: {}", atom_idx + 1));
    lines.push(format!("Degree: {}", degree));
    lines.push(format!("Ring: {}", ring));
    if let Some(chain_id) = atom.chain_id.as_deref().filter(|v| !v.is_empty()) {
        lines.push(format!("Chain: {}", chain_id));
    }
    if let Some(res_name) = atom.res_name.as_deref().filter(|v| !v.is_empty()) {
        lines.push(format!("Residue: {}", res_name));
    }
    lines.join("\n")
}

pub(crate) fn update_selected_atom_from_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    atom_query: Query<(&GlobalTransform, &AtomIndex), With<AtomEntity>>,
    ui_interactions: Query<&Interaction, With<Button>>,
    mut selected: ResMut<SelectedAtom>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let ui_active = ui_interactions.iter().any(|i| *i != Interaction::None);
    if ui_active {
        return;
    }
    let (Ok(window), Ok((camera, camera_transform))) = (windows.single(), camera_query.single())
    else {
        return;
    };
    const CLICK_RADIUS_PX: f32 = 16.0;
    let picked = pick_atom_under_cursor(window, camera, camera_transform, &atom_query)
        .filter(|(_, d2)| *d2 <= CLICK_RADIUS_PX * CLICK_RADIUS_PX)
        .map(|(idx, _)| idx);
    selected.index = picked;
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
    Query<'w, 's, &'static mut BackgroundColor, With<BondToleranceFill>>,
);

type HudTextQueries<'w, 's> = (
    Query<'w, 's, &'static mut TextColor, With<HudButtonLabel>>,
    Query<'w, 's, &'static mut TextColor, (With<FileUploadText>, Without<HudButtonLabel>)>,
    Query<'w, 's, &'static mut TextColor, With<HudHelpText>>,
    Query<'w, 's, &'static mut TextColor, With<HudLegendText>>,
    Query<'w, 's, &'static mut TextColor, With<AtomHoverText>>,
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

type ColorModeInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static Children,
    ),
    (Changed<Interaction>, With<ColorModeButton>),
>;

type MainCameraTransformProjectionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static Projection),
    (With<Camera3d>, Without<MoleculeRoot>),
>;

type MainCameraChangedTransformQuery<'w, 's> = Query<
    'w,
    's,
    &'static Transform,
    (With<MainCamera>, Without<GizmoAxisRoot>, Changed<Transform>),
>;

// System to set up file upload UI
pub(crate) fn setup_file_ui(mut commands: Commands, mut font_assets: ResMut<Assets<Font>>) {
    commands.insert_resource(UiTheme::default());
    commands.insert_resource(ColorModeAvailability::default());
    commands.insert_resource(AtomHoverCache::default());
    commands.insert_resource(SelectedAtom::default());
    let p = theme_palette(ThemeMode::Dark);
    let icon_font: Handle<Font> = font_assets.add(
        Font::try_from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../vizmat-app/assets/fonts/fa-solid-900.ttf"
            ))
            .to_vec(),
        )
        .expect("embedded Font Awesome TTF must be valid"),
    );
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
                    OpenFileButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Open File"),
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
                    ColorModeButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Color: Element"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text),
                        HudButtonLabel,
                        ColorModeLabel,
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

                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    BondToggleButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Bonds: On"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text),
                        HudButtonLabel,
                        BondToggleLabel,
                    ));
                });

                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(12.0),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.15)),
                    RelativeCursorPosition::default(),
                    BondToleranceSliderTrack,
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: Val::Percent(25.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(p.slider_fill),
                        BondToleranceFill,
                    ));
                });

                row.spawn((
                    Text::new("1.15"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(p.text),
                    HudButtonLabel,
                    BondToleranceText,
                ));
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
                    Text::new(format!("Drop {} file", SUPPORTED_EXTENSIONS_HELP)),
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
                            Text::new("\u{23FE}"),
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

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(38.0),
                display: Display::None,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor(p.border),
            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.10)),
            BondOrderLegendContainer,
        ))
        .with_children(|legend| {
            legend.spawn((
                Text::new("Bond orders: 1x, 2x"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(p.text_muted),
                HudLegendText,
                BondOrderLegendText,
            ));
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(60.0),
                display: Display::None,
                max_width: Val::Px(320.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor(p.border),
            BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.88)),
            AtomHoverPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(p.text_muted),
                AtomHoverText,
            ));
        });
}

// System to update file upload UI
pub(crate) fn update_file_ui(
    file_drag_drop: Res<crate::io::FileDragDrop>,
    theme: Res<UiTheme>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<FileUploadText>>,
) {
    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = file_drag_drop.status_message.clone();
        *color = match file_drag_drop.status_kind {
            FileStatusKind::Info => TextColor(theme_palette(theme.mode).text),
            FileStatusKind::Success => TextColor(Color::srgb(0.20, 0.72, 0.34)),
            FileStatusKind::Error => TextColor(Color::srgb(0.90, 0.20, 0.22)),
        };
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn bond_tolerance_controls(
    mut settings: ResMut<BondInferenceSettings>,
    crystal: Option<Res<Crystal>>,
    mut interaction_queries: ParamSet<(
        Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BondToggleButton>)>,
        Query<(&Interaction, &RelativeCursorPosition), With<BondToleranceSliderTrack>>,
    )>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<BondToleranceText>>,
        Query<&mut Text, With<BondToggleLabel>>,
    )>,
    mut node_queries: ParamSet<(
        Query<&mut Node, With<BondToleranceSliderTrack>>,
        Query<&mut Node, With<BondToleranceFill>>,
    )>,
    theme: Res<UiTheme>,
    time: Res<Time>,
) {
    const MIN_TOLERANCE: f32 = 1.00;
    const MAX_TOLERANCE: f32 = 1.60;
    const STEP: f32 = 0.02;

    let slider_percent = |v: f32| ((v - MIN_TOLERANCE) / (MAX_TOLERANCE - MIN_TOLERANCE)) * 100.0;
    let value_from_slider =
        |x_norm: f32| MIN_TOLERANCE + x_norm.clamp(0.0, 1.0) * (MAX_TOLERANCE - MIN_TOLERANCE);
    let using_file_bonds =
        settings.enabled && crystal.as_deref().is_some_and(|c| c.has_explicit_bonds());

    for (interaction, mut color) in &mut interaction_queries.p0() {
        match *interaction {
            Interaction::Pressed => {
                settings.enabled = !settings.enabled;
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
            }
            Interaction::Hovered => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));
            }
            Interaction::None => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));
            }
        }
    }

    for (interaction, cursor) in &interaction_queries.p1() {
        if *interaction == Interaction::Pressed && !using_file_bonds {
            if let Some(pos) = cursor.normalized {
                let raw = value_from_slider(pos.x);
                let snapped = (raw / STEP).round() * STEP;
                let snapped = snapped.clamp(MIN_TOLERANCE, MAX_TOLERANCE);
                if (snapped - settings.ui_tolerance_scale).abs() > f32::EPSILON {
                    settings.ui_tolerance_scale = snapped;
                    settings.last_ui_change_secs = time.elapsed_secs_f64();
                }
            }
        }
    }

    if let Ok(mut slider_track) = node_queries.p0().single_mut() {
        slider_track.display = if using_file_bonds {
            Display::None
        } else {
            Display::Flex
        };
    }
    if let Ok(mut text) = text_queries.p0().single_mut() {
        text.0 = if using_file_bonds {
            "File".into()
        } else {
            format!("{:.2}", settings.ui_tolerance_scale)
        };
    }
    if let Ok(mut text) = text_queries.p1().single_mut() {
        text.0 = if !settings.enabled {
            "Bonds: Off".into()
        } else if using_file_bonds {
            "Bonds: On (File)".into()
        } else {
            "Bonds: On (Infer)".into()
        };
    }
    if let Ok(mut fill) = node_queries.p1().single_mut() {
        fill.width = if using_file_bonds {
            Val::Percent(100.0)
        } else {
            Val::Percent(slider_percent(settings.ui_tolerance_scale))
        };
    }
}

pub(crate) fn apply_bond_tolerance_debounce(
    mut settings: ResMut<BondInferenceSettings>,
    time: Res<Time>,
) {
    const APPLY_DELAY_SECS: f64 = 0.20;
    if (settings.tolerance_scale - settings.ui_tolerance_scale).abs() <= f32::EPSILON {
        return;
    }
    if time.elapsed_secs_f64() - settings.last_ui_change_secs >= APPLY_DELAY_SECS {
        settings.tolerance_scale = settings.ui_tolerance_scale;
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
                            ThemeMode::Dark => "\u{23FE}".into(),
                            ThemeMode::Light => "\u{2600}".into(),
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

pub(crate) fn color_mode_button(
    mut mode: ResMut<AtomColorMode>,
    availability: Res<ColorModeAvailability>,
    mut interactions: ColorModeInteractionQuery<'_, '_>,
    theme: Res<UiTheme>,
) {
    for (interaction, mut background, _children) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                *mode = next_mode(&availability.modes, *mode);
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

pub(crate) fn update_color_mode_availability(
    crystal: Option<Res<Crystal>>,
    bond_settings: Res<BondInferenceSettings>,
    mut mode: ResMut<AtomColorMode>,
    mut availability: ResMut<ColorModeAvailability>,
) {
    let next_modes = compute_available_color_modes(crystal.as_deref(), &bond_settings);
    if availability.modes != next_modes {
        availability.modes = next_modes;
    }
    if !availability.modes.contains(&*mode) {
        *mode = AtomColorMode::Element;
    }
}

pub(crate) fn sync_color_mode_label(
    mode: Res<AtomColorMode>,
    mut labels: Query<&mut Text, With<ColorModeLabel>>,
) {
    if !mode.is_changed() {
        return;
    }
    if let Ok(mut text) = labels.single_mut() {
        text.0 = color_mode_label(*mode).into();
    }
}

pub(crate) fn apply_theme_to_atom_hover_panel(
    theme: Res<UiTheme>,
    mut bg_query: Query<&mut BackgroundColor, With<AtomHoverPanel>>,
    mut text_query: Query<&mut TextColor, With<AtomHoverText>>,
) {
    if !theme.is_changed() {
        return;
    }
    let p = theme_palette(theme.mode);
    if let Ok(mut bg) = bg_query.single_mut() {
        bg.0 = p.scene_bg;
    }

    if let Ok(mut text_color) = text_query.single_mut() {
        text_color.0 = p.text; // ← change text color here
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
    for mut fill in &mut themed.bg.p3() {
        *fill = BackgroundColor(p.slider_fill);
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
    for mut color in &mut themed.text.p3() {
        *color = TextColor(p.text_muted);
    }
    for mut color in &mut themed.text.p4() {
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

#[allow(clippy::type_complexity)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handle_open_file_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<OpenFileButton>),
    >,
    mut file_drag_drop: ResMut<crate::io::FileDragDrop>,
    theme: Res<UiTheme>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                let picked = rfd::FileDialog::new()
                    .add_filter("Structure", SUPPORTED_EXTENSIONS)
                    .pick_file();
                if let Some(path) = picked {
                    let ext = path
                        .extension()
                        .map(|s| s.to_string_lossy().to_ascii_lowercase());
                    match std::fs::read_to_string(&path) {
                        Ok(contents) => {
                            let parsed = match ext.as_deref() {
                                Some(ext) => parse_structure_by_extension(ext, &contents),
                                _ => Err(anyhow::anyhow!("Unsupported file extension")),
                            };
                            match parsed {
                                Ok(crystal) => {
                                    let atom_count = crystal.atoms.len();
                                    let file_bond_count =
                                        crystal.bonds.as_ref().map_or(0, Vec::len);
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("structure")
                                        .to_string();
                                    file_drag_drop.dragged_file = Some(path);
                                    file_drag_drop.loaded_crystal = Some(crystal);
                                    file_drag_drop.status_message = if file_bond_count > 0 {
                                        format!(
                                            "Loaded: {name} ({atom_count} atoms, {file_bond_count} file bonds)"
                                        )
                                    } else {
                                        format!("Loaded: {name} ({atom_count} atoms)")
                                    };
                                    file_drag_drop.status_kind = FileStatusKind::Success;
                                }
                                Err(e) => {
                                    file_drag_drop.status_message = format!("Parse error: {e}");
                                    file_drag_drop.status_kind = FileStatusKind::Error;
                                }
                            }
                        }
                        Err(e) => {
                            file_drag_drop.status_message = format!("Read error: {e}");
                            file_drag_drop.status_kind = FileStatusKind::Error;
                        }
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

#[allow(clippy::type_complexity)]
#[cfg(target_arch = "wasm32")]
pub(crate) fn handle_open_file_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<OpenFileButton>),
    >,
    theme: Res<UiTheme>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crystal: Option<Res<Crystal>>,
    bond_settings: Res<BondInferenceSettings>,
    color_mode: Res<AtomColorMode>,
    atom_query: Query<Entity, With<AtomEntity>>,
    bond_query: Query<Entity, With<BondEntity>>,
    molecule_root: Query<Entity, With<MoleculeRoot>>,
    mut last_bond_cfg: Local<Option<(bool, bool, f32, AtomColorMode)>>,
) {
    if let Some(crystal) = crystal {
        let has_file_bonds = crystal.has_explicit_bonds();
        let effective_tolerance = if has_file_bonds {
            0.0
        } else {
            bond_settings.tolerance_scale
        };
        let current_bond_cfg = (
            bond_settings.enabled,
            has_file_bonds,
            effective_tolerance,
            *color_mode,
        );
        let bond_cfg_changed = match *last_bond_cfg {
            Some(prev) => prev != current_bond_cfg,
            None => true,
        };
        *last_bond_cfg = Some(current_bond_cfg);

        if crystal.is_changed() || bond_cfg_changed {
            // Clear existing atoms
            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }
            // Clear existing bonds
            for entity in bond_query.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn new atoms
            if let Ok(root_entity) = molecule_root.single() {
                spawn_atoms(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &crystal,
                    &bond_settings,
                    *color_mode,
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
    bond_settings: &BondInferenceSettings,
    color_mode: AtomColorMode,
    root_entity: Entity,
) {
    // Create a sphere mesh for atoms
    let sphere_mesh = meshes.add(Sphere::new(1.0));
    let bond_mesh = meshes.add(Cylinder::new(1.0, 1.0));

    // Create materials for different elements
    let mut element_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();
    let (bonds, _source) = resolve_bonds(crystal, bond_settings);
    let ring_atoms = compute_ring_atoms(crystal.atoms.len(), &bonds);
    let bond_env_atoms = compute_bond_env_atoms(crystal.atoms.len(), &bonds);
    let functional_groups =
        compute_functional_groups(crystal, &bonds, &ring_atoms, &bond_env_atoms);
    let mut bond_materials: HashMap<u8, Handle<StandardMaterial>> = HashMap::new();

    commands.entity(root_entity).with_children(|parent| {
        for bond in &bonds {
            let a = &crystal.atoms[bond.a];
            let b = &crystal.atoms[bond.b];
            let pa = Vec3::new(a.x, a.y, a.z);
            let pb = Vec3::new(b.x, b.y, b.z);
            let axis = pb - pa;
            let length = axis.length();
            if length <= 1e-5 {
                continue;
            }
            let axis_dir = axis / length;
            let rotation = Quat::from_rotation_arc(Vec3::Y, axis_dir);
            let radius = 0.042;
            let stick_count = match bond.order {
                2 => 2,
                3 => 3,
                _ => 1,
            };
            let lateral_base = axis_dir.cross(Vec3::Y);
            let lateral = if lateral_base.length_squared() > 1e-5 {
                lateral_base.normalize()
            } else {
                axis_dir.cross(Vec3::X).normalize_or_zero()
            };
            let spacing = 0.075;
            let bond_color = match bond.order {
                2 => Color::srgb(0.36, 0.62, 0.86),
                3 => Color::srgb(0.88, 0.57, 0.27),
                _ => Color::srgb(0.65, 0.68, 0.72),
            };
            let bond_material = bond_materials
                .entry(bond.order)
                .or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: bond_color,
                        metallic: 0.0,
                        perceptual_roughness: 0.8,
                        ..default()
                    })
                })
                .clone();
            for idx in 0..stick_count {
                let shift = if stick_count == 1 {
                    0.0
                } else if stick_count == 2 {
                    if idx == 0 {
                        -0.5
                    } else {
                        0.5
                    }
                } else {
                    idx as f32 - 1.0
                };
                let center = (pa + pb) * 0.5 + lateral * spacing * shift;
                parent.spawn((
                    Mesh3d(bond_mesh.clone()),
                    MeshMaterial3d(bond_material.clone()),
                    Transform::from_translation(center)
                        .with_rotation(rotation)
                        .with_scale(Vec3::new(radius, length, radius)),
                    BondEntity,
                    BondOrder(bond.order),
                ));
            }
        }

        // Spawn atoms as 3D spheres
        for (idx, atom) in crystal.atoms.iter().enumerate() {
            let atom_color = match color_mode {
                AtomColorMode::Element => get_element_color(&atom.element),
                AtomColorMode::Chain => atom
                    .chain_id
                    .as_deref()
                    .map(chain_color)
                    .unwrap_or_else(|| get_element_color(&atom.element)),
                AtomColorMode::Residue => atom
                    .res_name
                    .as_deref()
                    .map(get_residue_class_color)
                    .unwrap_or_else(|| get_element_color(&atom.element)),
                AtomColorMode::Ring => {
                    if ring_atoms.get(idx).copied().unwrap_or(false) {
                        Color::srgb(0.94, 0.73, 0.22)
                    } else {
                        Color::srgb(0.34, 0.73, 0.96)
                    }
                }
                AtomColorMode::BondEnv => {
                    if bond_env_atoms.get(idx).copied().unwrap_or(false) {
                        Color::srgb(0.95, 0.46, 0.22)
                    } else {
                        Color::srgb(0.62, 0.64, 0.68)
                    }
                }
                AtomColorMode::Functional => functional_group_color(
                    functional_groups
                        .get(idx)
                        .copied()
                        .unwrap_or(FunctionalGroupClass::Other),
                ),
            };
            let material_key = match color_mode {
                AtomColorMode::Element => format!("E:{}", atom.element),
                AtomColorMode::Chain => format!("C:{}", atom.chain_id.as_deref().unwrap_or("_")),
                AtomColorMode::Residue => format!("R:{}", atom.res_name.as_deref().unwrap_or("_")),
                AtomColorMode::Ring => {
                    let bucket = if ring_atoms.get(idx).copied().unwrap_or(false) {
                        "ring"
                    } else {
                        "nonring"
                    };
                    format!("G:{bucket}")
                }
                AtomColorMode::BondEnv => {
                    let bucket = if bond_env_atoms.get(idx).copied().unwrap_or(false) {
                        "bondenv"
                    } else {
                        "other"
                    };
                    format!("B:{bucket}")
                }
                AtomColorMode::Functional => format!(
                    "F:{}",
                    functional_group_key(
                        functional_groups
                            .get(idx)
                            .copied()
                            .unwrap_or(FunctionalGroupClass::Other)
                    )
                ),
            };
            // Get or create material for this element
            let material = element_materials
                .entry(material_key)
                .or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: atom_color,
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
                AtomIndex(idx),
            ));
        }
    });
}

// System to set up the camera
pub fn setup_cameras(mut commands: Commands, windows: Query<&Window>) {
    let window = windows.single().unwrap();
    let viewport_size = UVec2::new(
        GIZMO_VIEWPORT_SIZE_PX.min(window.physical_width()),
        GIZMO_VIEWPORT_SIZE_PX.min(window.physical_height()),
    );
    let bottom_left_y = window
        .physical_height()
        .saturating_sub(viewport_size.y + GIZMO_VIEWPORT_MARGIN_PX);
    let viewport_position = UVec2::new(GIZMO_VIEWPORT_MARGIN_PX, bottom_left_y);

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

pub(crate) fn update_gizmo_viewport(
    windows: Query<&Window>,
    mut gizmo_camera_query: Query<&mut Camera, With<GizmoCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut camera) = gizmo_camera_query.single_mut() else {
        return;
    };

    let viewport_size = UVec2::new(
        GIZMO_VIEWPORT_SIZE_PX.min(window.physical_width()),
        GIZMO_VIEWPORT_SIZE_PX.min(window.physical_height()),
    );
    let viewport_position = UVec2::new(
        GIZMO_VIEWPORT_MARGIN_PX,
        window
            .physical_height()
            .saturating_sub(viewport_size.y + GIZMO_VIEWPORT_MARGIN_PX),
    );

    camera.viewport = Some(Viewport {
        physical_position: viewport_position,
        physical_size: viewport_size,
        ..default()
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
    camera_query: MainCameraChangedTransformQuery<'_, '_>,
    mut gizmo_query: Query<&mut Transform, (With<GizmoAxisRoot>, Without<MainCamera>)>,
) {
    if let (Ok(camera), Ok(mut gizmo)) = (camera_query.single(), gizmo_query.single_mut()) {
        // Keep axis in world-space orientation while gizmo camera rotates with the main camera.
        gizmo.rotation = camera.rotation.inverse();
    }
}

pub(crate) fn update_bond_order_legend(
    bond_settings: Res<BondInferenceSettings>,
    mut legend_query: Query<&mut Node, With<BondOrderLegendContainer>>,
    mut legend_text_query: Query<&mut Text, With<BondOrderLegendText>>,
    bond_orders: Query<&BondOrder, With<BondEntity>>,
) {
    let Ok(mut legend_node) = legend_query.single_mut() else {
        return;
    };
    let Ok(mut legend_text) = legend_text_query.single_mut() else {
        return;
    };

    if !bond_settings.enabled {
        legend_node.display = Display::None;
        return;
    }

    let mut orders = BTreeSet::new();
    for order in &bond_orders {
        orders.insert(order.0);
    }

    if orders.len() <= 1 {
        legend_node.display = Display::None;
        return;
    }

    legend_node.display = Display::Flex;
    let labels = orders
        .into_iter()
        .map(|order| match order {
            1 => "1x".to_string(),
            2 => "2x".to_string(),
            3 => "3x".to_string(),
            _ => format!("{order}x"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    legend_text.0 = format!("Bond orders: {labels}");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_atom_hover_label(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    atom_query: Query<(&GlobalTransform, &AtomIndex), With<AtomEntity>>,
    crystal: Option<Res<Crystal>>,
    cache: Res<AtomHoverCache>,
    selected: Res<SelectedAtom>,
    mut panel_query: Query<&mut Node, With<AtomHoverPanel>>,
    mut text_query: Query<&mut Text, With<AtomHoverText>>,
) {
    let Ok(mut panel_node) = panel_query.single_mut() else {
        return;
    };
    let Ok(mut panel_text) = text_query.single_mut() else {
        return;
    };
    let Some(crystal) = crystal else {
        panel_node.display = Display::None;
        return;
    };

    if let Some(selected_idx) = selected.index {
        if selected_idx < crystal.atoms.len() {
            panel_node.display = Display::Flex;
            panel_text.0 = format_atom_info(selected_idx, &crystal, &cache, true);
            return;
        }
    }

    let Ok(window) = windows.single() else {
        panel_node.display = Display::None;
        return;
    };
    if window.cursor_position().is_none() {
        panel_node.display = Display::None;
        return;
    }
    let Ok((camera, camera_transform)) = camera_query.single() else {
        panel_node.display = Display::None;
        return;
    };

    const HOVER_RADIUS_PX: f32 = 16.0;
    let Some((atom_idx, d2)) =
        pick_atom_under_cursor(window, camera, camera_transform, &atom_query)
    else {
        panel_node.display = Display::None;
        return;
    };
    if d2 > HOVER_RADIUS_PX * HOVER_RADIUS_PX {
        panel_node.display = Display::None;
        return;
    }

    panel_node.display = Display::Flex;
    panel_text.0 = format_atom_info(atom_idx, &crystal, &cache, false);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_atom_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedAtom>,
    crystal: Option<Res<Crystal>>,
    highlight_entities: Query<Entity, With<AtomSelectionHighlight>>,
    molecule_root: Query<Entity, With<MoleculeRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in &highlight_entities {
        commands.entity(entity).despawn();
    }
    let Some(crystal) = crystal else {
        return;
    };
    let Some(selected_idx) = selected.index else {
        return;
    };
    let Some(atom) = crystal.atoms.get(selected_idx) else {
        return;
    };
    let Ok(root) = molecule_root.single() else {
        return;
    };
    let mesh = meshes.add(Sphere::new(1.0));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.34, 0.66, 0.98, 0.22),
        emissive: Color::srgb(0.26, 0.58, 0.95).into(),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_xyz(atom.x, atom.y, atom.z)
                .with_scale(Vec3::splat(get_element_size(&atom.element) * 1.45)),
            AtomSelectionHighlight,
        ));
    });
}

// Simple camera controls
#[allow(clippy::too_many_arguments)]
pub(crate) fn camera_controls(
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<MoleculeRoot>)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_rig: ResMut<CameraRig>,
    ui_interactions: Query<&Interaction, With<Button>>,
) {
    if let Ok(mut transform) = camera_query.single_mut() {
        let mut zoom_change = 0.0;
        let mut pan_request = Vec2::ZERO;
        let ui_active = ui_interactions.iter().any(|i| *i != Interaction::None);

        const MIN_DISTANCE: f32 = 0.2;
        const MAX_DISTANCE: f32 = 200.0;

        let mut mouse_delta = Vec2::ZERO;
        for motion in mouse_motion_events.read() {
            mouse_delta += motion.delta;
        }

        if !ui_active && mouse_buttons.pressed(MouseButton::Left) {
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
            let mut offset = transform.translation - camera_rig.target;
            offset = yaw_rotation * pitch_rotation * offset;
            transform.translation = camera_rig.target + offset;
            transform.look_at(camera_rig.target, Vec3::Y);
            camera_rig.distance = offset.length().max(MIN_DISTANCE);
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
            let distance = (camera_rig.target - transform.translation)
                .length()
                .max(MIN_DISTANCE);
            let mut forward = (camera_rig.target - transform.translation).normalize_or_zero();
            if forward.length_squared() < f32::EPSILON {
                forward = -transform.forward().normalize_or_zero();
            }
            let yaw_rotation = Quat::from_rotation_y(yaw * rotate_speed * time.delta_secs());
            let rotated_forward = (yaw_rotation * forward).normalize_or_zero();
            camera_rig.target = transform.translation + rotated_forward * distance;
        }

        if !ui_active && mouse_buttons.pressed(MouseButton::Right) {
            pan_request = mouse_delta;
        }

        if !ui_active {
            for wheel in mouse_wheel_events.read() {
                zoom_change -= wheel.y * 0.002;
            }
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
            // Scale keyboard movement with viewing distance so large structures
            // don't feel sluggish and small structures don't feel too twitchy.
            let distance_factor = (distance * 0.25).clamp(0.5, 12.0);
            let step = move_dir.normalize() * move_speed * distance_factor * time.delta_secs();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bond_tolerance_controls_system_runs_without_query_conflicts() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<BondInferenceSettings>();
        app.init_resource::<UiTheme>();

        app.world_mut().spawn((
            BondToleranceSliderTrack,
            Interaction::None,
            RelativeCursorPosition::default(),
            Node::default(),
        ));
        app.world_mut().spawn((
            BondToleranceFill,
            Node::default(),
            BackgroundColor(Color::NONE),
        ));
        app.world_mut().spawn((
            BondToggleButton,
            Interaction::None,
            BackgroundColor(Color::NONE),
        ));
        app.world_mut()
            .spawn((BondToleranceText, Text::new(""), TextColor(Color::WHITE)));
        app.world_mut()
            .spawn((BondToggleLabel, Text::new(""), TextColor(Color::WHITE)));

        app.add_systems(Update, bond_tolerance_controls);
        app.update();
    }
}
