use bevy::app::TaskPoolPlugin;
use bevy::asset::{AssetApp, AssetPlugin, Assets};
use bevy::camera::{Camera3d, CameraPlugin};
use bevy::color::Color;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::image::ImagePlugin;
use bevy::input::InputPlugin;
use bevy::math::primitives::Cuboid;
use bevy::mesh::{CuboidMeshBuilder, Mesh, Mesh3d, MeshBuilder, MeshPlugin};
use bevy::prelude::{
    App, Commands, Component, IntoScheduleConfigs, Plugin, Query, Res, ResMut, Resource, Startup,
    Update, WindowPlugin,
};
use bevy::render::RenderPlugin;
use bevy::shader::Shader;
use bevy::transform::components::Transform;
use bevy::window::{MonitorSelection, VideoModeSelection, Window, WindowTheme};
use bevy::winit::{WakeUp, WinitPlugin};
use bevy::{log, shader};

use bevy::a11y::{self, AccessibilityPlugin, AccessibilityRequested, ManageAccessibilityUpdates};

use bevy::ecs::message::{Message, MessageRegistry}; // required by winit to handle Window Messages

use bevy::pbr::{MeshMaterial3d, PbrPlugin, StandardMaterial};

fn main() {
    App::new()
        // sending logs to console in browser:
        .add_plugins((
            bevy::app::PanicHandlerPlugin,
            bevy::log::LogPlugin {
                level: log::Level::DEBUG,
                filter: "".to_string(),
                // filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
                custom_layer: |_| None,
                fmt_layer: |_| None,
            },
        ))
        .add_plugins(TaskPoolPlugin::default())
        .add_plugins(InputPlugin)
        // Spawns a primary window
        // .add_plugins(WindowPlugin::default())
        .add_plugins(WindowPlugin {
            primary_window: Some(Window {
                title: "My Window".to_string(),
                window_theme: Some(WindowTheme::Dark),
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
        })
        .add_plugins(AccessibilityPlugin)
        .add_plugins(AssetPlugin::default())
        // Starts the event loop:
        // Required for Winit:
        .add_plugins(WinitPlugin::<WakeUp>::default())
        .add_plugins(RenderPlugin::default())
        .add_plugins(ImagePlugin::default())
        .add_plugins(MeshPlugin)
        .add_plugins(CameraPlugin)
        .init_asset::<bevy::shader::Shader>()
        .add_plugins(CorePipelinePlugin)
        .add_plugins(PbrPlugin::default())
        // app.init_asset::<bevy_shader::shader::Shader>()
        // .add_systems(Startup, spawn_3d_camera)
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
    // let my_mesh = meshes.add(Cuboid::new(1.2, 3.4, 5.6));
    // let my_cube_mesh = Mesh3d(my_mesh);
    // cube color
    // let my_material = materials.add(Color::srgba_u8(0, 10, 0, 0));
    // let my_cube_color = MeshMaterial3d(my_material);
    // cube position
    // let my_cube_position = Transform::from_xyz(0.0, 0.0, 0.0);

    // let my_cube = (my_cube_mesh, my_cube_color, my_cube_position);
    // commands.spawn(my_cube);

    // commands.spawn(Mesh::from(
    //     CuboidMeshBuilder::from(Cuboid::new(1.2, 3.4, 5.6)).build(),
    // ));
}

fn spawn_3d_camera(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}

fn test_system() {
    log::info!("this is my system...")
}
