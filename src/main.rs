pub mod sorter;
mod ui;

#[cfg(any(target_arch = "wasm32", rust_analyzer))]
mod wasm_audio_picker;

use bevy::{
    asset::AssetMetaCheck,
    audio::{AudioPlugin, PlaybackMode, Volume},
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::MouseWheel,
    platform::collections::HashMap,
    prelude::*,
};

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use futures::channel::mpsc::{Receiver, Sender};
use rfd::FileHandle;
use web_sys::{js_sys::Uint8Array, wasm_bindgen::JsValue};

use core::{fmt, time::Duration};
use std::{
    fs,
    path::{self, PathBuf},
};

// Not added into the system even on non-wasm builds, in which case this enum definition just
// exists but an instance of it is never created.
// {
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum WasmAudioReceiverListening {
    #[default]
    NotListening,
    Listening,
}
// // same as above. only in wasm
// pub enum FileEvent {
//     FileLoaded(Vec<u8>),
//     FileNoSelected,
//     Error(String),
// }
// // same as above. only in wasm.
// #[derive(Resource)]
// pub struct BrowserAudioElements {
//     sender: Sender<FileEvent>,
//     receiver: Receiver<FileEvent>,
//     // file_reader: FileReader,
//     // on_load_closure: Closure<dyn fnmut()>,
// }
// }

pub const PROGRAM_TITLE: &str = "3D Sorting";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum CameraControls {
    #[default]
    DragControl,
    AutoRotate,
    FollowSelected,
}

impl fmt::Display for CameraControls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraControls::DragControl => write!(f, "Drag Control"),
            CameraControls::AutoRotate => write!(f, "Auto Rotate"),
            CameraControls::FollowSelected => write!(f, "Follow Selected"),
        }
    }
}
impl CameraControls {
    pub const ALL: [CameraControls; 3] = [
        CameraControls::DragControl,
        CameraControls::AutoRotate,
        CameraControls::FollowSelected,
    ];
}
#[derive(Resource, Default)]
pub struct AudioControls {
    enabled: bool,
    volume: f32,
    pitch: f32,
    default_file_name: String,
    audio_source_handle_default: Handle<AudioSource>,
    selected_file_name: Option<String>,
    audio_source_handle: Option<Handle<AudioSource>>,
    // bevy egui:
    selected_path_buf: Option<PathBuf>,
    // open_file_dialog: Option<egui_file::FileDialog>,
    // audio_entity: Option<Entity>,
    // filter_closure: Box,
}

// #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
// pub enum AudioPicking {
//     #[default]
//     NotPicking,
//     Picking,
// }

//
// impl Default for AudioControls {
//     fn default() -> Self {
//         Self {
//             enabled: true,
//             volume: 10,
//             pitch: 1,
//             selected_file: "Default".to_string(),
//             default_file_path: "Default".to_string(),
//             ..Default::default()
//         }
//     }
// }

// Marker Components:
#[derive(Component)]
pub struct DefaultAudio;
#[derive(Component)]
pub struct SelectedAudio;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: PROGRAM_TITLE.to_string(),
                    window_theme: Some(bevy::window::WindowTheme::Dark),
                    recognize_doubletap_gesture: true,
                    recognize_pinch_gesture: true,
                    recognize_rotation_gesture: true,
                    recognize_pan_gesture: Some((1, 1)), // for iOS
                    // present_mode: bevy::window::PresentMode::Fifo..Default::default(),
                    mode: bevy::window::WindowMode::Windowed,
                    fit_canvas_to_parent: true, // wasm "fullscreen"
                    prevent_default_event_handling: true,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(AssetPlugin {
                // for assets to work on browser (not sure when)
                meta_check: AssetMetaCheck::Never,
                ..Default::default()
            }),
    )
    // https://github.com/Plonq/bevy_panorbit_camera
    .add_plugins(PanOrbitCameraPlugin)
    .add_plugins(EguiPlugin::default())
    // .add_plugins(GlobalsPlugin)
    // .init_asset::<AudioSource>()
    .init_resource::<ui::NumberRegex>()
    .init_resource::<ui::Random>()
    .init_resource::<ui::ParsedValues>()
    .init_resource::<ui::FontScale>()
    .init_resource::<ui::UserText>()
    .init_resource::<GlobalVolume>()
    // .init_resource::<AudioControls>()
    // .insert_resource(AudioControls {
    //     volume: 10,
    //     pitch: 1,
    //     enabled: true,
    //     default_file_path: "".to_string(),
    //     selected_file: "".to_string(),
    //     ..Default::default()
    // })
    .insert_resource(ui::CopyTimer {
        copy_timer: Timer::from_seconds(1.0, TimerMode::Once),
    })
    .insert_resource(sorter::IncrementTimer {
        duration: Duration::new(0, 0),
        increment_timer: Timer::from_seconds(0.0, TimerMode::Once),
        duration_f64: 0.0,
    })
    // set tonemapping to none for accurate color
    //
    .init_state::<sorter::SortState>();

    #[cfg(any(target_arch = "wasm32", rust_analyzer))]
    app.init_state::<WasmAudioReceiverListening>();

    // .init_state::<AudioPicking>()
    // .add_systems(Startup, tests)
    // .add_systems(Update, center_camera.run_if())
    app.add_systems(
        Startup,
        (
            finish_timers,
            spawn_audio_sources,
            spawn_3d_camera,
            spawn_cube_assets,
            ui::spawn_random_parsed_values,
        )
            .chain(),
    );
    #[cfg(any(target_arch = "wasm32", rust_analyzer))]
    app.add_systems(Startup, wasm_audio_picker::spawn_browser_audio_handlers);

    #[cfg(any(target_arch = "wasm32", rust_analyzer))]
    app.add_systems(
        Update,
        wasm_audio_picker::audio_select_listener
            .run_if(in_state(WasmAudioReceiverListening::Listening)),
    );

    // .add_systems(Update, audio_select.run_if(in_state(AudioPicking::Picking)))
    app.add_systems(Update, font_scale_inputs)
        .add_systems(EguiPrimaryContextPass, ui::ui_system);

    app.run();
}

// fn audio_select(mut audio_controls: ResMut<AudioControls>) {
//     if let Some(file_dialog) = &mut audio_controls.open_file_dialog {
//         //
//     }
// }

fn change_audio_source(
    audio_controls: &mut ResMut<AudioControls>,
    audio_assets: &mut ResMut<Assets<AudioSource>>,
    file_name: String,
    bytes: Vec<u8>,
) {
    audio_controls.selected_file_name = Some(file_name);
    if let Some(handle) = &mut audio_controls.audio_source_handle {
        // remove previous handle if it exists (bevy
        // does this automatically with reference
        // counting? So, not necessary?)
        audio_assets.remove(handle);
    }
    let handle = audio_assets.add(AudioSource {
        bytes: bytes.into(),
    });
    audio_controls.audio_source_handle = Some(handle);
}

fn spawn_audio_sources(
    mut commands: Commands,
    // mut audio_assets: ResMut<Assets<AudioSource>>,
    asset_server: Res<AssetServer>,
    // music_controller: Query<&AudioSink, With<DefaultAudio>>,
    mut global_volume: ResMut<GlobalVolume>,
) {
    let default_volume = 1.0;
    global_volume.volume = Volume::Linear(default_volume);

    // // load file and add to audio_assets
    let default_handle: Handle<bevy::audio::AudioSource> =
        asset_server.load("audio/impactWood_medium_000.ogg");
    //
    let file_name = "impactWood_medium_000.ogg".to_string();
    let default_pitch = 1.0;
    //
    commands.insert_resource(AudioControls {
        volume: default_volume,
        pitch: default_pitch,
        enabled: true,
        default_file_name: file_name,
        audio_source_handle_default: default_handle,
        // if these are none: the default audio plays
        selected_file_name: None,
        audio_source_handle: None,
        ..Default::default()
    });
    // commands.spawn((
    //     AudioPlayer::new(default_handle),
    //     PlaybackSettings {
    //         mode: PlaybackMode::Once,
    //         paused: true,
    //         ..Default::default()
    //     },
    //     DefaultAudio,
    // ));
    // commands.spawn(AudioPlayer::new(
    //     asset_server.load("impactWood_medium_000.ogg"),
    // ));
}

// fn update_cubes(
//     mut update_list: ResMut<ui::UpdateList>,
//     mut query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut Visibility,
//     )>,
// ) {
//     for update_data in &update_list.vals {
//         if let Ok((mut transform, mut material, mut visibility)) = query.get_mut(update_data.entity)
//         {
//             //
//         }
//     }
//     update_list.vals.clear();
// }

// fn change_heights(
//     mut commands: Commands,
//     query: Query<(Entity, &mut Transform, &ui::ChangeHeight)>,
// ) {
//     for (entity, mut transform, component) in query {
//         //
//         transform.scale.y = component.height as f32;
//         commands.entity(entity).remove::<ui::ChangeHeight>();
//     }
// }
//
// fn change_materials(
//     mut commands: Commands,
//     cube_assets: Res<ui::CubeAssets>,
//     query: Query<(
//         Entity,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &ui::ChangeMaterial,
//     )>,
// ) {
//     for (entity, mut material, component) in query {
//         *material = cube_assets
//             .materials
//             .get(&component.parsed_warning)
//             .unwrap()
//             .clone();
//         commands.entity(entity).remove::<ui::ChangeMaterial>();
//     }
// }

fn finish_timers(
    mut copy_timer: ResMut<ui::CopyTimer>,
    mut increment_timer: ResMut<sorter::IncrementTimer>,
) {
    copy_timer.copy_timer.finish();
    increment_timer.increment_timer.finish();
}

fn get_sorted_indices(values: &[f64]) -> Vec<usize> {
    // todo: sort indices by original value to get final positions
    let mut sorted_indices: Vec<usize> = (0..values.len()).collect();

    sorted_indices.sort_by(|&i, &j| values[i].partial_cmp(&values[j]).unwrap());

    sorted_indices
}

fn font_scale_inputs(
    mut font_scale: ResMut<ui::FontScale>,
    (mut mouse_scroll_event, keyboard_input): (
        MessageReader<MouseWheel>,
        Res<ButtonInput<KeyCode>>,
    ),
) {
    if keyboard_input.pressed(KeyCode::ControlLeft) || keyboard_input.pressed(KeyCode::ControlRight)
    {
        // Keyboard:
        if keyboard_input.pressed(KeyCode::Equal) {
            ui::increase_font(&mut font_scale);
        }
        if keyboard_input.pressed(KeyCode::Minus) {
            ui::decrease_font(&mut font_scale);
        }
        // Mouse scroll:
        for ev in mouse_scroll_event.read() {
            if ev.y > 0.0 {
                ui::increase_font(&mut font_scale);
            } else {
                ui::decrease_font(&mut font_scale);
            }
        }
    }
}

fn spawn_cube_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ui::CubeAssets {
        mesh: Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        materials: HashMap::from([
            (
                ui::ParsedWarning::Ok,
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba_u8(202, 211, 245, 0),
                    // emissive: LinearRgba {
                    //     red: 202.0,
                    //     green: 211.0,
                    //     blue: 245.0,
                    //     alpha: 0.0,
                    // },
                    unlit: true,
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::Error,
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba_u8(237, 135, 150, 0),
                    emissive: LinearRgba {
                        red: 37.0,
                        green: 135.0,
                        blue: 150.0,
                        alpha: 0.0,
                    },
                    // unlit: true,
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::PrecisionLoss,
                MeshMaterial3d(materials.add(StandardMaterial {
                    // base_color: Color::linear_rgb(238.0, 212.0, 159.0),
                    base_color: Color::srgba_u8(238, 212, 159, 0),
                    // base_color: Color::hsl,
                    // emissive: LinearRgba {
                    //     red: 238.0,
                    //     green: 212.0,
                    //     blue: 159.0,
                    //     alpha: 0.0,
                    // },
                    unlit: true,
                    fog_enabled: false,
                    ..Default::default()
                })),
            ),
        ]),
    });
}

// fn center_camera(mut query: Query<&mut Transform, With<PanOrbitCamera>>) {
//     let mut pan_orbit = query.single_mut().unwrap();
//     // pan_orbit.translation.x =
// }

fn spawn_3d_camera(mut commands: Commands) {
    // let problem_values = ProblemValues::new();
    // for key in problem_values.map.keys(){
    //     log::info!("{}", key)
    // }
    // log::info!("all keys printed");

    // todo: replace with own camera
    // add normal keyboard controls
    // add own mouse controls: right click should also work
    // add touch controls: left-right is faster than up-down in this solution... Strange.
    // game pad controls: unnecessary but add if easy
    // track pad: zooms
    // also: do not zoom when ctrl is pressed: that is for zooming in and out UI
    // do not zoom when pinching in within ui: in that case scale the UI instead?
    commands.spawn((
        PanOrbitCamera {
            focus: Vec3::ZERO,
            allow_upside_down: true,
            target_focus: Vec3::ZERO,
            zoom_lower_limit: 10.0,
            pan_sensitivity: 0.0,   // disable panning
            orbit_sensitivity: 2.0, // orbit faster
            // button_orbit: (MouseButton::Right, MouseButton::Left),
            orbit_smoothness: 0.0, // orbit without any smoothing
            ..Default::default()
        },
        Tonemapping::None, // more accurate colors
        // Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    log::info!("camera spawned")
}
