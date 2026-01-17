mod ui;

use bevy::{input::mouse::MouseWheel, platform::collections::HashMap, prelude::*};

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

pub const PROGRAM_TITLE: &str = "3D Sorting";

fn main() {
    App::new()
        // .add_message::<ui::UpdateCubes>()
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
        .init_resource::<ui::Random>()
        .init_resource::<ui::ParsedValues>()
        // .init_resource::<ui::UpdateList>()
        .init_resource::<ui::FontScale>()
        .init_resource::<ui::UserText>()
        .insert_resource(ui::CopyTimer {
            copy_timer: Timer::from_seconds(1.0, TimerMode::Once),
        })
        // .insert_resource(FontScale { scale: 1.0, max: 10.0, min: 0.1, scale_step: 0.1 })
        // .insert_resource(ui::ProblemValues::new())
        // .add_systems(Startup, tests)
        .add_systems(Startup, finish_copy_timer)
        .add_systems(Startup, spawn_3d_camera)
        // .add_systems(Startup, spawn_a_cube)
        .add_systems(
            Startup,
            (spawn_cube_assets, ui::spawn_random_parsed_values).chain(),
        )
        // .add_systems(Update, update_cubes.run_if(on_message::<ui::UpdateCubes>))
        // .add_systems(
        //     Update,
        //     change_heights.run_if(on_message::<ui::ChangeHeights>),
        // )
        // .add_systems(
        //     Update,
        //     change_materials.run_if(on_message::<ui::ChangeMaterials>),
        // )
        .add_systems(Update, font_scale_inputs)
        .add_systems(EguiPrimaryContextPass, ui::ui_system)
        // .add_systems(Update, test_system)
        .run();
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
                    base_color: Color::linear_rgb(202.0, 211.0, 245.0),
                    // emissive: LinearRgba {
                    //     red: 202.0,
                    //     green: 211.0,
                    //     blue: 245.0,
                    //     alpha: 0.0,
                    // },
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::Error,
                MeshMaterial3d(materials.add(StandardMaterial {
                    // base_color: Color::linear_rgb(37.0, 135.0, 150.0),
                    emissive: LinearRgba {
                        red: 37.0,
                        green: 135.0,
                        blue: 150.0,
                        alpha: 0.0,
                    },
                    ..Default::default()
                })),
            ),
            (
                ui::ParsedWarning::PrecisionLoss,
                MeshMaterial3d(materials.add(StandardMaterial {
                    // base_color: Color::linear_rgb(238.0, 212.0, 159.0),
                    emissive: LinearRgba {
                        red: 238.0,
                        green: 212.0,
                        blue: 159.0,
                        alpha: 0.0,
                    },
                    ..Default::default()
                })),
            ),
        ]),
    });
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
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    log::info!("camera spawned")
}
