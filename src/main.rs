pub mod sorter;
mod ui;

#[cfg(any(target_arch = "wasm32", rust_analyzer))]
mod wasm_audio_picker;

use bevy::{
    asset::AssetMetaCheck,
    audio::{PlaybackMode, Volume},
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::MouseWheel,
    platform::collections::HashMap,
    post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
    prelude::*,
};

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass, egui::style::Interaction};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use core::time::Duration;

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
    // pitch: f32,
    low_pitch: f32,
    high_pitch: f32,
    pitch_range: f32,
    default_file_name: String,
    audio_source_handle_default: Handle<AudioSource>,
    selected_file_name: Option<String>,
    audio_source_handle: Option<Handle<AudioSource>>,
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
            amount: 10,
            min: -50.0,
            max: 50.0,
        }
    }
}
#[derive(Resource)]
pub struct RNGColorControls {
    pub rng_cubes_enabled: bool,
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

#[derive(Resource, Default)]
pub struct HoveredCube {
    id: Option<Entity>,
    display: bool,
    screen_position: Vec2,
    value: f64,
    starting_pos: usize,
    ending_pos: usize,
}

#[derive(Component)]
pub struct CubeData {
    index: usize, // matches the index at ParsedValues.val (which holds this cube's data).
}

#[derive(Resource, Default)]
pub struct ScannedCube{
    entity: Option<Entity>
}


// Marker Components:
#[derive(Component)]
pub struct DefaultAudio;
#[derive(Component)]
pub struct SelectedAudio;

#[derive(Message)]
pub struct StopAllAudio;

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
                // for assets to work on browser
                meta_check: AssetMetaCheck::Never,
                ..Default::default()
            }),
    )
    // https://github.com/Plonq/bevy_panorbit_camera
    .add_plugins(PanOrbitCameraPlugin)
    .add_plugins(EguiPlugin::default())
    .add_plugins(MeshPickingPlugin)
    .insert_resource(MeshPickingSettings {
        require_markers: true,
        ..Default::default()
    })
    .init_resource::<ui::NumberRegex>()
    .init_resource::<ui::Random>()
    .init_resource::<ui::ParsedValues>()
    .init_resource::<ui::FontScale>()
    .init_resource::<ui::UserText>()
    .init_resource::<HoveredCube>()
    .init_resource::<ScannedCube>()
    .init_resource::<RNGValuesControls>()
    .insert_resource(ClearColor {
        ..Default::default()
    })
    .insert_resource(ui::CopyTimer {
        copy_timer: Timer::from_seconds(1.0, TimerMode::Once),
    })
    .insert_resource(sorter::IncrementTimer {
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
    .init_state::<CameraControlsFollow>()
    .init_state::<CameraControlsAutoRotate>();

    #[cfg(any(target_arch = "wasm32", rust_analyzer))]
    app.init_state::<WasmAudioReceiverListening>();

    app.add_systems(Startup, tests);
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
    app.add_systems(Update, font_scale_inputs)
        .add_systems(
            Update,
            auto_rotate_camera.run_if(in_state(CameraControlsAutoRotate::AutoRotate)),
        )
        .add_systems(EguiPrimaryContextPass, ui::ui_system)
        .add_systems(
            Update,
            (detect_cube_hover_enter, detect_cube_hover_exit).chain(),
        )
        .add_systems(Update,
            keep_cube_centered
            .run_if(in_state(sorter::SortState::Sorting))
            .run_if(in_state(CameraControlsFollow::Following))
        )
        ;

    app.add_message::<StopAllAudio>();
    app.add_systems(Update, stop_all_audio.run_if(on_message::<StopAllAudio>));

    // sort systems
    app.init_state::<sorter::SortState>()
        .add_systems(OnExit(sorter::SortState::Sorting), center_camera_on_sort_exited.run_if(in_state(CameraControlsFollow::Following)))
        .init_state::<sorter::Algorithms>()
        // .add_systems(OnEnter(sorter::SortState::Sorting), sorter::begin_sorting)
        // Quick Sort:
        .init_resource::<sorter::quick_sort::QuickSortColors>()
        // .add_message::<sorter::quick_sort::SetupRange>()
        // .add_systems(
        //     Update, 
        //     sorter::quick_sort::setup_range
        //     .run_if(on_message::<sorter::quick_sort::SetupRange>)
        //     .run_if(in_state(sorter::Algorithms::QuickSort))
        //     .run_if(in_state(sorter::SortState::Sorting))
        // )
        // .add_message::<sorter::quick_sort::Swap>()
        // .add_systems(
        //     Update, 
        //     sorter::quick_sort::swap
        //     .run_if(on_message::<sorter::quick_sort::Swap>)
        //     .run_if(in_state(sorter::Algorithms::QuickSort))
        //     .run_if(in_state(sorter::SortState::Sorting))
        // )
        // .add_message::<sorter::quick_sort::Compare>()
        // .add_systems(
        //     Update, 
        //     sorter::quick_sort::compare
        //     .run_if(on_message::<sorter::quick_sort::Compare>)
        //     .run_if(in_state(sorter::Algorithms::QuickSort))
        //     .run_if(in_state(sorter::SortState::Sorting))
        //     // ensure this main function doesn't run at the same time as any others (ideally all
        //     // sort functions NEVER run in parallel).
        //     .after(sorter::quick_sort::setup_range)
        //     .before(sorter::quick_sort::swap)
        // )
        .add_systems(
            Update, sorter::quick_sort::increment_sorting
            .run_if(in_state(sorter::SortState::Sorting))
            .run_if(in_state(sorter::Algorithms::QuickSort))
            )
        .add_systems(
            OnEnter(sorter::SortState::NotSorting), 
            sorter::quick_sort::complete.run_if(in_state(sorter::Algorithms::QuickSort))
        )
        // separate comma
        ;

    app.run();
}

fn tests(){
    let test_vec = [1, 1, 1, 1, 1];
    let mut sorted_indices: Vec<usize> = (0..test_vec.len()).collect();
    // sort by the values inside of these indices.
    sorted_indices.sort_by(
        |&i, &j| {
            test_vec[i].partial_cmp(&test_vec[j])
    }.unwrap());
    log::info!("[{}, {}, {}, {}, {}]", 
        sorted_indices[0],
        sorted_indices[1],
        sorted_indices[2],
        sorted_indices[3],
        sorted_indices[4]
        );
}

pub fn keep_cube_centered(
    transform: Query<&Transform, With<CubeData>>,
    scanned_cube: Res<ScannedCube>,
    mut camera_query: Query<&mut PanOrbitCamera>,
    ){
    // INFO: runs if in state: Sorting, and Following.
    if let Some(scanned_cube) = scanned_cube.entity{
        // INFO: the scanned_cube is set when sorting (when algorithm scans through all the cubes).
        center_camera_on_cube(transform.get(scanned_cube).unwrap(), &mut camera_query);
    }
}

pub fn detect_cube_clicked(
    mut event: On<Pointer<Click>>, 
    transform: Query<&Transform, With<CubeData>>, 
    mut camera_query: Query<&mut PanOrbitCamera>,
){
    // INFO: This system is attached to cubes when they are spawned
    center_camera_on_cube(transform.get(event.entity).unwrap(), &mut camera_query);
    event.propagate(false);
}

pub fn center_camera_on_sort_exited(
    mut camera_query: Query<&mut PanOrbitCamera>, 
    parsed_values: Res<ui::ParsedValues>, 
    cube_scale_controls: ResMut<crate::CubeScaleControls>,
){
    // once sort is exited (early on after finishing) and if we are following scan cube
    // recenter the camera
    let end_index = parsed_values.end_index;
    let cube_width = ui::get_cube_size_from_width_scale(parsed_values.end_index, &cube_scale_controls);
    center_camera_on_all_cubes(&mut camera_query, cube_width, end_index);
}

pub fn center_camera_on_all_cubes(camera_query: &mut Query<&mut PanOrbitCamera>, cube_width: f32, end_index: usize){
    let mut pan_orbit = camera_query.single_mut().unwrap();
    let center = cube_width * ((end_index) as f32) / 2.0;

    if !pan_orbit.initialized{
        // setting target_focus before initialization = doesn't do anything.
        // setting both target_focus and focus each time = doesn't update if egui is in focus
        // has to be set focus before initialization, and set target_focus afterwards.
        pan_orbit.focus.x = center;
    }else{
        pan_orbit.target_focus.x = center;
        pan_orbit.target_focus.y = 0.0;
    }

}

pub fn center_camera_on_cube(cube_transform: &Transform, camera_query: &mut Query<&mut PanOrbitCamera>){
    let mut cube_center = cube_transform.translation;
    cube_center.x += cube_transform.scale.x / 2.0;

    let mut pan_orbit = camera_query.single_mut().unwrap();
    pan_orbit.target_focus = cube_center;
    // log::info!("Cursor clicked a cube at position: {}", transform.get(event.entity).unwrap().translation.x);
}

fn detect_cube_hover_enter(
    mut event_hover_enter: MessageReader<Pointer<Over>>,
    mut hovered_cube: ResMut<HoveredCube>,
    cube_query: Query<Entity, With<CubeData>>,
) {
    if let Some(e) = event_hover_enter.read().last()
        && let Ok(cube_id) = cube_query.get(e.entity)
    {
        // log::info!("hovered in: {}", cube_id);
        hovered_cube.id = Some(cube_id);
        hovered_cube.display = true;
    }
}

fn detect_cube_hover_exit(
    mut event_hover_exit: MessageReader<Pointer<Out>>,
    mut hovered_cube: ResMut<HoveredCube>,
    cube_query: Query<Entity, With<CubeData>>,
) {
    if let Some(hovered_cube_id) = hovered_cube.id {
        for e in event_hover_exit.read() {
            if let Ok(exited_cube) = cube_query.get(e.entity)
                && exited_cube == hovered_cube_id
            {
                // log::info!("hovered out: {}", hovered_cube_id);
                hovered_cube.id = None;
                hovered_cube.display = false;
                return;
            }
        }
    }
}


fn stop_all_audio(
    mut commands: Commands,
    audio_query: Query<Entity, With<AudioPlayer>>,
    ){
    for e in audio_query.iter(){
        commands.entity(e).despawn();
    }
}



fn play_audio(
    commands: &mut Commands, 
    audio_controls: &Res<AudioControls>,
    cube_index: usize,
    total_cubes: usize,
) {
    if !audio_controls.enabled {
        return;
    }
    // normalize selected cube
    // 1 minus to reverse the pitch: higher values = deeper, and lower values = higher
    let normalized_cube = 1.0 - (cube_index as f32 / total_cubes as f32);
    // get pitch from normalized cube
    // let min_pitch = audio_controls.pitch * 0.75;
    // let max_pitch = audio_controls.pitch * 1.25;
    // let range = max_pitch - min_pitch; 
    let pitch = audio_controls.low_pitch + normalized_cube * audio_controls.pitch_range;
    
    // let pitch = audio_controls.pitch / (cube_index+1) as f32;
    log::info!("pitch for cube {}: {}", cube_index, pitch);

    let audio_source_handle = audio_controls
        .audio_source_handle
        .clone()
        .unwrap_or(audio_controls.audio_source_handle_default.clone());

    commands.spawn((
        AudioPlayer::new(audio_source_handle),
        PlaybackSettings {
            volume: Volume::Linear(audio_controls.volume),
            speed: pitch,
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
    // let default_pitch = 1.0;
    let high_pitch = 2.0;
    let low_pitch = 0.5;
    let pitch_range = high_pitch - low_pitch;
    //
    commands.insert_resource(AudioControls {
        volume: default_volume,
        // pitch: default_pitch,
        low_pitch, 
        high_pitch,
        pitch_range,
        enabled: true,
        default_file_name: file_name,
        audio_source_handle_default: default_handle,
        // if these are none: the default audio plays
        selected_file_name: None,
        audio_source_handle: None,
        // ..Default::default()
    });
}

fn finish_timers(
    mut copy_timer: ResMut<ui::CopyTimer>,
    mut increment_timer: ResMut<sorter::IncrementTimer>,
) {
    copy_timer.copy_timer.finish();
    increment_timer.increment_timer.finish();
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
    commands.spawn((
        MeshPickingCamera::default(),
        PanOrbitCamera {
            focus: Vec3::ZERO,
            allow_upside_down: true,
            target_focus: Vec3::ZERO,
            zoom_lower_limit: 10.0,
            pan_sensitivity: 0.0,   // disable panning
            orbit_sensitivity: 2.0, // orbit faster
            // button_orbit: (MouseButton::Right, MouseButton::Left),
            orbit_smoothness: 0.01, // orbit without any smoothing
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
