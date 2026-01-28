// WebSocket client module for connecting to structure update server
// Supports both native (async-tungstenite) and WASM (web-sys) targets

use crate::structure::{Atom, UpdateStructure};
use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AtomData {
    element: String,
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct StructureMessage {
    atoms: Vec<AtomData>,
}

impl From<AtomData> for Atom {
    fn from(data: AtomData) -> Self {
        Atom {
            element: data.element,
            x: data.x,
            y: data.y,
            z: data.z,
        }
    }
}

// Resource to hold the channel receiver
#[derive(Resource)]
pub struct WebSocketStream {
    receiver: Receiver<UpdateStructure>,
}

// System to set up WebSocket connection
pub fn setup_websocket_stream(mut commands: Commands) {
    let (tx, rx) = unbounded();

    #[cfg(not(target_arch = "wasm32"))]
    {
        setup_native_websocket(tx);
    }

    #[cfg(target_arch = "wasm32")]
    {
        setup_wasm_websocket(tx);
    }

    commands.insert_resource(WebSocketStream { receiver: rx });
    info!("WebSocket stream initialized");
}

// System to poll WebSocket stream and send updates to Bevy
pub fn poll_websocket_stream(
    stream: Res<WebSocketStream>,
    mut events: EventWriter<UpdateStructure>,
) {
    while let Ok(update) = stream.receiver.try_recv() {
        info!(
            "Received structure update with {} atoms",
            update.atoms.len()
        );
        events.write(update);
    }
}

// Native WebSocket client using async-tungstenite run on async_std runtime
#[cfg(not(target_arch = "wasm32"))]
fn setup_native_websocket(tx: Sender<UpdateStructure>) {
    use bevy::tasks::IoTaskPool;

    let pool = IoTaskPool::get();

    pool.spawn(async move {
        let url = "ws://127.0.0.1:9001";
        println!("Connecting to WS: {url}");

        match async_tungstenite::async_std::connect_async(url).await {
            Ok((ws_stream, _)) => {
                use futures_util::StreamExt;

                println!("Connected!");
                let (_, mut read) = ws_stream.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(async_tungstenite::tungstenite::Message::Text(text)) => {
                            if let Ok(structure_msg) =
                                serde_json::from_str::<StructureMessage>(&text)
                            {
                                let atoms = structure_msg
                                    .atoms
                                    .into_iter()
                                    .map(std::convert::Into::into)
                                    .collect();

                                if tx.send(UpdateStructure { atoms }).is_err() {
                                    println!("Bevy channel closed");
                                    break;
                                }
                            }
                        }
                        Ok(async_tungstenite::tungstenite::Message::Close(_)) => {
                            println!("Server closed WebSocket");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WS error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => eprintln!("Failed to connect WS: {}", e),
        }
    })
    .detach();
}

// WASM WebSocket client using web-sys
#[cfg(target_arch = "wasm32")]
fn setup_wasm_websocket(tx: Sender<UpdateStructure>) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{ErrorEvent, MessageEvent, WebSocket};

    let ws = WebSocket::new("ws://127.0.0.1:9001").unwrap();

    // onmessage callback
    let tx_clone = tx.clone();
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            let text: String = txt.into();
            if let Ok(structure_msg) = serde_json::from_str::<StructureMessage>(&text) {
                let atoms: Vec<Atom> = structure_msg.atoms.into_iter().map(|a| a.into()).collect();

                let _ = tx_clone.send(UpdateStructure { atoms });
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // onerror callback
    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        web_sys::console::error_1(&format!("WebSocket error: {:?}", e).into());
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // onopen callback
    let onopen_callback = Closure::wrap(Box::new(move |_| {
        web_sys::console::log_1(&"WebSocket connected".into());
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    // Keep the WebSocket alive by leaking it
    // In production, you'd want proper cleanup
    Box::leak(Box::new(ws));
}
