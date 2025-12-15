use bevy::prelude::*;

use bevy_panorbit_camera;

fn main() {
    App::new()
        // sending logs to console in browser:
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Window".to_string(),
                window_theme: Some(bevy::window::WindowTheme::Dark),
                recognize_doubletap_gesture: true,
                recognize_pinch_gesture: true,
                recognize_rotation_gesture: true,
                recognize_pan_gesture: Some((1, 1)), // for iOS
                // present_mode: bevy::window::PresentMode::Fifo..Default::default(),
                // mode: bevy::window::WindowMode::Fullscreen(
                //     MonitorSelection::Current,
                //     VideoModeSelection::Current,
                // ),
                // resolution: WindowResolution
                ..Default::default()
            }),
            ..Default::default()
        }))
        // https://github.com/Plonq/bevy_panorbit_camera
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)
        .add_systems(Startup, spawn_3d_camera)
        .add_systems(Startup, spawn_a_cube)
        // .add_systems(Update, test_system)
        .run();
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
    commands.spawn((
        bevy_panorbit_camera::PanOrbitCamera {
            focus: Vec3::ZERO,
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

fn test_system() {
    log::info!("this is my system...")
}
