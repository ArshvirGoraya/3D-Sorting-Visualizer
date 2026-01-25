// This module should only be included under #cfg[target_arch = "wasm32"] since its only meant for
// the web/browser.

use bevy::{
    asset::Assets,
    audio::AudioSource,
    ecs::{
        resource::Resource,
        system::{Commands, ResMut},
    },
    state::state::NextState,
};
use web_sys::{
    Document, FileReader, HtmlInputElement, Window,
    js_sys::{self, ArrayBuffer, Uint8Array},
    wasm_bindgen::{JsCast, JsValue, prelude::Closure},
};

use futures::channel::mpsc::{Receiver, Sender, channel};

use crate::AudioControls;

pub struct LoadedFile {
    file_name: String,
    bytes: Vec<u8>,
}

pub enum FileEvent {
    // Must send Vec<u8>, not JsValue or Uint8Array (bevy cannot use those like those as resources).
    FileLoaded(LoadedFile),
    FileNotSelected,
    Error(String),
}
// same as above. only in wasm.
#[derive(Resource)]
pub struct BrowserAudioFileChannel {
    receiver: Receiver<FileEvent>,
    // file_reader: FileReader,
    // on_load_closure: Closure<dyn fnmut()>,
}

// pub async fn browser_audio_picker(browser_audio_elements: BrowserAudioElements) {
//     browser_audio_elements.input_element.click();
//     // let input_element: HtmlInputElement = web_sys::window()
//     //     .expect("window should exist")
//     //     .document()
//     //     .expect("document should exist")
//     //     .get_element_by_id("audio_picker")
//     //     .expect("audio_picker input should exist in index.html")
//     //     .dyn_into::<HtmlInputElement>() // must convert with dyn_into instead of just .into()
//     //     .expect("audio_picker id must be on a input element");
//     //
//     // // starts the picker
//     // input_element.click();
//
//     // Now I just need to get the audio from it...
//
//     //
//
//     // let onchange_closure = Closure::
//
//     // input_element.set_onload();
//
//     log::info!("browser audio picker!")
// }

// #[wasm_bindgen]
// pub fn browser_file_bytes_response(bytes: Uint8Array) {
//     //
// }

pub fn spawn_browser_audio_handlers(mut commands: Commands) {
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
    // let sender_clone = sender.clone(); // if sender is removed from being added to
    // BrowserAudioFileChannel, then this clone is not needed!

    // this outer function cannot be async. Cannot be attached to set_onchange() if it were.
    let input_onchange_closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::Event| {
        let mut sender_clone = sender.clone(); // have to clone it here as well or will
        // become a FnOnce
        // arg can be event: web_sys::Event
        let file = input_element_clone
            .files()
            .expect("filelist should exist")
            .get(0);

        if let Some(file) = file {
            let array_buffer_promise = file.array_buffer();
            let file_name = file.name();

            // adds block to browser event loop (will swtich to this and back until done)
            wasm_bindgen_futures::spawn_local(async move {
                // Awaits the promise
                let array_buffer_in_js_value_result =
                    wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await;

                match array_buffer_in_js_value_result {
                    Ok(array_buffer_in_js_value) => {
                        // array_buffer_in_js_value.

                        if let Err(error) =
                            sender_clone.try_send(FileEvent::FileLoaded(LoadedFile {
                                file_name,
                                bytes: Uint8Array::new(&array_buffer_in_js_value).to_vec(),
                            }))
                        {
                            log::warn!("sender failed to send file {}", file.name());
                            // try to stop listening?
                        }
                    }
                    Err(err) => {
                        if let Err(error) =
                            sender_clone.try_send(FileEvent::Error(format!("{err:?}")))
                        {
                            log::warn!("sender failed to send error {error:?}");
                            // try to stop listening?
                        }
                    }
                }
            });
        } else {
            if let Err(error) = sender_clone.try_send(FileEvent::FileNotSelected) {
                // turns of listening to receiver system (as do the above sender events)
                log::warn!("sender failed to send no file selected event");
                // try to stop listening?
            }
        }
    });

    input_element.set_onchange(Some(input_onchange_closure.as_ref().unchecked_ref()));

    input_onchange_closure.forget(); // do not delete the closure after this function scope ends!
    // (keep alive for app's lifetime.)

    // commands.insert_resource(BrowserInputElement { input_element });
    commands.insert_resource(BrowserAudioFileChannel { receiver });
}

pub fn audio_select_listener(
    mut audio_controls: ResMut<AudioControls>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    mut audio_file_channel: ResMut<BrowserAudioFileChannel>,
    mut audio_receiver_listening_set: ResMut<NextState<crate::WasmAudioReceiverListening>>,
) {
    // only runs if in_state(WasmAudioReceiverListening::Listening)
    log::info!("Listener Ran!");
    if let Ok(file_event_option) = audio_file_channel.receiver.try_next() {
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
                    //
                }
                FileEvent::Error(err_string) => {
                    //
                }
            }
            // stop listening once file_event is received.
            audio_receiver_listening_set.set(crate::WasmAudioReceiverListening::NotListening);
        }
    }
}
