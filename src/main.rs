use bevy::{color, prelude::*};

use bevy_egui::{
    EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle,
    egui::{self, Color32},
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use regex_lite::Regex;

const PROGRAM_TITLE: &str = "3D Sorting";

#[derive(Default)]
pub struct UiState {}

#[derive(Resource)]
pub struct GithubImage {
    texture_id: egui::TextureId,
}

#[derive(Resource, Default)]
pub struct TextInput {
    val: String,
}

#[derive(Resource)]
pub struct NumberRegex {
    re: Regex,
}

impl Default for NumberRegex {
    fn default() -> Self {
        // matches integers, floats, scientific notion
        Self {
            re: Regex::new(r"-?\d+(?:\.\d+)?(?:[eE]-?\d+)?").expect("valid regex"),
        }
    }
}

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
        .insert_resource(TextInput::default())
        // .insert_resource(NumberRegex::default())
        .init_resource::<NumberRegex>()
        .add_systems(Startup, spawn_3d_camera)
        .add_systems(Startup, spawn_a_cube)
        .add_systems(Startup, get_texture_ids)
        .add_systems(EguiPrimaryContextPass, ui_system)
        // .add_systems(Update, test_system)
        .run();
}

fn get_texture_ids(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut egui_textures: ResMut<bevy_egui::EguiUserTextures>,
) {
    let handle: Handle<Image> = asset_server.load("github-white.png");
    let texture_id = egui_textures.add_image(EguiTextureHandle::Strong(handle));
    commands.insert_resource(GithubImage { texture_id });
}

fn detect_precision_loss(original: &str, parsed: f64) -> bool {
    let roundtrip = format!("{:.17}", parsed); // max precision
    normalize_string(original) != normalize_string(&roundtrip)
}

fn normalize_string(s: &str) -> String {
    let mut s = s.trim().to_lowercase();
    // Remove leading zeros (except before decimal)
    if let Some(pos) = s.find('.') {
        let (int, frac) = s.split_at(pos);
        let int = int.trim_start_matches('0');
        s = format!("{}{}", if int.is_empty() { "0" } else { int }, frac);
    } else {
        s = s.trim_start_matches('0').to_string();
        if s.is_empty() {
            s = "0".to_string();
        }
    }
    // Remove trailing zeros after decimal
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

fn ui_system(
    mut contexts: EguiContexts,
    mut ui_state: Local<UiState>,
    github_image: Res<GithubImage>,
    mut img_loaded: Local<bool>,
    mut rendered_texture_id: Local<egui::TextureId>,
    image_assets: ResMut<Assets<Image>>,
    mut my_text: Local<String>,
    // mut my_text: ResMut<TextInput>,
    number_regex: Res<NumberRegex>,
) -> Result {
    egui::Window::new(PROGRAM_TITLE).show(contexts.ctx_mut()?, |ui| {
        let github_button = egui::Button::image(egui::load::SizedTexture::new(
            github_image.texture_id,
            // images.github.1,
            [25.0, 25.0],
        ));
        if ui.add(github_button).on_hover_text("https://github.com/ArshvirGoraya/3D-Sorting-Visualizer").clicked() {
            ui.ctx().open_url(egui::OpenUrl {
                new_tab: true,
                url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
            });
        }
        if ui
            .add(egui::TextEdit::multiline(&mut *my_text).hint_text("numbers here")).on_hover_text("supports positive and negative ints, floats and scientific notations with the following regex: r\"-?\\d+(?:\\.\\d+)?(?:[eE]-?\\d+)?\"")
            .changed()
        {
            let parsed_numbers: Vec<f64> = number_regex
                .re
                .find_iter(&my_text)
                .filter_map(|m|{
                    let s = m.as_str();
                    match s.parse::<f64>(){
                        Ok(n) =>{
                            if n.is_infinite(){
                                log::warn!("over-flowed number: {}", s);
                            }else if n == 0.0 && !s.trim().starts_with('0'){
                                log::warn!("under-flowed number: {}", s);
                            }
                            else if detect_precision_loss(s, n){
                                log::warn!("precision loss on number: {}", s);
                            }
                            else if n.is_nan(){
                                log::error!("number is NaN: {}", s);
                            }
                            Some(n)
                        },
                        Err(err) => {
                            log::error!("failed to parse: {} with err {}", s, err);
                            None
                        }
                    }
                })
                .collect();
            log::info!("parsed numbers: {:?}", parsed_numbers);
        }
    });
    Ok(())
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
