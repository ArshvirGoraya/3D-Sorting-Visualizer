// INFO: This module should only be included under #cfg[target_arch = "wasm32"] since its only meant for
// the web/browser.

use bevy::{
    asset::Assets,
    audio::AudioSource,
    ecs::{
        message::MessageReader,
        resource::Resource,
        system::{Commands, ResMut},
    },
    input::mouse::MouseMotion,
    state::state::NextState,
};
use web_sys::{
    HtmlInputElement, console,
    js_sys::Uint8Array,
    wasm_bindgen::{JsCast, prelude::Closure},
};

use futures::channel::mpsc::{Receiver, channel};

use crate::AudioControls;

pub struct LoadedFile {
    file_name: String,
    bytes: Vec<u8>,
}

pub enum FileEvent {
    FileLoaded(LoadedFile),
    FileNotSelected,
    Error(String),
}

#[derive(Resource)]
pub struct BrowserAudioFileChannel {
    receiver: Receiver<FileEvent>,
}

pub fn spawn_browser_audio_handlers(mut commands: Commands) {
    // INFO: system runs at startup on wasm builds

    let input_element: HtmlInputElement = web_sys::window()
        .expect("window should exist")
        .document()
        .expect("document should exist")
        .get_element_by_id("audio_picker")
        .expect("audio_picker input should exist in index.html")
        .dyn_into::<HtmlInputElement>() // must convert with dyn_into instead of just .into()
        .expect("audio_picker id must be on a input element");

    let (sender, receiver) = channel(1);

    let input_element_clone = input_element.clone();

    // this outer function cannot be async. Could not be attached to set_onchange() if it were.
    let input_onchange_closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::Event| {
        console::log_1(&"running closure".into());

        // have to clone it here as well or will become a FnOnce. arg can be event: web_sys::Event
        let mut sender_clone = sender.clone();

        let file = input_element_clone
            .files()
            .expect("filelist should exist")
            .get(0);

        if let Some(file) = file {
            let array_buffer_promise = file.array_buffer();
            let file_name = file.name();

            // adds block to browser event loop (will switch to this and back until done)
            wasm_bindgen_futures::spawn_local(async move {
                console::log_1(&"local thread created: awaiting file list to update".into());

                // awaits: checks to see if done yet (wont continue from this line until it is)
                let array_buffer_in_js_value_result =
                    wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await;
                console::log_1(&"local thread stopped: file list updated".into());

                match array_buffer_in_js_value_result {
                    Ok(array_buffer_in_js_value) => {
                        if let Err(error) =
                            sender_clone.try_send(FileEvent::FileLoaded(LoadedFile {
                                file_name,
                                bytes: Uint8Array::new(&array_buffer_in_js_value).to_vec(),
                            }))
                        {
                            // Failed to send file buffer...
                            log::warn!(
                                "sender failed to send file {} with error {}",
                                file.name(),
                                error
                            );
                        }
                    }
                    Err(err) => {
                        if let Err(error) =
                            sender_clone.try_send(FileEvent::Error(format!("{err:?}")))
                        {
                            // Failed to send error message...
                            log::warn!("sender failed to send error {error:?}");
                        }
                    }
                }
            });
        } else {
            if let Err(error) = sender_clone.try_send(FileEvent::FileNotSelected) {
                // Failed to get file from picker...
                log::warn!(
                    "sender failed to send no file not selected event with error {}",
                    error
                );
            }
        }
    });

    // On file list change (when file is selected), run the above closure
    // INFO: the closure runs AFTER the file is selected, not when the input button is clicked!
    // The thread that starts is the sender sending file data to the receiver and then the thread
    // ends.
    input_element.set_onchange(Some(input_onchange_closure.as_ref().unchecked_ref()));

    input_onchange_closure.forget(); // do not delete the closure after this function scope ends!
    // (keep alive for app's lifetime.)

    commands.insert_resource(BrowserAudioFileChannel { receiver });
}

pub fn audio_select_listener(
    mut audio_controls: ResMut<AudioControls>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    mut audio_file_channel: ResMut<BrowserAudioFileChannel>,
    mut audio_receiver_listening_set: ResMut<NextState<crate::WasmAudioReceiverListening>>,
    mouse_event: MessageReader<MouseMotion>,
) {
    // INFO: only runs if in_state(WasmAudioReceiverListening::Listening)

    // INFO: this receiver is bounded to channel that can only hold 1 data at a time.
    // Will only ever receive a single piece of that data (in this case the entire file's byte), never more.
    if let Ok(file_event_option) = audio_file_channel.receiver.try_next() {
        log::info!("Listener Received something!");
        if let Some(file_event) = file_event_option {
            match file_event {
                FileEvent::FileLoaded(loaded_file) => {
                    let bytes_length = loaded_file.bytes.len();
                    log::info!("selected file length: {}", bytes_length);
                    crate::change_audio_source(
                        &mut audio_controls,
                        &mut audio_assets,
                        loaded_file.file_name,
                        loaded_file.bytes,
                    );
                }
                FileEvent::FileNotSelected => {
                    log::info!("no file selected");
                }
                FileEvent::Error(err_string) => {
                    log::warn!("file selection error: {err_string}");
                }
            }
            // stop listening once file_event is received.
            audio_receiver_listening_set.set(crate::WasmAudioReceiverListening::NotListening);
        }
    }

    if !mouse_event.is_empty() {
        // INFO: stop listening when a mouse event is detected
        // if "cancel" is clicked, no file is chosen, but listener doesn't stop.
        // so need a way to stop it.
        // Arbitrarily, listening for mouse events is chosen to stop it.
        // INFO: stopping the listener is not actually necessary. Can keep listening forever, just
        // stopping to reduce computation.
        audio_receiver_listening_set.set(crate::WasmAudioReceiverListening::NotListening);
        // log::info!("Listener Stopped!");
    }
}
