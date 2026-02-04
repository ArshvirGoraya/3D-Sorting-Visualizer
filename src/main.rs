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
    post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
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
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum WasmAudioReceiverListening {
    #[default]
    NotListening,
    Listening,
}

pub const PROGRAM_TITLE: &str = "3D Sorting";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum CameraControlsFollow {
    #[default]
    NotFollowing,
    Following,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum CameraControlsAutoRotate {
    #[default]
    NotAutoRotate,
    AutoRotate,
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

#[derive(Resource)]
pub struct RNGValuesControls {
    amount: usize,
    min: f64,
    max: f64,
}
impl Default for RNGValuesControls {
    fn default() -> Self {
        Self {
            amount: 25,
            min: -50.0,
            max: 50.0,
        }
    }
}
#[derive(Resource)]
pub struct RNGColorControls {
    rng_cubes_enabled: bool,
    background_color: [u8; 3],
}

#[derive(Resource)]
pub struct CubeScaleControls {
    positional_heights: bool,
    height_scale_enable: bool,
    height_scale: f64,
    width_scale_enable: bool,
    width_scale: f64,
}

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
    // .init_resource::<CameraControls>()
    // .insert_resource(ClearColorConfig)
    .insert_resource(ClearColor {
        ..Default::default()
    })
    .insert_resource(ui::CopyTimer {
        copy_timer: Timer::from_seconds(1.0, TimerMode::Once),
    })
    .insert_resource(sorter::IncrementTimer {
        duration: Duration::new(0, 0),
        increment_timer: Timer::from_seconds(0.0, TimerMode::Once),
        duration_f64: 0.0,
    })
    .insert_resource(RNGColorControls {
        rng_cubes_enabled: true,
        background_color: [43, 44, 47],
    })
    .insert_resource(CubeScaleControls {
        positional_heights: true,
        height_scale_enable: false,
        height_scale: 1.0,
        width_scale_enable: false,
        width_scale: 5.0,
    })
    // set tonemapping to none for accurate color
    //
    .init_state::<sorter::SortState>()
    .init_state::<CameraControlsFollow>()
    .init_state::<CameraControlsAutoRotate>();

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

    app.add_systems(
        Update,
        auto_rotate_camera.run_if(in_state(CameraControlsAutoRotate::AutoRotate)),
    );

    // .add_systems(Update, audio_select.run_if(in_state(AudioPicking::Picking)))
    app.add_systems(Update, font_scale_inputs)
        .add_systems(EguiPrimaryContextPass, ui::ui_system);

    app.run();
}

fn play_audio(commands: &mut Commands, audio_controls: &mut ResMut<AudioControls>) {
    if !audio_controls.enabled {
        return;
    }
    let audio_source_handle = audio_controls
        .audio_source_handle
        .clone()
        .unwrap_or(audio_controls.audio_source_handle_default.clone());
    commands.spawn((
        AudioPlayer::new(audio_source_handle),
        PlaybackSettings {
            volume: Volume::Linear(audio_controls.volume),
            speed: audio_controls.pitch,
            mode: PlaybackMode::Despawn,
            ..Default::default()
        },
    ));
}

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
}

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

fn spawn_and_get_random_color_handle(
    materials: &mut ResMut<Assets<StandardMaterial>>,
    random: &mut ResMut<ui::Random>,
) -> MeshMaterial3d<StandardMaterial> {
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb_u8(random.rng.u8(..), random.rng.u8(..), random.rng.u8(..)),
        unlit: true,
        ..Default::default()
    }))
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
                    base_color: Color::srgb_u8(202, 211, 245),
                    unlit: true,
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::Error,
                MeshMaterial3d(materials.add(StandardMaterial {
                    // base_color: Color::srgb_u8(237, 135, 150),
                    // unlit: true,
                    emissive: LinearRgba {
                        red: 50.0,
                        green: 00.0,
                        blue: 0.0,
                        alpha: 0.0,
                    },
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::PrecisionLoss,
                MeshMaterial3d(materials.add(StandardMaterial {
                    // base_color: Color::srgb_u8(238, 212, 159),
                    // unlit: true,
                    emissive: LinearRgba {
                        red: 50.0,
                        green: 50.0,
                        blue: 0.0,
                        alpha: 0.0,
                    },
                    ..Default::default()
                })),
            ),
        ]),
    });
}

fn auto_rotate_camera(mut camera_query: Query<&mut PanOrbitCamera>) {
    let mut pan_orbit = camera_query.single_mut().unwrap();
    pan_orbit.target_yaw += 0.04;
    // only runs when auto rotate state is true.
}

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
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Bloom::NATURAL,
        // Bloom::ANAMORPHIC,
        // Bloom::OLD_SCHOOL,
        // Bloom::SCREEN_BLUR,
        Bloom {
            intensity: 0.1,
            prefilter: BloomPrefilter {
                threshold: 1.0,
                ..Default::default()
            },
            low_frequency_boost_curvature: 0.0,
            composite_mode: BloomCompositeMode::EnergyConserving,
            scale: Vec2::new(0.0, 0.5),
            ..Default::default()
        },
    ));
    log::info!("camera spawned")
}
