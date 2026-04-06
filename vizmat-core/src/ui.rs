#![allow(clippy::needless_pass_by_value)]

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::view::RenderLayers;
use bevy::ui::FocusPolicy;
use bevy::ui::RelativeCursorPosition;
use crossbeam_channel::{Receiver, Sender};
#[cfg(target_arch = "wasm32")]
use {
    crate::{get_web_theme, set_web_theme},
    gloo::net::http::Request,
    wasm_bindgen_futures::spawn_local,
    web_sys::window,
};

use crate::constants::{get_element_color, get_element_size, get_residue_class_color};
#[allow(unused_imports)]
use crate::formats::parse_structure_by_extension;
#[cfg(not(target_arch = "wasm32"))]
use crate::formats::SUPPORTED_EXTENSIONS;
use crate::formats::SUPPORTED_EXTENSIONS_HELP;
use crate::io::FileStatusKind;
use crate::structure::{
    AtomColorMode, AtomEntity, AtomIndex, Bond, BondCache, BondEntity, BondInferenceSettings,
    BondOrder, LatticeEntity, Site, StructureView,
};

const LAYER_GIZMO: RenderLayers = RenderLayers::layer(1);
const LAYER_CANVAS: RenderLayers = RenderLayers::layer(0);
const GIZMO_VIEWPORT_SIZE_PX: u32 = 200;
const GIZMO_VIEWPORT_MARGIN_PX: u32 = 10;
const DEFAULT_STRUCTURE_PATH: &str = "compounds/water.xyz";
#[cfg(target_arch = "wasm32")]
const DEFAULT_STRUCTURES_ASSET_BASE_URL: &str = "assets/structures";
const DEFAULT_STRUCTURES_REMOTE_BASE_URL: &str =
    "https://raw.githubusercontent.com/syzer/vizmat-structures/main";
const EMBEDDED_STRUCTURE_LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vizmat-app/assets/structures/list.txt"
));

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppUiState {
    #[default]
    Startup,
    Running,
}

#[derive(Component)]
pub(crate) struct StartupScreenRoot;

#[derive(Component)]
pub(crate) struct HiddenOnStartup;

#[derive(Component)]
pub(crate) struct StartupTitleText;

#[derive(Component)]
pub(crate) struct StartupHelpText;

#[derive(Component)]
pub(crate) struct StartupSecondaryHelpText;

#[derive(Component)]
pub(crate) struct StartupIconText;

#[derive(Component)]
pub(crate) struct StartupDropzonePanel;

#[derive(Component)]
pub(crate) struct StartupDropzoneSegment;

#[derive(Resource, Clone)]
pub(crate) struct IconFont(pub(crate) Handle<Font>);

#[derive(Component)]
pub(crate) struct MainCamera;

#[derive(Component)]
pub(crate) struct StructureRoot;

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
pub(crate) struct StructurePickerToggleButton;

#[derive(Component)]
pub(crate) struct StructurePickerPanel;

#[derive(Component)]
pub(crate) struct StructurePickerQueryText;

#[derive(Component)]
pub(crate) struct StructurePickerResultsRoot;

#[derive(Component, Clone)]
pub(crate) struct StructurePickerResultButton {
    pub(crate) path: String,
}

#[derive(Resource, Default)]
pub(crate) struct StructurePickerState {
    pub(crate) entries: Vec<String>,
    pub(crate) query: String,
    pub(crate) visible: bool,
}

// ---- for arcball control ---
#[derive(Resource, Default)]
pub struct ArcballState {
    pub prev_cursor: Option<Vec2>,
}

fn normalize_mouse(pos: Vec2, window: Vec2) -> Vec2 {
    Vec2::new(
        (2.0 * pos.x - window.x) / window.x,
        (window.y - 2.0 * pos.y) / window.y,
    )
}

fn arcball_vector(p: Vec2) -> Vec3 {
    let d2 = p.length_squared();

    if d2 <= 1.0 {
        Vec3::new(p.x, p.y, (1.0 - d2).sqrt())
    } else {
        Vec3::new(p.x, p.y, 0.0).normalize()
    }
}

fn arcball_quat(v0: Vec3, v1: Vec3) -> Quat {
    let cross = v1.cross(v0);
    let dot = v0.dot(v1);

    if cross.length_squared() < 1e-10 {
        Quat::IDENTITY
    } else {
        Quat::from_xyzw(cross.x, cross.y, cross.z, 1.0 + dot).normalize()
    }
}

// end of helpers for arcball ------

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
pub(crate) struct StructureLoadingOverlay;

#[derive(Component)]
pub(crate) struct StructureLoadingTitle;

#[derive(Component)]
pub(crate) struct StructureLoadingTelemetry;

#[derive(Component)]
pub(crate) struct StructureLoadingProgressFill;

#[derive(Debug)]
pub(crate) struct CatalogLoadResult {
    pub(crate) path: String,
    pub(crate) source: String,
    pub(crate) contents: Result<String, String>,
}

#[derive(Resource, Clone)]
pub(crate) struct CatalogLoadChannel {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    sender: Sender<CatalogLoadResult>,
    receiver: Receiver<CatalogLoadResult>,
}

impl Default for CatalogLoadChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

impl CatalogLoadChannel {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn send(&self, result: CatalogLoadResult) {
        let _ = self.sender.send(result);
    }

    pub(crate) fn try_recv(&self) -> Option<CatalogLoadResult> {
        self.receiver.try_recv().ok()
    }
}

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
    sites: &[Site],
    bonds: &[crate::structure::Bond],
    ring_atoms: &[bool],
    bond_env_atoms: &[bool],
) -> Vec<FunctionalGroupClass> {
    let nsites = sites.len();
    let elements = sites
        .iter()
        .map(|atom| atom.element.to_ascii_uppercase())
        .collect::<Vec<_>>();
    let adjacency = build_bond_adjacency(nsites, bonds);
    let carbonyl_carbons = (0..nsites)
        .map(|idx| is_carbonyl_carbon(idx, &elements, &adjacency))
        .collect::<Vec<_>>();

    (0..nsites)
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
    sv: Option<&StructureView>,
    bond_settings: &BondInferenceSettings,
    bond_cache: &mut BondCache,
) -> Vec<AtomColorMode> {
    let mut modes = vec![AtomColorMode::Element];
    let Some(sv) = sv else {
        return modes;
    };
    let sites = sv.sites();

    let chain_count = count_unique_non_empty(sites.iter().map(|a| a.chain_id.clone()));
    if chain_count > 1 {
        modes.push(AtomColorMode::Chain);
    }

    let residue_count = count_unique_non_empty(sites.iter().map(|a| a.res_name.clone()));
    if residue_count > 1 {
        modes.push(AtomColorMode::Residue);
    }

    let bonds = bonds_for_color(sv, bond_settings, bond_cache);
    if !bonds.is_empty() {
        let ring_atoms = compute_ring_atoms(sites.len(), bonds);
        let ring_count = ring_atoms.iter().filter(|&&v| v).count();
        if ring_count > 0 && ring_count < sites.len() {
            modes.push(AtomColorMode::Ring);
        }

        let bond_env_atoms = compute_bond_env_atoms(sites.len(), bonds);
        let env_count = bond_env_atoms.iter().filter(|&&v| v).count();
        if env_count > 0 && env_count < sites.len() {
            modes.push(AtomColorMode::BondEnv);
        }

        let functional_groups =
            compute_functional_groups(&sites, bonds, &ring_atoms, &bond_env_atoms);
        let unique_groups = functional_groups.iter().copied().collect::<HashSet<_>>();
        if unique_groups.len() > 1 {
            modes.push(AtomColorMode::Functional);
        }
    }

    modes
}

fn bonds_for_color<'a>(
    sv: &'a StructureView,
    bond_settings: &BondInferenceSettings,
    bond_cache: &'a mut BondCache,
) -> &'a [Bond] {
    static EMPTY_BONDS: [Bond; 0] = [];

    if bond_settings.enabled {
        let (bonds, _) = bond_cache.resolve_bonds_cached(sv, bond_settings);
        return bonds;
    }

    sv.bonds
        .as_ref()
        .filter(|b| !b.is_empty())
        .map(|b| b.as_slice())
        .unwrap_or(&EMPTY_BONDS)
}

pub(crate) fn update_atom_hover_cache(
    sv: Option<Res<StructureView>>,
    bond_settings: Res<BondInferenceSettings>,
    mut bond_cache: ResMut<BondCache>,
    mut cache: ResMut<AtomHoverCache>,
) {
    let Some(sv) = sv else {
        cache.degree.clear();
        cache.ring_atoms.clear();
        return;
    };
    let nsites = sv.nsites();

    if !sv.is_changed() && !bond_settings.is_changed() {
        return;
    }

    let bonds = bonds_for_color(&sv, &bond_settings, &mut bond_cache);
    cache.degree = compute_atom_degree(nsites, bonds);
    cache.ring_atoms = compute_ring_atoms(nsites, bonds);
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
    idx: usize,
    sv: &StructureView,
    cache: &AtomHoverCache,
    selected: bool,
) -> String {
    let sites = sv.sites();
    let Some(site) = sites.get(idx) else {
        return String::new();
    };
    let degree = cache.degree.get(idx).copied().unwrap_or(0);
    let ring = if cache.ring_atoms.get(idx).copied().unwrap_or(false) {
        "yes"
    } else {
        "no"
    };
    let mut lines = Vec::new();
    if selected {
        lines.push("Selected atom".to_string());
    }
    lines.push(format!("Element: {}", site.element));
    lines.push(format!("Index: {}", idx + 1));
    lines.push(format!("Degree: {}", degree));
    lines.push(format!("Ring: {}", ring));
    if let Some(chain_id) = site.chain_id.as_deref().filter(|v| !v.is_empty()) {
        lines.push(format!("Chain: {}", chain_id));
    }
    if let Some(res_name) = site.res_name.as_deref().filter(|v| !v.is_empty()) {
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
    (With<Camera3d>, Without<StructureRoot>),
>;

type MainCameraChangedTransformQuery<'w, 's> = Query<
    'w,
    's,
    &'static Transform,
    (With<MainCamera>, Without<GizmoAxisRoot>, Changed<Transform>),
>;

type StartupThemeTextQueries<'w, 's> = (
    Query<'w, 's, &'static mut TextColor, With<StartupTitleText>>,
    Query<'w, 's, &'static mut TextColor, With<StartupHelpText>>,
    Query<'w, 's, &'static mut TextColor, With<StartupSecondaryHelpText>>,
    Query<'w, 's, &'static mut TextColor, With<StartupIconText>>,
    Query<'w, 's, &'static mut BackgroundColor, With<StartupDropzonePanel>>,
    Query<'w, 's, &'static mut BackgroundColor, With<StartupDropzoneSegment>>,
);

type StructureLoadingTextQueries<'w, 's> = (
    Query<'w, 's, &'static mut Text, With<StructureLoadingTitle>>,
    Query<'w, 's, &'static mut Text, With<StructureLoadingTelemetry>>,
);

type StructureLoadingNodeQueries<'w, 's> = (
    Query<'w, 's, &'static mut Node, With<StructureLoadingOverlay>>,
    Query<'w, 's, &'static mut Node, With<StructureLoadingProgressFill>>,
);

fn parse_embedded_structure_entries() -> Vec<String> {
    EMBEDDED_STRUCTURE_LIST
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn structure_matches_query(path: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    path.to_ascii_lowercase()
        .contains(&query.trim().to_ascii_lowercase())
}

fn filtered_structure_entries(state: &StructurePickerState) -> Vec<String> {
    state
        .entries
        .iter()
        .filter(|entry| structure_matches_query(entry, &state.query))
        .take(12)
        .cloned()
        .collect()
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn structures_local_dir() -> &'static str {
    option_env!("VIZMAT_STRUCTURES_LOCAL_DIR").unwrap_or(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vizmat-app/assets/structures"
    ))
}

#[cfg(target_arch = "wasm32")]
fn structures_asset_base_url() -> &'static str {
    option_env!("VIZMAT_STRUCTURES_ASSET_BASE_URL").unwrap_or(DEFAULT_STRUCTURES_ASSET_BASE_URL)
}

fn structures_remote_base_url() -> &'static str {
    option_env!("VIZMAT_STRUCTURES_REMOTE_BASE_URL").unwrap_or(DEFAULT_STRUCTURES_REMOTE_BASE_URL)
}

fn structure_remote_url(path: &str) -> String {
    format!("{}/{}", structures_remote_base_url(), path)
}

fn looks_like_html_response(contents: &str) -> bool {
    let trimmed = contents.trim_start();
    let prefix: String = trimmed.chars().take(256).collect();
    let prefix_lower = prefix.to_ascii_lowercase();
    prefix_lower.starts_with("<!doctype")
        || prefix_lower.starts_with("<html")
        || prefix_lower.starts_with("<head")
        || prefix_lower.starts_with("<body")
}

#[cfg(target_arch = "wasm32")]
fn structure_local_asset_url(path: &str) -> String {
    format!("{}/{}", structures_asset_base_url(), path)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn structure_local_asset_path(path: &str) -> PathBuf {
    PathBuf::from(structures_local_dir()).join(path)
}

fn set_catalog_loaded_status(path: &str, atom_count: usize, file_bond_count: usize) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    if file_bond_count > 0 {
        format!("Loaded: {name} ({atom_count} atoms, {file_bond_count} file bonds)")
    } else {
        format!("Loaded: {name} ({atom_count} atoms)")
    }
}

fn parse_and_store_catalog_structure(
    path: &str,
    contents: &str,
    source: &str,
    file_drag_drop: &mut crate::io::FileDragDrop,
) -> bool {
    let ext = path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match parse_structure_by_extension(&ext, contents) {
        Ok(sv) => {
            let atom_count = sv.nsites();
            let file_bond_count = sv.bonds.as_ref().map_or(0, Vec::len);
            file_drag_drop.loaded_structure = Some(sv);
            file_drag_drop.status_message =
                set_catalog_loaded_status(path, atom_count, file_bond_count);
            file_drag_drop.status_kind = FileStatusKind::Success;
            info!(
                "Structure picker: loaded '{path}' from {source} with {atom_count} atoms and {file_bond_count} file bonds"
            );
            true
        }
        Err(err) => {
            warn!("Structure picker: parse error for '{path}' from {source}: {err}");
            file_drag_drop.status_message = format!("Parse error: {err}");
            file_drag_drop.status_kind = FileStatusKind::Error;
            false
        }
    }
}

pub(crate) fn setup_startup_screen(
    mut commands: Commands,
    icon_font: Res<IconFont>,
    theme: Res<UiTheme>,
) {
    let p = theme_palette(theme.mode);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(24.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(-10),
            FocusPolicy::Pass,
            BackgroundColor(Color::NONE),
            StartupScreenRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|content| {
                content.spawn((
                    Text::new("VIZMAT"),
                    TextFont {
                        font_size: 50.0,
                        ..default()
                    },
                    TextLayout::new_with_justify(JustifyText::Center),
                    TextColor(p.text),
                    StartupTitleText,
                ));
                content
                    .spawn((
                        Node {
                            width: Val::Percent(88.0),
                            min_width: Val::Px(260.0),
                            max_width: Val::Px(620.0),
                            padding: UiRect::all(Val::Px(22.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(12.0),
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        BackgroundColor(p.bar_bg_alt),
                        StartupDropzonePanel,
                    ))
                    .with_children(|dropzone| {
                        dropzone
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    left: Val::Px(14.0),
                                    right: Val::Px(14.0),
                                    height: Val::Px(2.0),
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|edge| {
                                for _ in 0..8 {
                                    edge.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            height: Val::Px(2.0),
                                            ..default()
                                        },
                                        BackgroundColor(p.border),
                                        StartupDropzoneSegment,
                                    ));
                                }
                            });

                        dropzone
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    bottom: Val::Px(0.0),
                                    left: Val::Px(14.0),
                                    right: Val::Px(14.0),
                                    height: Val::Px(2.0),
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|edge| {
                                for _ in 0..8 {
                                    edge.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            height: Val::Px(2.0),
                                            ..default()
                                        },
                                        BackgroundColor(p.border),
                                        StartupDropzoneSegment,
                                    ));
                                }
                            });

                        dropzone
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(14.0),
                                    bottom: Val::Px(14.0),
                                    left: Val::Px(0.0),
                                    width: Val::Px(2.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|edge| {
                                for _ in 0..5 {
                                    edge.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            width: Val::Px(2.0),
                                            ..default()
                                        },
                                        BackgroundColor(p.border),
                                        StartupDropzoneSegment,
                                    ));
                                }
                            });

                        dropzone
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(14.0),
                                    bottom: Val::Px(14.0),
                                    right: Val::Px(0.0),
                                    width: Val::Px(2.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|edge| {
                                for _ in 0..5 {
                                    edge.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            width: Val::Px(2.0),
                                            ..default()
                                        },
                                        BackgroundColor(p.border),
                                        StartupDropzoneSegment,
                                    ));
                                }
                            });

                        dropzone.spawn((
                            Text::new("\u{f063}\n\u{f49e}"),
                            TextFont {
                                font: icon_font.0.clone(),
                                font_size: 38.0,
                                ..default()
                            },
                            TextLayout::new_with_justify(JustifyText::Center),
                            TextColor(p.text_muted),
                            StartupIconText,
                        ));
                        dropzone.spawn((
                            Text::new("Drag and drop a file"),
                            TextFont {
                                font_size: 23.0,
                                ..default()
                            },
                            TextLayout::new_with_justify(JustifyText::Center),
                            TextColor(p.text),
                            StartupHelpText,
                        ));
                    });
                content.spawn((
                    Text::new("or click Load Structure"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextLayout::new_with_justify(JustifyText::Center),
                    TextColor(p.text_muted),
                    StartupSecondaryHelpText,
                ));
            });
        });
}

pub(crate) fn cleanup_startup_screen(
    mut commands: Commands,
    startup_query: Query<Entity, With<StartupScreenRoot>>,
) {
    for entity in &startup_query {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn hide_non_startup_controls(mut controls: Query<&mut Node, With<HiddenOnStartup>>) {
    for mut node in &mut controls {
        node.display = Display::None;
    }
}

pub(crate) fn show_non_startup_controls(mut controls: Query<&mut Node, With<HiddenOnStartup>>) {
    for mut node in &mut controls {
        node.display = Display::Flex;
    }
}

pub(crate) fn transition_to_running_on_structure_loaded(
    sv: Option<Res<StructureView>>,
    file_drag_drop: Res<crate::io::FileDragDrop>,
    mut next_ui_state: ResMut<NextState<AppUiState>>,
) {
    if sv.is_some() || file_drag_drop.loaded_structure.is_some() {
        next_ui_state.set(AppUiState::Running);
    }
}

// System to set up file upload UI
pub(crate) fn setup_file_ui(mut commands: Commands, mut font_assets: ResMut<Assets<Font>>) {
    #[cfg(target_arch = "wasm32")]
    let mut theme = UiTheme::default();
    #[cfg(not(target_arch = "wasm32"))]
    let theme = UiTheme::default();
    #[cfg(target_arch = "wasm32")]
    {
        let web_theme = get_web_theme().unwrap_or_default();
        if web_theme.eq_ignore_ascii_case("light") {
            theme.mode = ThemeMode::Light;
        } else if web_theme.eq_ignore_ascii_case("dark") {
            theme.mode = ThemeMode::Dark;
        }
    }
    commands.insert_resource(theme);
    commands.insert_resource(ColorModeAvailability::default());
    commands.insert_resource(AtomHoverCache::default());
    commands.insert_resource(SelectedAtom::default());
    commands.insert_resource(StructurePickerState {
        entries: parse_embedded_structure_entries(),
        query: String::new(),
        visible: false,
    });
    let p = theme_palette(theme.mode);
    let is_mobile = {
        #[cfg(target_arch = "wasm32")]
        let is_mobile = window()
            .and_then(|window| window.navigator().user_agent().ok())
            .is_some_and(|ua| {
                let ua = ua.to_lowercase();
                ua.contains("mobile") || ua.contains("android") || ua.contains("iphone")
            });

        #[cfg(not(target_arch = "wasm32"))]
        let is_mobile = false;

        is_mobile
    };

    let controls_hint = if is_mobile {
        "Touch: one-finger: rotate, two-finger: pan, pinch: zoom"
    } else {
        "Doom-like: W/A/S/D move  Shift sprint  Q/E rotate  LMB rotate  RMB pan  Wheel zoom"
    };

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
    commands.insert_resource(IconFont(icon_font.clone()));
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
                    StructurePickerToggleButton,
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Load Structure"),
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
                    Visibility::Visible,
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    ColorModeButton,
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    ResetCameraButton,
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    LightAttachmentButton { attached: false },
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    BorderColor(p.border),
                    BackgroundColor(p.button_bg),
                    BondToggleButton,
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    BorderColor(p.border),
                    BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.15)),
                    RelativeCursorPosition::default(),
                    BondToleranceSliderTrack,
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    TextColor(p.text),
                    HudButtonLabel,
                    BondToleranceText,
                    HiddenOnStartup,
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
                    Visibility::Visible,
                    TextColor(p.text),
                    FileUploadText,
                    HiddenOnStartup,
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
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                display: Display::None,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(20),
            FocusPolicy::Pass,
            BackgroundColor(Color::NONE),
            StructureLoadingOverlay,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(p.border),
                    BackgroundColor(p.bar_bg_alt),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Calibrating molecule..."),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(p.text),
                        StructureLoadingTitle,
                    ));
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(240.0),
                                height: Val::Px(6.0),
                                border: UiRect::all(Val::Px(1.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Stretch,
                                ..default()
                            },
                            BorderColor(p.border),
                            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.18)),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(35.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(p.slider_fill),
                                StructureLoadingProgressFill,
                            ));
                        });
                    panel.spawn((
                        Text::new("Fetching structure..."),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(p.text_muted),
                        StructureLoadingTelemetry,
                    ));
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(54.0),
                left: Val::Px(10.0),
                width: Val::Px(360.0),
                max_height: Val::Px(320.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor(p.border),
            BackgroundColor(p.bar_bg_alt),
            StructurePickerPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Search structures:"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(p.text_muted),
                HudHelpText,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(p.text),
                StructurePickerQueryText,
            ));
            panel.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                StructurePickerResultsRoot,
            ));
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
                Text::new(controls_hint),
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
fn status_message_for_hud(status: &str, kind: FileStatusKind) -> String {
    match kind {
        FileStatusKind::Info => {
            if status.starts_with("Loading:") {
                "Loading structure...".to_string()
            } else {
                status.to_string()
            }
        }
        FileStatusKind::Success => status.to_string(),
        FileStatusKind::Error => {
            if status.starts_with("Load error") {
                "Could not load structure from remote library. Check connection and try again."
                    .to_string()
            } else if status.starts_with("Read error:") {
                "Could not read structure source. Check path or network and try again.".to_string()
            } else if status.starts_with("Parse error:") {
                "Could not parse that structure file. Try a different file.".to_string()
            } else if status.starts_with("Unsupported file.") {
                format!("Unsupported file. Use {}.", SUPPORTED_EXTENSIONS_HELP)
            } else {
                "Load failed. Try again with another structure.".to_string()
            }
        }
    }
}

pub(crate) fn update_file_ui(
    file_drag_drop: Res<crate::io::FileDragDrop>,
    theme: Res<UiTheme>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<FileUploadText>>,
) {
    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = status_message_for_hud(&file_drag_drop.status_message, file_drag_drop.status_kind);
        *color = match file_drag_drop.status_kind {
            FileStatusKind::Info => TextColor(theme_palette(theme.mode).text),
            FileStatusKind::Success => TextColor(Color::srgb(0.20, 0.72, 0.34)),
            FileStatusKind::Error => TextColor(Color::srgb(0.90, 0.20, 0.22)),
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_structure_loading_overlay(
    file_drag_drop: Res<crate::io::FileDragDrop>,
    time: Res<Time>,
    _theme: Res<UiTheme>,
    mut node_queries: ParamSet<StructureLoadingNodeQueries<'_, '_>>,
    mut text_queries: ParamSet<StructureLoadingTextQueries<'_, '_>>,
) {
    let status = file_drag_drop.status_message.as_str();
    let is_visible = status.starts_with("Loading:");
    let mut overlay_query = node_queries.p0();
    let Ok(mut overlay) = overlay_query.single_mut() else {
        return;
    };
    overlay.display = if is_visible {
        Display::Flex
    } else {
        Display::None
    };
    if !is_visible {
        return;
    }

    for mut text in &mut text_queries.p0() {
        text.0 = "Calibrating molecule...".to_string();
    }
    for mut text in &mut text_queries.p1() {
        text.0 = "Fetching structure...".to_string();
    }
    let mut fill_query = node_queries.p1();
    if let Ok(mut fill) = fill_query.single_mut() {
        let phase = (time.elapsed_secs() * 1.4).sin() * 0.5 + 0.5;
        let width = 25.0 + phase * 55.0;
        fill.width = Val::Percent(width);
    }
}

pub(crate) fn handle_catalog_load_results(
    mut file_drag_drop: ResMut<crate::io::FileDragDrop>,
    catalog_channel: Option<Res<CatalogLoadChannel>>,
) {
    let Some(channel) = catalog_channel else {
        return;
    };
    while let Some(result) = channel.try_recv() {
        match result.contents {
            Ok(contents) => {
                if parse_and_store_catalog_structure(
                    &result.path,
                    &contents,
                    &result.source,
                    &mut file_drag_drop,
                ) {
                    file_drag_drop.dragged_file = None;
                }
            }
            Err(err) => {
                let name = result.path.rsplit('/').next().unwrap_or(&result.path);
                file_drag_drop.status_message = format!("Load error ({name}): {err}");
                file_drag_drop.status_kind = FileStatusKind::Error;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn bond_tolerance_controls(
    mut settings: ResMut<BondInferenceSettings>,
    sv: Option<Res<StructureView>>,
    mut interaction_queries: ParamSet<(
        Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BondToggleButton>)>,
        Query<(&Interaction, &RelativeCursorPosition), With<BondToleranceSliderTrack>>,
    )>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<BondToleranceText>>,
        Query<&mut Text, With<BondToggleLabel>>,
        Query<&mut Visibility, With<BondToleranceText>>,
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
        settings.enabled && sv.as_deref().is_some_and(|c| c.has_explicit_bonds());

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

    let show_tolerance_controls = settings.enabled && !using_file_bonds;
    if let Ok(mut slider_track) = node_queries.p0().single_mut() {
        slider_track.display = if show_tolerance_controls {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut text) = text_queries.p0().single_mut() {
        text.0 = if using_file_bonds {
            "File".into()
        } else {
            format!("{:.2}", settings.ui_tolerance_scale)
        };
    }
    if let Ok(mut value_visibility) = text_queries.p2().single_mut() {
        *value_visibility = if show_tolerance_controls {
            Visibility::Visible
        } else {
            Visibility::Hidden
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
                #[cfg(target_arch = "wasm32")]
                {
                    let web_theme = match theme.mode {
                        ThemeMode::Dark => "dark",
                        ThemeMode::Light => "light",
                    };
                    set_web_theme(web_theme);
                }
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
    sv: Option<Res<StructureView>>,
    bond_settings: Res<BondInferenceSettings>,
    mut bond_cache: ResMut<BondCache>,
    mut mode: ResMut<AtomColorMode>,
    mut availability: ResMut<ColorModeAvailability>,
) {
    // XXX: should this system called every frame?
    let next_modes = compute_available_color_modes(sv.as_deref(), &bond_settings, &mut bond_cache);
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

pub(crate) fn apply_theme_to_startup_screen(
    theme: Res<UiTheme>,
    mut startup_text_queries: ParamSet<StartupThemeTextQueries<'_, '_>>,
) {
    if !theme.is_changed() {
        return;
    }
    let p = theme_palette(theme.mode);
    for mut color in &mut startup_text_queries.p0() {
        *color = TextColor(p.text);
    }
    for mut color in &mut startup_text_queries.p1() {
        *color = TextColor(p.text);
    }
    for mut color in &mut startup_text_queries.p2() {
        *color = TextColor(p.text_muted);
    }
    for mut color in &mut startup_text_queries.p3() {
        *color = TextColor(p.text_muted);
    }
    for mut color in &mut startup_text_queries.p4() {
        *color = BackgroundColor(p.bar_bg_alt);
    }
    for mut color in &mut startup_text_queries.p5() {
        *color = BackgroundColor(p.border);
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

impl Default for CameraRig {
    fn default() -> Self {
        let target = Vec3::ZERO;
        let transform = Transform::from_xyz(5.0, 5.0, 5.0).looking_at(target, Vec3::Y);

        Self {
            target,
            distance: transform.translation.distance(target),
            initial_target: target,
            initial_translation: transform.translation,
            initial_rotation: transform.rotation,
            initial_scale: transform.scale,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct TwoFingersTouchGestureState {
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
}

#[derive(Default)]
pub(crate) enum OneFingerTouchGestureKind {
    #[default]
    Move,
    Down,
}

#[derive(Resource, Default)]
pub(crate) struct OneFingerTouchGestureState {
    pub(crate) kind: OneFingerTouchGestureKind,
    pub(crate) position: Vec2,
}

fn structure_center_and_extents(sv: &StructureView) -> Option<(Vec3, Vec3)> {
    let sites = sv.sites();
    let first = sites.first()?;
    let mut min = Vec3::new(first.x, first.y, first.z);
    let mut max = min;

    for site in &sv.sites() {
        let p = Vec3::new(site.x, site.y, site.z);
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
    sv: Option<&StructureView>,
) {
    let Some(sv) = sv else {
        apply_initial_camera_reset(transform, rig);
        return;
    };
    let Some((center, extents)) = structure_center_and_extents(sv) else {
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

// System to clear existing atoms when new structure is loaded
#[allow(dead_code)]
pub fn clear_old_atoms(mut commands: Commands, atom_query: Query<Entity, With<AtomEntity>>) {
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn load_structure_from_catalog_path(
    path: &str,
    file_drag_drop: &mut crate::io::FileDragDrop,
    catalog_channel: Option<&CatalogLoadChannel>,
) {
    let name = path.rsplit('/').next().unwrap_or(path);
    file_drag_drop.status_message = format!("Loading: {name}");
    file_drag_drop.status_kind = FileStatusKind::Info;

    #[cfg(target_arch = "wasm32")]
    let _ = catalog_channel;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let spawn_remote_fallback =
            |path_owned: String, remote_url: String, sender: CatalogLoadChannel| {
                std::thread::spawn(move || {
                    let result = match reqwest::blocking::get(&remote_url) {
                        Ok(response) => {
                            if response.status().is_success() {
                                response
                                    .text()
                                    .map_err(|e| format!("Read error: {e}"))
                                    .and_then(|text| {
                                        if looks_like_html_response(&text) {
                                            Err(format!(
                                            "Unexpected HTML response while loading {remote_url}"
                                        ))
                                        } else {
                                            Ok(text)
                                        }
                                    })
                            } else {
                                Err(format!(
                                    "HTTP {} while loading remote structure",
                                    response.status()
                                ))
                            }
                        }
                        Err(remote_err) => Err(format!("Network error: {remote_err}")),
                    };
                    sender.send(CatalogLoadResult {
                        path: path_owned,
                        source: "remote fallback".to_string(),
                        contents: result,
                    });
                });
            };

        let local_path = structure_local_asset_path(path);
        match std::fs::read_to_string(&local_path) {
            Ok(contents) => {
                if looks_like_html_response(&contents) {
                    warn!(
                        "Structure picker: local asset '{}' looks like HTML, trying remote fallback",
                        local_path.display()
                    );
                    let Some(channel) = catalog_channel else {
                        file_drag_drop.status_message =
                            "Read error: local asset looks like HTML. Remote fallback unavailable."
                                .to_string();
                        file_drag_drop.status_kind = FileStatusKind::Error;
                        return;
                    };
                    spawn_remote_fallback(
                        path.to_string(),
                        structure_remote_url(path),
                        channel.clone(),
                    );
                } else if parse_and_store_catalog_structure(
                    path,
                    &contents,
                    "local disk",
                    file_drag_drop,
                ) {
                    file_drag_drop.dragged_file = Some(local_path);
                }
            }
            Err(err) => {
                warn!(
                    "Structure picker: read error for '{path}' at '{}': {err}",
                    local_path.display()
                );
                let Some(channel) = catalog_channel else {
                    file_drag_drop.status_message =
                        format!("Read error: {err}. Remote fallback unavailable.");
                    file_drag_drop.status_kind = FileStatusKind::Error;
                    return;
                };
                let path_owned = path.to_string();
                let remote_url = structure_remote_url(path);
                let sender = channel.clone();
                spawn_remote_fallback(path_owned, remote_url, sender);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let path_owned = path.to_string();
        spawn_local(async move {
            let local_url = structure_local_asset_url(&path_owned);
            let remote_url = structure_remote_url(&path_owned);

            async fn fetch_text(url: String) -> anyhow::Result<String> {
                let resp = Request::get(&url).send().await?;
                if !resp.ok() {
                    anyhow::bail!("HTTP {} while loading {url}", resp.status());
                }
                let text = resp.text().await?;
                if looks_like_html_response(&text) {
                    anyhow::bail!("Unexpected HTML response while loading {url}");
                }
                Ok(text)
            }

            let result = match fetch_text(local_url).await {
                Ok(text) => Ok(text),
                Err(err) => {
                    warn!("Structure picker: local URL failed, trying remote fallback: {err}");
                    fetch_text(remote_url).await
                }
            };

            match result {
                Ok(text) => crate::send_event(crate::WebEvent::Drop {
                    name: path_owned,
                    data: text.into_bytes(),
                    mime_type: "text/plain".to_string(),
                }),
                Err(err) => {
                    warn!("Structure picker: web fetch failed for '{path_owned}': {err}");
                    crate::send_event(crate::WebEvent::CatalogLoadError {
                        path: path_owned,
                        message: format!("{err}"),
                    })
                }
            };
        });
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn structure_picker_toggle_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StructurePickerToggleButton>),
    >,
    mut picker: ResMut<StructurePickerState>,
    theme: Res<UiTheme>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                picker.visible = !picker.visible;
                if picker.visible {
                    picker.query.clear();
                }
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

pub(crate) fn structure_picker_keyboard_search(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut picker: ResMut<StructurePickerState>,
    mut file_drag_drop: ResMut<crate::io::FileDragDrop>,
    catalog_channel: Option<Res<CatalogLoadChannel>>,
) {
    if !picker.visible {
        return;
    }

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            Key::Escape => {
                picker.visible = false;
            }
            Key::Backspace => {
                picker.query.pop();
            }
            Key::Enter => {
                if let Some(first) = filtered_structure_entries(&picker).first().cloned() {
                    load_structure_from_catalog_path(
                        &first,
                        &mut file_drag_drop,
                        catalog_channel.as_deref(),
                    );
                    picker.visible = false;
                }
            }
            Key::Character(_) => {
                if let Some(text) = &event.text {
                    // Keep search input simple and predictable for all layouts.
                    if text.chars().all(|ch| !ch.is_control()) {
                        picker.query.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn refresh_structure_picker_panel(
    mut commands: Commands,
    picker: Res<StructurePickerState>,
    mut panel_query: Query<&mut Node, With<StructurePickerPanel>>,
    mut query_text: Query<&mut Text, With<StructurePickerQueryText>>,
    mut results_root_query: Query<(Entity, Option<&Children>), With<StructurePickerResultsRoot>>,
    theme: Res<UiTheme>,
) {
    let Ok(mut panel_node) = panel_query.single_mut() else {
        return;
    };
    panel_node.display = if picker.visible {
        Display::Flex
    } else {
        Display::None
    };

    if !picker.visible {
        return;
    }

    if let Ok(mut text) = query_text.single_mut() {
        text.0 = if picker.query.is_empty() {
            "Type to filter (Enter to load first match, Esc to close)".to_string()
        } else {
            format!("query: {}", picker.query)
        };
    }

    if !picker.is_changed() && !theme.is_changed() {
        return;
    }

    let Ok((results_root, maybe_children)) = results_root_query.single_mut() else {
        return;
    };
    if let Some(children) = maybe_children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let palette = theme_palette(theme.mode);
    for path in filtered_structure_entries(&picker) {
        let label = if path == DEFAULT_STRUCTURE_PATH {
            format!("DEFAULT · {path}")
        } else {
            path.clone()
        };
        commands.entity(results_root).with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(palette.border),
                    BackgroundColor(palette.button_bg),
                    StructurePickerResultButton { path: path.clone() },
                    HudButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette.text),
                        HudButtonLabel,
                    ));
                });
        });
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn structure_picker_result_buttons(
    mut interaction_query: Query<
        (
            &Interaction,
            &StructurePickerResultButton,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<StructurePickerResultButton>),
    >,
    mut picker: ResMut<StructurePickerState>,
    mut file_drag_drop: ResMut<crate::io::FileDragDrop>,
    catalog_channel: Option<Res<CatalogLoadChannel>>,
    theme: Res<UiTheme>,
) {
    for (interaction, selected, mut background) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                load_structure_from_catalog_path(
                    &selected.path,
                    &mut file_drag_drop,
                    catalog_channel.as_deref(),
                );
                picker.visible = false;
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

// System to handle button click to load default structure
#[allow(clippy::type_complexity)]
pub(crate) fn handle_load_default_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<LoadDefaultButton>),
    >,
    mut commands: Commands,
    sv: Option<Res<StructureView>>,
    theme: Res<UiTheme>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                // Load default water structure
                if sv.is_none() {
                    crate::io::load_default_structure(commands.reborrow());
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
                                Ok(sv) => {
                                    let atom_count = sv.nsites();
                                    let file_bond_count = sv.bonds.as_ref().map_or(0, Vec::len);
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("structure")
                                        .to_string();
                                    file_drag_drop.dragged_file = Some(path);
                                    file_drag_drop.loaded_structure = Some(sv);
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

// System to respawn atoms when structure changes
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sv: Option<Res<StructureView>>,
    bond_settings: Res<BondInferenceSettings>,
    color_mode: Res<AtomColorMode>,
    mut bond_cache: ResMut<BondCache>,
    atom_query: Query<Entity, With<AtomEntity>>,
    bond_query: Query<Entity, With<BondEntity>>,
    lattice_query: Query<Entity, With<LatticeEntity>>,
    structure_root: Query<Entity, With<StructureRoot>>,
    mut last_bond_cfg: Local<Option<(bool, bool, f32, AtomColorMode)>>,
) {
    if let Some(sv) = sv {
        let has_file_bonds = sv.has_explicit_bonds();
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

        if sv.is_changed() || bond_cfg_changed {
            // Clear existing atoms
            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }
            // Clear existing bonds
            for entity in bond_query.iter() {
                commands.entity(entity).despawn();
            }
            // Clear existing lattice
            for entity in lattice_query.iter() {
                commands.entity(entity).despawn();
            }

            if matches!(sv.inner, ccmat_core::Structure::Crystal(_)) {
                spawn_lattice(&mut commands, &mut materials, &mut meshes, &sv);
            }

            // Spawn new atoms
            if let Ok(root_entity) = structure_root.single() {
                spawn_atoms(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &sv,
                    &bond_settings,
                    *color_mode,
                    &mut bond_cache,
                    root_entity,
                );
            }

            println!("Scene updated with new crystal structure");
        }
    }
}

fn spawn_lattice(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    sv: &StructureView,
) {
    // create cell axis
    let ccmat_core::Structure::Crystal(crystal) = &sv.inner else {
        unreachable!("crystal must have lattice")
    };
    let latt = &crystal.lattice(); // & to avoid clone
    let (a, b, c) = (latt.a(), latt.b(), latt.c());
    let a = Vec3::new(
        f64::from(a[0]) as f32,
        f64::from(a[1]) as f32,
        f64::from(a[2]) as f32,
    );
    let b = Vec3::new(
        f64::from(b[0]) as f32,
        f64::from(b[1]) as f32,
        f64::from(b[2]) as f32,
    );
    let c = Vec3::new(
        f64::from(c[0]) as f32,
        f64::from(c[1]) as f32,
        f64::from(c[2]) as f32,
    );
    let mesh = create_wireframe_mesh(&a, &b, &c);
    let mesh = meshes.add(mesh);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        LatticeEntity,
    ));
}

#[allow(clippy::too_many_arguments)]
// Helper function to spawn atoms
fn spawn_atoms(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    sv: &StructureView,
    bond_settings: &BondInferenceSettings,
    color_mode: AtomColorMode,
    bond_cache: &mut BondCache,
    root_entity: Entity,
) {
    // Create a sphere mesh for atoms
    let sphere_mesh = meshes.add(Sphere::new(1.0));
    let bond_mesh = meshes.add(Cylinder::new(1.0, 1.0));

    let sites = sv.sites();
    let nsites = sites.len();

    // Create materials for different elements
    let mut element_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();
    let (render_bonds, _source) = bond_cache.resolve_bonds_cached(sv, bond_settings);
    let color_bonds = if bond_settings.enabled {
        render_bonds
    } else {
        sv.bonds
            .as_ref()
            .filter(|b| !b.is_empty())
            .map(|b| b.as_slice())
            .unwrap_or(&[])
    };
    let ring_atoms = compute_ring_atoms(nsites, color_bonds);
    let bond_env_atoms = compute_bond_env_atoms(nsites, color_bonds);
    let functional_groups =
        compute_functional_groups(&sites, color_bonds, &ring_atoms, &bond_env_atoms);
    let mut bond_materials: HashMap<u8, Handle<StandardMaterial>> = HashMap::new();

    commands.entity(root_entity).with_children(|parent| {
        for bond in render_bonds {
            let a = &sites[bond.a];
            let b = &sites[bond.b];
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
        for (idx, site) in sites.iter().enumerate() {
            let atom_color = match color_mode {
                AtomColorMode::Element => get_element_color(&site.element),
                AtomColorMode::Chain => site
                    .chain_id
                    .as_deref()
                    .map(chain_color)
                    .unwrap_or_else(|| get_element_color(&site.element)),
                AtomColorMode::Residue => site
                    .res_name
                    .as_deref()
                    .map(get_residue_class_color)
                    .unwrap_or_else(|| get_element_color(&site.element)),
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
                AtomColorMode::Element => format!("E:{}", site.element),
                AtomColorMode::Chain => format!("C:{}", site.chain_id.as_deref().unwrap_or("_")),
                AtomColorMode::Residue => format!("R:{}", site.res_name.as_deref().unwrap_or("_")),
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
                Transform::from_xyz(site.x, site.y, site.z)
                    .with_scale(Vec3::splat(get_element_size(&site.element))),
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

    commands.spawn((
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        StructureRoot,
    ));

    // Spawn cameras
    let camera_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                ..default()
            },
            Projection::default(),
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
    commands.insert_resource(CameraRig::default());
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
    sv: Option<Res<StructureView>>,
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
    let Some(sv) = sv else {
        panel_node.display = Display::None;
        return;
    };

    if let Some(selected_idx) = selected.index {
        if selected_idx < sv.nsites() {
            panel_node.display = Display::Flex;
            panel_text.0 = format_atom_info(selected_idx, &sv, &cache, true);
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
    panel_text.0 = format_atom_info(atom_idx, &sv, &cache, false);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_atom_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedAtom>,
    sv: Option<Res<StructureView>>,
    highlight_entities: Query<Entity, With<AtomSelectionHighlight>>,
    structure_root: Query<Entity, With<StructureRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in &highlight_entities {
        commands.entity(entity).despawn();
    }
    let Some(sv) = sv else {
        return;
    };
    let Some(selected_idx) = selected.index else {
        return;
    };
    let sites = sv.sites();
    let Some(site) = sites.get(selected_idx) else {
        return;
    };
    let Ok(root) = structure_root.single() else {
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
            Transform::from_xyz(site.x, site.y, site.z)
                .with_scale(Vec3::splat(get_element_size(&site.element) * 1.45)),
            AtomSelectionHighlight,
        ));
    });
}

// Simple camera controls
#[allow(clippy::too_many_arguments)]
pub(crate) fn camera_controls(
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<StructureRoot>)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut one_finger_touch_gestures: ResMut<OneFingerTouchGestureState>,
    mut two_fingers_touch_gestures: ResMut<TwoFingersTouchGestureState>,
    mut camera_rig: ResMut<CameraRig>,
    windows: Query<&Window>,
    ui_interactions: Query<&Interaction, With<Button>>,
    mut state: ResMut<ArcballState>,
) {
    if let Ok(mut transform) = camera_query.single_mut() {
        let mut zoom_change = 0.0;

        let cursor_pos = one_finger_touch_gestures.position;
        let touch_pan = two_fingers_touch_gestures.pan;
        let touch_zoom = two_fingers_touch_gestures.zoom;
        one_finger_touch_gestures.position = Vec2::ZERO;
        two_fingers_touch_gestures.pan = Vec2::ZERO;
        two_fingers_touch_gestures.zoom = 0.0;

        let ui_active = ui_interactions.iter().any(|i| *i != Interaction::None);

        const MIN_DISTANCE: f32 = 0.2;
        const MAX_DISTANCE: f32 = 200.0;

        let mut mouse_delta = Vec2::ZERO;
        for motion in mouse_motion_events.read() {
            mouse_delta += motion.delta;
        }

        for wheel in mouse_wheel_events.read() {
            zoom_change -= wheel.y * 0.004;
        }

        if ui_active {
            return;
        }

        debug!("cursor_pos: {cursor_pos}");
        if cursor_pos != vec2(0.0, 0.0) {
            let Ok(window) = windows.single() else { return };
            let window_size = Vec2::new(window.width(), window.height());

            if matches!(
                one_finger_touch_gestures.kind,
                OneFingerTouchGestureKind::Down
            ) {
                state.prev_cursor = Some(cursor_pos);
                return;
            }

            debug!("cursor: {cursor_pos}");
            debug!("state.prev_cursor: {:?}", state.prev_cursor);
            if let Some(prev) = state.prev_cursor {
                let prev = normalize_mouse(prev, window_size);
                let curr = normalize_mouse(cursor_pos, window_size);

                let v0 = arcball_vector(prev);
                let v1 = arcball_vector(curr);

                let cam_rot = transform.rotation;
                let rotation_cam = arcball_quat(v0, v1);
                let rotation_cam = rotation_cam.normalize();
                let rotation_world = cam_rot * rotation_cam * cam_rot.inverse();
                let rotation_world = rotation_world.normalize();

                let mut offset = transform.translation - camera_rig.target;

                offset = rotation_world * offset;

                transform.translation = camera_rig.target + offset;
                transform.rotation = (rotation_world * transform.rotation).normalize();

                state.prev_cursor = Some(cursor_pos);
            }
        }

        if (mouse_buttons.pressed(MouseButton::Left)) && !keyboard.pressed(KeyCode::ShiftLeft) {
            // This is a direct-view arcball control implementation.
            let Ok(window) = windows.single() else { return };
            let Some(cursor) = window.cursor_position() else {
                return;
            };
            if mouse_buttons.just_pressed(MouseButton::Left) {
                state.prev_cursor = Some(cursor);
                return;
            }

            let window_size = Vec2::new(window.width(), window.height());
            if let Some(prev) = state.prev_cursor {
                // mouse event
                let prev = normalize_mouse(prev, window_size);
                let curr = normalize_mouse(cursor, window_size);

                let v0 = arcball_vector(prev);
                let v1 = arcball_vector(curr);

                let cam_rot = transform.rotation;
                let rotation_cam = arcball_quat(v0, v1);
                let rotation_cam = rotation_cam.normalize();
                let rotation_world = cam_rot * rotation_cam * cam_rot.inverse();
                let rotation_world = rotation_world.normalize();

                let mut offset = transform.translation - camera_rig.target;

                offset = rotation_world * offset;

                transform.translation = camera_rig.target + offset;
                transform.rotation = (rotation_world * transform.rotation).normalize();

                state.prev_cursor = Some(cursor);
            }
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            // Pan
            let pan_request = mouse_delta;

            // Keep camera offset updated relative to target.
            let mut offset = transform.translation - camera_rig.target;
            if offset.length_squared() < f32::EPSILON {
                offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
            }

            let distance = offset.length().max(MIN_DISTANCE);
            let right = transform.right();
            let up = transform.up();

            if pan_request != Vec2::ZERO {
                let pan_speed = 0.0005 * distance;
                let pan_offset = (-pan_request.x * right + pan_request.y * up) * pan_speed;
                camera_rig.target += pan_offset;
            }

            let direction = offset.normalize_or_zero();
            offset = if direction.length_squared() > 0.0 {
                direction * distance
            } else {
                // TODO: ??? why needed?
                Vec3::new(0.0, 0.0, distance)
            };

            transform.translation = camera_rig.target + offset;
        }

        info!("touch_pan: {touch_pan}");
        if touch_pan != vec2(0.0, 0.0) {
            // Pan
            let pan_request = touch_pan;

            // Keep camera offset updated relative to target.
            let mut offset = transform.translation - camera_rig.target;
            if offset.length_squared() < f32::EPSILON {
                offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
            }

            let distance = offset.length().max(MIN_DISTANCE);
            let right = transform.right();
            let up = transform.up();

            if pan_request != Vec2::ZERO {
                let pan_speed = 0.0005 * distance;
                let pan_offset = (-pan_request.x * right + pan_request.y * up) * pan_speed;
                camera_rig.target += pan_offset;
            }

            let direction = offset.normalize_or_zero();
            offset = if direction.length_squared() > 0.0 {
                direction * distance
            } else {
                // TODO: ??? why needed?
                Vec3::new(0.0, 0.0, distance)
            };

            transform.translation = camera_rig.target + offset;
        }

        if zoom_change != 0.0 {
            // Keep camera offset updated relative to target.
            let mut offset = transform.translation - camera_rig.target;
            if offset.length_squared() < f32::EPSILON {
                offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
            }
            let direction = offset.normalize_or_zero();
            let mut distance = offset.length().max(MIN_DISTANCE);
            // TODO: speed still not very ideal, maybe better to use exp scale on zoom change
            if zoom_change != 0.0 {
                let factor = (1.0 + zoom_change).clamp(0.2, 5.0);
                distance = (distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
            }
            camera_rig.distance = distance;
            offset = direction * distance;
            transform.translation = camera_rig.target + offset;
        }

        if touch_zoom != 0.0 {
            // Keep camera offset updated relative to target.
            let mut offset = transform.translation - camera_rig.target;
            if offset.length_squared() < f32::EPSILON {
                offset = Vec3::new(0.0, 0.0, camera_rig.distance.max(1.0));
            }
            let direction = offset.normalize_or_zero();
            let mut distance = offset.length().max(MIN_DISTANCE);
            let factor = (1.0 - touch_zoom).clamp(0.2, 5.0);
            distance = (distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
            camera_rig.distance = distance;
            offset = direction * distance;
            transform.translation = camera_rig.target + offset;
        }

        if keyboard.pressed(KeyCode::KeyQ)
            || keyboard.pressed(KeyCode::KeyE)
            || keyboard.pressed(KeyCode::KeyW)
            || keyboard.pressed(KeyCode::KeyS)
            || keyboard.pressed(KeyCode::KeyA)
            || keyboard.pressed(KeyCode::KeyD)
            || keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight)
        {
            const ROTATE_SPEED: f32 = 1.8;
            const MOVE_SPEED: f32 = 5.0;
            const SPRINT_SPEED: f32 = 10.0;
            // --- YAW rotation (Q/E) ---
            let mut offset = transform.translation - camera_rig.target;
            let mut yaw = 0.0;
            if keyboard.pressed(KeyCode::KeyQ) {
                yaw += 1.0;
            }
            if keyboard.pressed(KeyCode::KeyE) {
                yaw -= 1.0;
            }
            let up = *transform.up();
            if yaw != 0.0 {
                let yaw_rotation =
                    Quat::from_axis_angle(up, yaw * ROTATE_SPEED * time.delta_secs());
                offset = yaw_rotation * offset;
                transform.translation = camera_rig.target + offset;
                transform.look_at(camera_rig.target, up);
            }

            // --- WASD movement ---
            let forward = (-offset).normalize_or_zero();
            let right = transform.right();
            let mut move_dir = Vec3::ZERO;
            if keyboard.pressed(KeyCode::KeyW) {
                move_dir += forward;
            }
            if keyboard.pressed(KeyCode::KeyS) {
                move_dir -= forward;
            }
            if keyboard.pressed(KeyCode::KeyD) {
                move_dir += *right;
            }
            if keyboard.pressed(KeyCode::KeyA) {
                move_dir -= *right;
            }

            if move_dir.length_squared() > 0.0 {
                let sprint =
                    keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
                let move_speed = if sprint { SPRINT_SPEED } else { MOVE_SPEED };
                let distance = offset.length().max(MIN_DISTANCE);
                let distance_factor = (distance * 0.25).clamp(0.5, 12.0);
                let step = move_dir.normalize() * move_speed * distance_factor * time.delta_secs();
                camera_rig.target += step;
                transform.translation += step;
            }
        }
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
    sv: Option<Res<StructureView>>,
    camera_entity: Option<Res<MainCameraEntity>>,
    mut camera_query: MainCameraTransformProjectionQuery<'_, '_>,
    mut camera_rig: Option<ResMut<CameraRig>>,
    mut structure_query: Query<&mut Transform, (With<StructureRoot>, Without<Camera3d>)>,
) {
    let Some(sv) = sv else {
        return;
    };
    if !sv.is_changed() {
        return;
    }
    let (Some(camera_entity), Some(rig)) = (camera_entity.as_deref(), camera_rig.as_deref_mut())
    else {
        return;
    };

    if let Ok((mut transform, projection)) = camera_query.get_mut(camera_entity.0) {
        apply_framed_camera_reset(&mut transform, projection, rig, Some(&sv));
    }

    if let Ok(mut structure_transform) = structure_query.single_mut() {
        structure_transform.rotation = Quat::IDENTITY;
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
    mut structure_query: Query<&mut Transform, (With<StructureRoot>, Without<Camera3d>)>,
    sv: Option<Res<StructureView>>,
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
                        apply_framed_camera_reset(&mut transform, projection, rig, sv.as_deref());
                    }
                }
                if let Ok(mut structure_transform) = structure_query.single_mut() {
                    structure_transform.rotation = Quat::IDENTITY;
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

// create box frames as the cell
fn create_wireframe_mesh(a: &Vec3, b: &Vec3, c: &Vec3) -> Mesh {
    // look at the direction of c
    let vertices = vec![
        // bottom
        [0., 0., 0.],
        [a.x, a.y, a.z],
        [a.x + b.x, a.y + b.y, a.z + b.z],
        [b.x, b.y, b.z],
        // // top,
        [c.x, c.y, c.z],
        [c.x + a.x, c.y + a.y, c.z + a.z],
        [c.x + a.x + b.x, c.y + a.y + b.y, c.z + a.z + b.z],
        [c.x + b.x, c.x + b.y, c.z + b.z],
    ];

    let indices = vec![
        0, 1, 1, 2, 2, 3, 3, 0, // bottom
        4, 5, 5, 6, 6, 7, 7, 4, // top
        0, 4, 1, 5, 2, 6, 3, 7, // sides
    ];

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use ccmat_core::{atomic_number, sites_cart_coord, MoleculeBuilder};

    use super::*;
    use crate::io::FileDragDrop;
    use crate::structure::StructureView;

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

    #[test]
    fn startup_screen_spawns_hello_label() {
        let mut app = App::new();
        app.insert_resource(UiTheme::default());
        app.insert_resource(IconFont(Handle::<Font>::default()));
        app.add_systems(Startup, setup_startup_screen);
        app.update();

        let hello_count = app
            .world()
            .iter_entities()
            .filter_map(|entity| entity.get::<Text>())
            .filter(|text| text.0.contains("Load Structure"))
            .count();
        assert_eq!(
            hello_count, 1,
            "startup screen should show startup instructions"
        );
    }

    #[test]
    fn startup_state_transitions_to_running_when_structure_is_loaded() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(FileDragDrop::default());
        app.init_state::<AppUiState>();
        app.add_systems(Update, transition_to_running_on_structure_loaded);

        let sites = sites_cart_coord![
            (0.0, 0.0, 0.0), atomic_number!(O);
        ];
        let mol = MoleculeBuilder::new().with_sites(sites).build_uncheck();
        let s = StructureView {
            inner: ccmat_core::Structure::Molecule(mol),
            bonds: None,
            chain_ids: None,
            residues: None,
        };
        app.world_mut()
            .resource_mut::<FileDragDrop>()
            .loaded_structure = Some(s);

        app.update();
        app.update();

        let state = app.world().resource::<State<AppUiState>>();
        assert_eq!(state.get(), &AppUiState::Running);
    }

    #[test]
    fn startup_screen_is_cleaned_up_after_transition() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(UiTheme::default());
        app.insert_resource(IconFont(Handle::<Font>::default()));
        app.insert_resource(FileDragDrop::default());
        app.init_state::<AppUiState>();
        app.add_systems(OnEnter(AppUiState::Startup), setup_startup_screen);
        app.add_systems(OnExit(AppUiState::Startup), cleanup_startup_screen);
        app.add_systems(Update, transition_to_running_on_structure_loaded);

        app.update();
        let startup_count_before = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<StartupScreenRoot>())
            .count();
        assert_eq!(startup_count_before, 1);

        let sites = sites_cart_coord![
            (0.0, 0.0, 0.0), atomic_number!(O);
        ];
        let mol = MoleculeBuilder::new().with_sites(sites).build_uncheck();
        let s = StructureView {
            inner: ccmat_core::Structure::Molecule(mol),
            bonds: None,
            chain_ids: None,
            residues: None,
        };
        app.world_mut()
            .resource_mut::<FileDragDrop>()
            .loaded_structure = Some(s);

        app.update();
        app.update();

        let startup_count_after = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<StartupScreenRoot>())
            .count();
        assert_eq!(startup_count_after, 0);
    }

    #[test]
    fn startup_theme_system_runs_without_query_conflicts() {
        let mut app = App::new();
        app.insert_resource(UiTheme {
            mode: ThemeMode::Dark,
        });
        app.world_mut()
            .spawn((StartupTitleText, TextColor(Color::WHITE)));
        app.world_mut()
            .spawn((StartupHelpText, TextColor(Color::WHITE)));

        app.add_systems(Update, apply_theme_to_startup_screen);

        app.world_mut().resource_mut::<UiTheme>().mode = ThemeMode::Light;
        app.update();
    }

    #[test]
    fn hud_status_message_is_user_friendly_for_errors() {
        let msg = status_message_for_hud("Read error: missing file", FileStatusKind::Error);
        assert!(msg.contains("Could not read structure source"));
        let msg = status_message_for_hud("Parse error: bad token", FileStatusKind::Error);
        assert!(msg.contains("Could not parse"));
    }

    #[test]
    fn structure_loading_overlay_system_runs_without_query_conflicts() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.insert_resource(UiTheme {
            mode: ThemeMode::Dark,
        });
        app.insert_resource(crate::io::FileDragDrop::default());

        app.world_mut()
            .spawn((StructureLoadingOverlay, Node::default()));
        app.world_mut()
            .spawn((StructureLoadingTitle, Text::new("")));
        app.world_mut()
            .spawn((StructureLoadingTelemetry, Text::new("")));
        app.world_mut()
            .spawn((StructureLoadingProgressFill, Node::default()));
        app.add_systems(Update, update_structure_loading_overlay);
        app.update();
    }
}
