mod ui;

use bevy::{input::mouse::MouseWheel, prelude::*};

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

pub const PROGRAM_TITLE: &str = "3D Sorting";

fn main() {
    App::new()
        // sending logs to console in browser:
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: PROGRAM_TITLE.to_string(),
                // resolution: WindowResolution {
                //     ..Default::default()
                // },
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
        }))
        // https://github.com/Plonq/bevy_panorbit_camera
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(EguiPlugin::default())
        // .insert_resource(NumberRegex::default())
        .init_resource::<ui::NumberRegex>()
        .init_resource::<ui::ParsedValues>()
        .init_resource::<ui::FontScale>()
        .insert_resource(ui::CopyTimer {
            copy_timer: Timer::from_seconds(1.0, TimerMode::Once),
        })
        // .insert_resource(FontScale { scale: 1.0, max: 10.0, min: 0.1, scale_step: 0.1 })
        .insert_resource(ui::ProblemValues::new())
        // .add_systems(Startup, tests)
        .add_systems(Startup, finish_copy_timer)
        .add_systems(Startup, spawn_3d_camera)
        .add_systems(Startup, spawn_a_cube)
        .add_systems(Update, font_scale_inputs)
        .add_systems(EguiPrimaryContextPass, ui::ui_system)
        // .add_systems(Update, test_system)
        .run();
}

fn finish_copy_timer(mut copy_timer: ResMut<ui::CopyTimer>) {
    copy_timer.copy_timer.finish();
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

fn spawn_a_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // cube mesh
    let my_mesh = meshes.add(Cuboid::new(1.2, 3.4, 5.6));
    let my_cube_mesh = Mesh3d(my_mesh);
    // cube color
    let my_material = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0, u8::MAX, 0, u8::MAX),
        // unlit: true,
        // alpha_mode: AlphaMode::Opaque,
        emissive: LinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 20.0,
            alpha: 0.0,
        },
        ..Default::default()
    });
    let my_cube_color = MeshMaterial3d(my_material);
    // cube position
    let my_cube_position = Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(5.0, 5.0, 5.0));

    let my_cube = (my_cube_mesh, my_cube_color, my_cube_position);
    commands.spawn(my_cube);

    // commands.spawn(Mesh::from(
    //     CuboidMeshBuilder::from(Cuboid::new(1.2, 3.4, 5.6)).build(),
    // ));
    log::info!("cube spawned")
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
        // Camera3d::default(),
        Transform::from_xyz(100.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    log::info!("camera spawned")
}
