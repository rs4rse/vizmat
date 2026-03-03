use std::sync::OnceLock;

use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use crossbeam_channel::Sender;

#[cfg(target_arch = "wasm32")]
use {
    crossbeam_channel::{Receiver, TryRecvError},
    gloo::events::{EventListener, EventListenerOptions},
    gloo::utils::format::JsValueSerdeExt,
    web_sys::wasm_bindgen::{JsCast, JsValue},
    web_sys::{CustomEvent, CustomEventInit, DragEvent, File, FileReader},
};

pub(crate) mod io;
pub(crate) mod ui;

pub(crate) mod client;
pub(crate) mod constants;
pub(crate) mod formats;
pub(crate) mod structure;

use crate::client::{poll_websocket_stream, setup_websocket_stream};
use crate::formats::{
    is_supported_extension, parse_structure_by_extension, SUPPORTED_EXTENSIONS_HELP,
};
use crate::io::{handle_file_drag_drop, load_dropped_file, update_crystal_from_file, FileDragDrop};
use crate::structure::{
    update_crystal_system, AtomColorMode, BondInferenceSettings, Crystal, UpdateStructure,
};
use crate::ui::{
    apply_bond_tolerance_debounce, apply_theme_to_atom_hover_panel, apply_theme_to_hud,
    apply_theme_to_startup_screen, auto_reset_view_on_crystal_change, bond_tolerance_controls,
    camera_controls, cleanup_startup_screen, color_mode_button, handle_catalog_load_results,
    handle_load_default_button, handle_open_file_button, hide_non_startup_controls,
    refresh_structure_picker_panel, reset_camera_button_interaction, setup_cameras, setup_file_ui,
    setup_light, setup_startup_screen, show_non_startup_controls, structure_picker_keyboard_search,
    structure_picker_result_buttons, structure_picker_toggle_button, sync_atom_selection_highlight,
    sync_color_mode_label, sync_gizmo_axis_rotation, toggle_light_attachment, toggle_theme_button,
    transition_to_running_on_structure_loaded, update_atom_hover_cache, update_atom_hover_label,
    update_bond_order_legend, update_color_mode_availability, update_file_ui,
    update_gizmo_viewport, update_scene, update_selected_atom_from_click,
    update_structure_loading_overlay, AppUiState, CatalogLoadChannel,
};
use crate::ui::{setup_buttons, spawn_axis};

/// channel sender to share with multiple producers and offering a simple `send` function
#[derive(Resource, Clone, Debug)]
pub struct ChannelSender<T: Event>(Sender<T>);

impl<T: Event> ChannelSender<T> {
    /// send `event` to our central receiver that forwards them as triggers that can be observed
    pub fn send(&self, event: impl Into<T>) {
        let event = event.into();
        if let Err(err) = self.0.send(event) {
            error!("sending failed due to receiver being dropped: {err:?}");
        }
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Entry point for WASM
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start() {
    run_app();
}

#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
#[derive(Event, Clone, Debug)]
pub enum WebEvent {
    Drop {
        name: String,
        data: Vec<u8>,
        mime_type: String,
    },
    CatalogLoadError {
        path: String,
        message: String,
    },
}

pub struct WebPlugin {
    #[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
    pub dom_drop_element_id: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct EventReceiver<T: Event>(Receiver<T>);

#[cfg(target_arch = "wasm32")]
fn process_events<T: Event>(receiver: Option<Res<EventReceiver<T>>>, mut commands: Commands) {
    if let Some(receiver) = receiver.as_ref() {
        loop {
            match receiver.0.try_recv() {
                Ok(msg) => {
                    commands.trigger(msg);
                }
                Err(TryRecvError::Disconnected) => {
                    error!("sender dropped, removing receiver");
                    commands.remove_resource::<EventReceiver<T>>();
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
            }
        }
    }
}

impl Plugin for WebPlugin {
    #[cfg_attr(not(target_family = "wasm"), allow(unused_variables))]
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            let (sender, receiver) = crossbeam_channel::unbounded();
            app.insert_resource(EventReceiver::<WebEvent>(receiver));
            app.add_systems(PreUpdate, process_events::<WebEvent>);
            let sender = ChannelSender::<WebEvent>(sender);
            set_sender(sender);
            register_drop(&self.dom_drop_element_id).unwrap();
        }
    }
}

static SENDER: OnceLock<Option<ChannelSender<WebEvent>>> = OnceLock::new();

pub fn send_event(e: WebEvent) {
    let Some(sender) = SENDER.get().and_then(Option::as_ref) else {
        return bevy::log::error!("`WebPlugin` not installed correctly (no sender found)");
    };
    sender.send(e);
}

pub fn set_sender(sender: ChannelSender<WebEvent>) {
    while SENDER.set(Some(sender.clone())).is_err() {}
}

#[cfg(target_arch = "wasm32")]
fn window() -> Window {
    Window {
        canvas: Some("#bevy-canvas".into()),
        fit_canvas_to_parent: true,
        resize_constraints: WindowResizeConstraints {
            min_width: 1.0,
            min_height: 1.0,
            ..default()
        },
        prevent_default_event_handling: false,
        ..default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn window() -> Window {
    use bevy::window::WindowResolution;

    Window {
        title: "demo".into(),
        resolution: WindowResolution::new(500.0, 500.0),
        ..default()
    }
}

#[cfg(target_arch = "wasm32")]
pub fn get_web_theme() -> Option<String> {
    let doc = gloo::utils::document();
    let theme = doc.document_element()?.get_attribute("data-theme");
    Some(theme.unwrap_or_else(|| "dark".to_string()))
}

#[cfg(target_arch = "wasm32")]
pub fn set_web_theme(theme: &str) -> Option<()> {
    let doc = gloo::utils::document();
    let normalized = if theme == "light" { "light" } else { "dark" };
    doc.document_element()?
        .set_attribute("data-theme", normalized)
        .ok()?;

    let event_init = CustomEventInit::new();
    event_init.set_detail(&JsValue::from_serde(&serde_json::json!({ "theme": normalized })).ok()?);
    let event = CustomEvent::new_with_event_init_dict("vizmat-theme-change", &event_init).ok()?;
    web_sys::window()?.dispatch_event(&event).ok()?;

    Some(())
}

#[cfg(target_arch = "wasm32")]
pub fn register_drop(id: &str) -> Option<()> {
    let doc = gloo::utils::document();
    let element = doc.get_element_by_id(id)?;

    EventListener::new_with_options(
        &element,
        "dragover",
        EventListenerOptions::enable_prevent_default(),
        move |event| {
            let event: DragEvent = event.clone().dyn_into().expect("dynamic cast fail");
            event.stop_propagation();
            event.prevent_default();

            event
                .data_transfer()
                .expect("invalid data transfer")
                .set_drop_effect("copy");
            event
                .data_transfer()
                .expect("invalid data transfer")
                .set_effect_allowed("all");

            info!("dragover event");
        },
    )
    .forget();

    EventListener::new_with_options(
        &element,
        "drop",
        EventListenerOptions::enable_prevent_default(),
        move |event| {
            let event: DragEvent = event.clone().dyn_into().expect("dynamic cast fail");
            event.stop_propagation();
            event.prevent_default();

            info!("drop event");

            let transfer = event.data_transfer().expect("invalid data transfer");
            let file_list = transfer.files().expect("invalid file list");
            let item_list = transfer.items();

            let mut files: Vec<File> = Vec::new();
            for idx in 0..file_list.length() {
                if let Some(file) = file_list.get(idx) {
                    files.push(file);
                }
            }
            if files.is_empty() {
                for idx in 0..item_list.length() {
                    let item = item_list.get(idx).expect("invalid item");
                    if let Ok(Some(file)) = item.get_as_file() {
                        files.push(file);
                    }
                }
            }

            for (idx, file_info) in files.into_iter().enumerate() {
                let idx = idx as u32;

                info!(
                    "file[{idx}] = '{}' - {} - {} b",
                    file_info.name(),
                    file_info.type_(),
                    file_info.size()
                );

                let file_reader = FileReader::new().unwrap();

                {
                    let file_reader = file_reader.clone();
                    let file_info = file_info.clone();
                    EventListener::new(&file_reader.clone(), "load", move |_event| {
                        let result = file_reader.result().expect("result invalid");
                        let result = web_sys::js_sys::Uint8Array::new(&result);
                        let mut data: Vec<u8> = vec![0; result.length() as usize];
                        result.copy_to(&mut data);

                        info!("drop file read: {}", file_info.name());

                        send_event(WebEvent::Drop {
                            name: file_info.name(),
                            data,
                            mime_type: file_info.type_(),
                        });
                    })
                    .forget();
                }

                file_reader.read_as_array_buffer(&file_info).unwrap();
            }
        },
    )
    .forget();

    Some(())
}

/// Shared function for Bevy app setup
pub fn run_app() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: Level::INFO,
                    filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
                    custom_layer: |_| None,
                })
                .set(WindowPlugin {
                    primary_window: Some(window()),
                    ..default()
                }),
        )
        .add_plugins(WebPlugin {
            dom_drop_element_id: String::from("bevy-canvas"),
        })
        .init_resource::<FileDragDrop>()
        .init_resource::<CatalogLoadChannel>()
        .init_resource::<AtomColorMode>()
        .init_resource::<BondInferenceSettings>()
        .init_state::<AppUiState>()
        .add_event::<UpdateStructure>()
        .add_event::<bevy::window::FileDragAndDrop>()
        .add_systems(
            Startup,
            (
                setup_cameras,
                setup_buttons,
                setup_file_ui,
                setup_startup_screen.after(setup_file_ui),
                hide_non_startup_controls.after(setup_file_ui),
                setup_websocket_stream,
            ),
        )
        .add_systems(OnExit(AppUiState::Startup), cleanup_startup_screen)
        .add_systems(OnEnter(AppUiState::Running), show_non_startup_controls)
        .add_systems(Startup, spawn_axis.after(setup_cameras))
        .add_systems(Startup, (setup_light).after(setup_cameras))
        .add_systems(
            Update,
            (
                poll_websocket_stream,
                update_crystal_system,
                handle_file_drag_drop,
                load_dropped_file,
                update_crystal_from_file,
            ),
        )
        .add_systems(Update, update_file_ui)
        .add_systems(Update, update_structure_loading_overlay)
        .add_systems(Update, handle_catalog_load_results)
        .add_systems(
            Update,
            transition_to_running_on_structure_loaded.run_if(in_state(AppUiState::Startup)),
        )
        .add_systems(
            Update,
            reset_camera_button_interaction.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            handle_load_default_button.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            handle_open_file_button.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(Update, structure_picker_toggle_button)
        .add_systems(Update, structure_picker_keyboard_search)
        .add_systems(
            Update,
            refresh_structure_picker_panel.after(structure_picker_keyboard_search),
        )
        .add_systems(Update, structure_picker_result_buttons)
        .add_systems(
            Update,
            update_selected_atom_from_click.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            update_color_mode_availability.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            update_atom_hover_cache
                .after(update_color_mode_availability)
                .run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            color_mode_button.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            sync_color_mode_label
                .after(color_mode_button)
                .run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            bond_tolerance_controls.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            apply_bond_tolerance_debounce
                .after(bond_tolerance_controls)
                .run_if(in_state(AppUiState::Running)),
        )
        .add_systems(Update, toggle_theme_button)
        .add_systems(Update, apply_theme_to_hud)
        .add_systems(Update, apply_theme_to_atom_hover_panel)
        .add_systems(Update, apply_theme_to_startup_screen)
        .add_systems(
            Update,
            auto_reset_view_on_crystal_change
                .after(update_crystal_from_file)
                .run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            toggle_light_attachment.run_if(in_state(AppUiState::Running)),
        )
        .add_systems(
            Update,
            (
                camera_controls,
                sync_gizmo_axis_rotation,
                update_gizmo_viewport,
                update_scene,
                sync_atom_selection_highlight.after(update_scene),
                update_bond_order_legend.after(update_scene),
                update_atom_hover_label.after(update_scene),
            )
                .run_if(in_state(AppUiState::Running)),
        )
        .add_observer(web_event_observer)
        .run();
}

fn web_event_observer(
    trigger: Trigger<WebEvent>,
    mut file_drag_drop: ResMut<FileDragDrop>,
    mut next_ui_state: ResMut<NextState<AppUiState>>,
    mut commands: Commands,
) {
    match trigger.event() {
        WebEvent::Drop {
            name,
            data,
            mime_type,
        } => {
            let ext = name.rsplit('.').next().unwrap_or_default();
            if !is_supported_extension(ext) {
                set_file_error_status(
                    &mut file_drag_drop,
                    format!(
                        "Unsupported file. Please drop {}",
                        SUPPORTED_EXTENSIONS_HELP
                    ),
                );
                return;
            }
            let contents = String::from_utf8_lossy(data);
            match parse_structure_by_extension(ext, &contents) {
                Ok(crystal) => {
                    let crystal_resource = crystal.clone();
                    set_file_loaded_status(&mut file_drag_drop, name, crystal);
                    commands.insert_resource(crystal_resource);
                    next_ui_state.set(AppUiState::Running);
                }
                Err(e) => {
                    eprintln!("Failed to parse structure file: {}", e);
                    set_file_error_status(&mut file_drag_drop, format!("Parse error: {e}"));
                    warn!("Web drop parse error: {e}");
                }
            }

            info!("loaded file: '{name}'");
            info!("loaded file mime type: '{mime_type}'");
        }
        WebEvent::CatalogLoadError { path, message } => {
            let name = path.rsplit('/').next().unwrap_or(path);
            file_drag_drop.status_message = format!("Load error ({name}): {message}");
            file_drag_drop.status_kind = crate::io::FileStatusKind::Error;
            eprintln!("Failed to load catalog structure '{path}': {message}");
        }
    }
}

fn set_file_loaded_status(file_drag_drop: &mut FileDragDrop, name: &str, crystal: Crystal) {
    let atom_count = crystal.atoms.len();
    let file_bond_count = crystal.bonds.as_ref().map_or(0, Vec::len);
    file_drag_drop.dragged_file = None;
    file_drag_drop.loaded_crystal = Some(crystal);
    file_drag_drop.status_message = if file_bond_count > 0 {
        format!("Loaded: {name} ({atom_count} atoms, {file_bond_count} file bonds)")
    } else {
        format!("Loaded: {name} ({atom_count} atoms)")
    };
    file_drag_drop.status_kind = crate::io::FileStatusKind::Success;
}

fn set_file_error_status(file_drag_drop: &mut FileDragDrop, message: String) {
    file_drag_drop.status_message = message;
    file_drag_drop.status_kind = crate::io::FileStatusKind::Error;
}
