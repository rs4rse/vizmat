use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo::utils::window;
#[cfg(target_arch = "wasm32")]
use web_sys::Event;

use super::{
    themed_button_bg, CatalogLoadChannel, HudButton, HudButtonLabel, ThemeMode, TouchGestureState,
    UiTheme, DEFAULT_STRUCTURE_PATH,
};

#[derive(Component)]
pub(crate) struct StructurePickerToggleButton;

#[derive(Component)]
pub(crate) struct StructurePickerPanel;

#[derive(Component)]
pub(crate) struct StructurePickerQueryText;

#[derive(Component)]
pub(crate) struct StructurePickerQueryIcon;

#[derive(Component)]
pub(crate) struct StructurePickerQueryBar;

#[derive(Component)]
pub(crate) struct StructurePickerQueryCaret;

#[derive(Component)]
pub(crate) struct StructurePickerResultsRoot;

#[derive(Component)]
pub(crate) struct StructurePickerResultsScroll;

#[derive(Component)]
pub(crate) struct StructurePickerResultsLayout;

#[derive(Component)]
pub(crate) struct StructurePickerScrollbarTrack;

#[derive(Component)]
pub(crate) struct StructurePickerScrollbarThumb;

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

#[derive(Resource, Default)]
pub(crate) struct StructurePickerSelectionState {
    pending: Option<(Entity, String)>,
    suppress_click_frames: u8,
}

impl StructurePickerSelectionState {
    fn consume_suppression(&mut self) -> bool {
        if self.suppress_click_frames > 0 {
            self.suppress_click_frames = self.suppress_click_frames.saturating_sub(1);
            true
        } else {
            false
        }
    }

    fn suppress_for_touch_drag(&mut self) {
        self.suppress_click_frames = self.suppress_click_frames.max(2);
    }

    fn is_suppressed(&self) -> bool {
        self.suppress_click_frames > 0
    }
}

#[derive(Resource)]
pub(crate) struct StructurePickerCaretState {
    timer: Timer,
    pub(crate) visible: bool,
}

impl Default for StructurePickerCaretState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.55, TimerMode::Repeating),
            visible: true,
        }
    }
}

const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_TRACK_GAP: f32 = 4.0;
const MIN_SCROLLBAR_THUMB_PX: f32 = 18.0;

fn picker_scrollbar_colors(mode: ThemeMode) -> (Color, Color) {
    match mode {
        ThemeMode::Dark => (
            Color::srgba(0.26, 0.30, 0.36, 0.52),
            Color::srgba(0.60, 0.66, 0.74, 0.88),
        ),
        ThemeMode::Light => (
            Color::srgba(0.72, 0.76, 0.82, 0.52),
            Color::srgba(0.34, 0.39, 0.50, 0.88),
        ),
    }
}

pub(crate) fn set_structure_picker_keyboard_active(visible: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = visible;
    }

    #[cfg(target_arch = "wasm32")]
    let window = window();

    #[cfg(target_arch = "wasm32")]
    let event_name = if visible {
        "vizmat-structure-picker-open"
    } else {
        "vizmat-structure-picker-close"
    };

    #[cfg(target_arch = "wasm32")]
    match Event::new(event_name) {
        Ok(event) => {
            let _ = window.dispatch_event(&event);
        }
        Err(err) => {
            warn!("Failed to notify picker keyboard visibility for '{event_name}': {err:?}");
        }
    }
}

pub(crate) fn setup_structure_picker_panel(
    commands: &mut Commands,
    theme: &UiTheme,
    icon_font: &Handle<Font>,
) {
    let palette = super::theme_palette(theme.mode);
    let (scrollbar_track_bg, scrollbar_thumb_bg) = picker_scrollbar_colors(theme.mode);
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
            BorderColor(palette.border),
            BackgroundColor(palette.bar_bg_alt),
            StructurePickerPanel,
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(28.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    BorderColor(palette.border),
                    BackgroundColor(palette.bar_bg),
                    StructurePickerQueryBar,
                ))
                .with_children(|row| {
                    row.spawn((Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(0.0),
                        ..default()
                    },))
                        .with_children(|query_group| {
                            query_group.spawn((
                                Text::new("Search structures..."),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(palette.text_muted),
                                StructurePickerQueryText,
                            ));
                            query_group.spawn((
                                Text::new("|"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(palette.text_muted),
                                Visibility::Hidden,
                                StructurePickerQueryCaret,
                            ));
                        });
                    row.spawn((
                        Text::new("\u{f002}"),
                        TextFont {
                            font: icon_font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette.text_muted),
                        StructurePickerQueryIcon,
                    ));
                });
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(120.0),
                        max_height: Val::Px(240.0),
                        height: Val::Px(180.0),
                        align_self: AlignSelf::Stretch,
                        min_width: Val::Px(260.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    StructurePickerResultsLayout,
                ))
                .with_children(|results| {
                    results.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_grow: 1.0,
                            overflow: Overflow {
                                x: OverflowAxis::Clip,
                                y: OverflowAxis::Scroll,
                            },
                            padding: UiRect::axes(Val::Px(2.0), Val::Px(2.0)),
                            ..default()
                        },
                        ScrollPosition::default(),
                        StructurePickerResultsRoot,
                        StructurePickerResultsScroll,
                    ));

                    results
                        .spawn((
                            Node {
                                width: Val::Px(SCROLLBAR_WIDTH),
                                margin: UiRect::left(Val::Px(SCROLLBAR_TRACK_GAP)),
                                position_type: PositionType::Relative,
                                align_self: AlignSelf::Stretch,
                                ..default()
                            },
                            BackgroundColor(scrollbar_track_bg),
                            StructurePickerScrollbarTrack,
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    height: Val::Px(MIN_SCROLLBAR_THUMB_PX),
                                    ..default()
                                },
                                Visibility::Hidden,
                                BorderRadius::MAX,
                                BackgroundColor(scrollbar_thumb_bg),
                                StructurePickerScrollbarThumb,
                            ));
                        });
                });
        });
}

pub(crate) fn filtered_structure_entries(state: &StructurePickerState) -> Vec<String> {
    state
        .entries
        .iter()
        .filter(|entry| structure_matches_query(entry, &state.query))
        .cloned()
        .collect()
}

pub(crate) fn apply_structure_picker_query_text(
    picker: &mut StructurePickerState,
    caret_state: &mut StructurePickerCaretState,
    query: String,
) {
    picker.query = query;
    caret_state.visible = true;
    caret_state.timer.reset();
}

#[allow(clippy::type_complexity)]
pub(crate) fn structure_picker_toggle_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StructurePickerToggleButton>),
    >,
    mut picker: ResMut<StructurePickerState>,
    theme: Res<UiTheme>,
    mut caret_state: ResMut<StructurePickerCaretState>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                picker.visible = !picker.visible;
                if picker.visible {
                    apply_structure_picker_query_text(&mut picker, &mut caret_state, String::new());
                    set_structure_picker_keyboard_active(true);
                } else {
                    set_structure_picker_keyboard_active(false);
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
    mut caret_state: ResMut<StructurePickerCaretState>,
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
                set_structure_picker_keyboard_active(false);
            }
            Key::Backspace => {
                let _ = picker.query.pop();
                let query = picker.query.clone();
                apply_structure_picker_query_text(&mut picker, &mut caret_state, query);
            }
            Key::Enter => {
                if let Some(first) = filtered_structure_entries(&picker).first().cloned() {
                    super::load_structure_from_catalog_path(
                        &first,
                        &mut file_drag_drop,
                        catalog_channel.as_deref(),
                    );
                    picker.visible = false;
                    set_structure_picker_keyboard_active(false);
                }
            }
            Key::Character(_) => {
                if let Some(text) = &event.text {
                    // Keep search input simple and predictable for all layouts.
                    if text.chars().all(|ch| !ch.is_control()) {
                        let mut query = picker.query.clone();
                        query.push_str(text);
                        apply_structure_picker_query_text(&mut picker, &mut caret_state, query);
                    }
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn refresh_structure_picker_panel(
    mut commands: Commands,
    picker: Res<StructurePickerState>,
    mut panel_query: Query<&mut Node, With<StructurePickerPanel>>,
    mut panel_style_query: Query<
        (&mut BackgroundColor, &mut BorderColor),
        (With<StructurePickerPanel>, Without<StructurePickerQueryBar>),
    >,
    mut query_bar_style: Query<
        (&mut BackgroundColor, &mut BorderColor),
        (With<StructurePickerQueryBar>, Without<StructurePickerPanel>),
    >,
    mut query_text: Query<
        (&mut Text, &mut TextColor),
        (
            With<StructurePickerQueryText>,
            Without<StructurePickerQueryIcon>,
            Without<StructurePickerQueryCaret>,
        ),
    >,
    mut query_caret_color: Query<
        &mut TextColor,
        (
            With<StructurePickerQueryCaret>,
            Without<StructurePickerQueryText>,
            Without<StructurePickerQueryIcon>,
        ),
    >,
    mut query_icon_color: Query<
        &mut TextColor,
        (
            With<StructurePickerQueryIcon>,
            Without<StructurePickerQueryText>,
            Without<StructurePickerQueryCaret>,
        ),
    >,
    results_root_query: Query<
        Entity,
        (
            With<StructurePickerResultsRoot>,
            With<StructurePickerResultsScroll>,
        ),
    >,
    result_buttons: Query<Entity, With<StructurePickerResultButton>>,
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

    let palette = super::theme_palette(theme.mode);

    for (mut panel_bg, mut panel_border) in &mut panel_style_query {
        panel_bg.0 = palette.bar_bg_alt;
        panel_border.0 = palette.border;
    }
    for (mut query_bar_bg, mut query_bar_border) in &mut query_bar_style {
        query_bar_bg.0 = palette.bar_bg;
        query_bar_border.0 = palette.border;
    }
    if let Ok((mut text, mut text_color)) = query_text.single_mut() {
        if picker.query.is_empty() {
            text.0 = "Search structures...".to_string();
            text_color.0 = palette.text_muted;
        } else {
            text.0 = picker.query.clone();
            text_color.0 = palette.text;
        }
    }
    if let Ok(mut caret_color) = query_caret_color.single_mut() {
        caret_color.0 = if picker.query.is_empty() {
            palette.text_muted
        } else {
            palette.text
        };
    }
    if let Ok(mut icon_color) = query_icon_color.single_mut() {
        icon_color.0 = palette.text_muted;
    }

    if !picker.is_changed() && !theme.is_changed() {
        return;
    }

    for child in result_buttons.iter() {
        commands.entity(child).despawn();
    }

    let Ok(results_root) = results_root_query.single() else {
        return;
    };

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

pub(crate) fn blink_structure_picker_query_caret(
    time: Res<Time>,
    picker: Res<StructurePickerState>,
    mut caret_state: ResMut<StructurePickerCaretState>,
    mut caret_query: Query<&mut Visibility, With<StructurePickerQueryCaret>>,
) {
    let Ok(mut caret_visibility) = caret_query.single_mut() else {
        return;
    };

    if !picker.visible {
        *caret_visibility = Visibility::Hidden;
        return;
    }

    caret_state.timer.tick(time.delta());
    if caret_state.timer.just_finished() {
        caret_state.visible = !caret_state.visible;
    }
    *caret_visibility = if caret_state.visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

#[allow(clippy::type_complexity)]
pub(crate) fn update_structure_picker_scroll_indicator(
    layout_query: Query<&Children, With<StructurePickerResultsLayout>>,
    mut thumb_query: Query<
        (&mut Node, &mut Visibility, &mut BackgroundColor),
        (
            With<StructurePickerScrollbarThumb>,
            Without<StructurePickerScrollbarTrack>,
        ),
    >,
    scroll_query: Query<(&ScrollPosition, &ComputedNode), With<StructurePickerResultsRoot>>,
    mut track_query: Query<
        (&Children, &ComputedNode, &mut BackgroundColor),
        (
            With<StructurePickerScrollbarTrack>,
            Without<StructurePickerScrollbarThumb>,
        ),
    >,
    theme: Res<UiTheme>,
) {
    let (track_color, thumb_color) = picker_scrollbar_colors(theme.mode);
    for mut thumb_bg in thumb_query.iter_mut().map(|(_, _, bg)| bg) {
        thumb_bg.0 = thumb_color;
    }
    for (_, _, mut track_bg) in &mut track_query {
        track_bg.0 = track_color;
    }

    let Ok(layout_children) = layout_query.single() else {
        return;
    };

    let mut results_root: Option<Entity> = None;
    let mut track_root: Option<Entity> = None;
    for child in layout_children.iter() {
        if scroll_query.contains(child) {
            results_root = Some(child);
            continue;
        }
        if track_query.contains(child) {
            track_root = Some(child);
        }
    }

    let (Some(results_root), Some(track_root)) = (results_root, track_root) else {
        return;
    };

    let (scroll_position, scroll_computed) = match scroll_query.get(results_root) {
        Ok(value) => value,
        Err(_) => return,
    };
    let (track_children, track_computed, _) = match track_query.get(track_root) {
        Ok(value) => value,
        Err(_) => return,
    };

    let mut thumb_entity = None;
    for entity in track_children.iter() {
        if thumb_query.get(entity).is_ok() {
            thumb_entity = Some(entity);
            break;
        }
    }
    let Some(thumb_entity) = thumb_entity else {
        return;
    };

    let Ok((mut thumb_node, mut visibility, _)) = thumb_query.get_mut(thumb_entity) else {
        return;
    };

    let viewport_height = scroll_computed.size.y * scroll_computed.inverse_scale_factor;
    let content_height = scroll_computed.content_size.y * scroll_computed.inverse_scale_factor;
    let track_height = track_computed.size.y * track_computed.inverse_scale_factor;
    let max_scroll = (content_height - viewport_height).max(0.0);

    if max_scroll <= 0.0 || track_height <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    let viewport_ratio = (viewport_height / content_height).clamp(0.05, 1.0);
    let mut thumb_height =
        (track_height * viewport_ratio).clamp(MIN_SCROLLBAR_THUMB_PX, track_height);

    if thumb_height > track_height {
        thumb_height = track_height;
    }

    let scroll_offset = scroll_position.offset_y.clamp(0.0, max_scroll);
    let track_range = (track_height - thumb_height).max(0.0);
    let thumb_top = if track_range > 0.0 {
        (scroll_offset / max_scroll) * track_range
    } else {
        0.0
    };

    thumb_node.top = Val::Px(thumb_top);
    thumb_node.height = Val::Px(thumb_height);
    *visibility = Visibility::Inherited;
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn structure_picker_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    scroll_nodes: Query<Entity, With<StructurePickerResultsScroll>>,
    panel_nodes: Query<Entity, With<StructurePickerPanel>>,
    mut scroll_positions: Query<&mut ScrollPosition>,
    parents: Query<&ChildOf>,
    touch_gesture_state: Res<TouchGestureState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut picker_selection_state: ResMut<StructurePickerSelectionState>,
) {
    let _ = picker_selection_state.consume_suppression();

    let picker_hovered = panel_nodes
        .iter()
        .chain(scroll_nodes.iter())
        .any(|picker_root| {
            hover_map.iter().any(|(_, pointer_map)| {
                pointer_map
                    .iter()
                    .any(|(hovered, _)| is_descendant_or_self(*hovered, picker_root, &parents))
            })
        });

    for mouse_wheel in mouse_wheel_events.read() {
        let (mut dx, mut dy) = match mouse_wheel.unit {
            MouseScrollUnit::Line => (mouse_wheel.x * 12.0, mouse_wheel.y * 12.0),
            MouseScrollUnit::Pixel => (mouse_wheel.x, mouse_wheel.y),
        };
        if keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight)
        {
            std::mem::swap(&mut dx, &mut dy);
        }

        let mut did_scroll = false;
        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                let hovered = *entity;
                for scroll_root in scroll_nodes.iter() {
                    if is_descendant_or_self(hovered, scroll_root, &parents) {
                        if let Ok(mut scroll_position) = scroll_positions.get_mut(scroll_root) {
                            scroll_position.offset_x -= dx;
                            scroll_position.offset_y -= dy;
                            did_scroll = true;
                            break;
                        }
                    }
                }
                if did_scroll {
                    break;
                }
            }
            if did_scroll {
                break;
            }
        }
    }

    let rotate_drag = touch_gesture_state.rotate.length_squared() > 0.0004;
    if picker_hovered && rotate_drag {
        picker_selection_state.suppress_for_touch_drag();
        picker_selection_state.pending = None;
        for scroll_root in scroll_nodes.iter() {
            if let Ok(mut scroll_position) = scroll_positions.get_mut(scroll_root) {
                scroll_position.offset_x -= touch_gesture_state.rotate.x;
                scroll_position.offset_y -= touch_gesture_state.rotate.y;
                break;
            }
        }
    }
}

fn is_descendant_or_self(mut entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    loop {
        if entity == ancestor {
            return true;
        }
        match parents.get(entity) {
            Ok(parent) => entity = parent.parent(),
            Err(_) => return false,
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn structure_picker_result_buttons(
    mut interaction_query: Query<
        (
            Entity,
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
    mut picker_selection_state: ResMut<StructurePickerSelectionState>,
) {
    let mut selected_path = None;

    for (entity, interaction, selected, mut background) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Pressed));
                if !picker_selection_state.is_suppressed() {
                    picker_selection_state.pending = Some((entity, selected.path.clone()));
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::Hovered));

                if let Some((pending_entity, pending_path)) = picker_selection_state.pending.take()
                {
                    if pending_entity == entity {
                        if !picker_selection_state.is_suppressed() {
                            selected_path = Some(pending_path);
                        }
                    } else {
                        picker_selection_state.pending = Some((pending_entity, pending_path));
                    }
                }
            }
            Interaction::None => {
                *background = BackgroundColor(themed_button_bg(theme.mode, Interaction::None));

                if let Some((pending_entity, _)) = picker_selection_state.pending.as_ref() {
                    if *pending_entity == entity {
                        picker_selection_state.pending = None;
                    }
                }
            }
        }
    }

    if let Some(path) = selected_path {
        super::load_structure_from_catalog_path(
            &path,
            &mut file_drag_drop,
            catalog_channel.as_deref(),
        );
        picker.visible = false;
        set_structure_picker_keyboard_active(false);
    }
}

fn structure_matches_query(path: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    path.to_ascii_lowercase()
        .contains(&query.trim().to_ascii_lowercase())
}

pub(crate) fn parse_embedded_structure_entries() -> Vec<String> {
    super::EMBEDDED_STRUCTURE_LIST
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_matches_query_returns_true_for_empty_query() {
        assert!(structure_matches_query("proteins/6VXX.pdb", ""));
        assert!(structure_matches_query("proteins/6VXX.pdb", "   "));
    }

    #[test]
    fn structure_matches_query_is_case_insensitive() {
        assert!(structure_matches_query("proteins/6VXX.pdb", "6vxx"));
        assert!(structure_matches_query("proteins/water.xyz", "WATER"));
        assert!(structure_matches_query("proteins/abc", "PrO"));
        assert!(!structure_matches_query("proteins/abc", "xyz"));
    }

    #[test]
    fn filtered_structure_entries_filters_by_query() {
        let state = StructurePickerState {
            entries: vec![
                "proteins/6VXX.pdb".to_string(),
                "compounds/water.xyz".to_string(),
                "proteins/4v6f.pdb".to_string(),
            ],
            query: "V6F".to_string(),
            visible: true,
        };

        let filtered = filtered_structure_entries(&state);

        assert_eq!(filtered, vec!["proteins/4v6f.pdb".to_string()]);
    }

    #[test]
    fn setup_structure_picker_panel_spawns_one_query_text_and_icon() {
        let mut app = App::new();
        let icon_font = Handle::<Font>::default();

        setup_structure_picker_panel(
            &mut app.world_mut().commands(),
            &UiTheme::default(),
            &icon_font,
        );
        app.update();

        let query_text_count = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<StructurePickerQueryText>())
            .count();
        let query_icon_count = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<StructurePickerQueryIcon>())
            .count();
        let query_caret_count = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<StructurePickerQueryCaret>())
            .count();

        assert_eq!(query_text_count, 1);
        assert_eq!(query_icon_count, 1);
        assert_eq!(query_caret_count, 1);
    }

    #[test]
    fn refresh_structure_picker_panel_shows_placeholder_when_empty_query() {
        let mut app = App::new();
        app.insert_resource(StructurePickerState {
            entries: vec!["proteins/6VXX.pdb".to_string()],
            query: String::new(),
            visible: true,
        });
        app.insert_resource(UiTheme::default());
        let icon_font = Handle::<Font>::default();

        setup_structure_picker_panel(
            &mut app.world_mut().commands(),
            &UiTheme::default(),
            &icon_font,
        );
        app.add_systems(Update, refresh_structure_picker_panel);
        app.update();

        let palette = super::super::theme_palette(UiTheme::default().mode);
        let query_text = app.world().iter_entities().find_map(|entity| {
            if entity.contains::<StructurePickerQueryText>() {
                entity.get::<Text>().zip(entity.get::<TextColor>())
            } else {
                None
            }
        });
        let panel_node = app.world().iter_entities().find_map(|entity| {
            if entity.contains::<StructurePickerPanel>() {
                entity.get::<Node>()
            } else {
                None
            }
        });

        let (query_text, query_color) = query_text.expect("query text should exist");
        let panel_node = panel_node.expect("picker panel should exist");

        assert_eq!(query_text.0, "Search structures...");
        assert_eq!(query_color.0, palette.text_muted);
        assert_eq!(panel_node.display, Display::Flex);
    }

    #[test]
    fn refresh_structure_picker_panel_shows_query_and_text_color() {
        let mut app = App::new();
        app.insert_resource(StructurePickerState {
            entries: vec!["proteins/6VXX.pdb".to_string()],
            query: "6VXX".to_string(),
            visible: true,
        });
        app.insert_resource(UiTheme::default());
        let icon_font = Handle::<Font>::default();

        setup_structure_picker_panel(
            &mut app.world_mut().commands(),
            &UiTheme::default(),
            &icon_font,
        );
        app.add_systems(Update, refresh_structure_picker_panel);
        app.update();

        let palette = super::super::theme_palette(UiTheme::default().mode);
        let query_text = app.world().iter_entities().find_map(|entity| {
            if entity.contains::<StructurePickerQueryText>() {
                entity.get::<Text>().zip(entity.get::<TextColor>())
            } else {
                None
            }
        });
        let (query_text, query_color) = query_text.expect("query text should exist");

        assert_eq!(query_text.0, "6VXX");
        assert_eq!(query_color.0, palette.text);
    }
}
