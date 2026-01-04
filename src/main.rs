use bevy::{input::mouse::MouseWheel, prelude::*};

use bevy_egui::{
    EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle,
    egui::{self, Layout, Margin, Spacing, vec2},
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use regex_lite::Regex;

const PROGRAM_TITLE: &str = "3D Sorting";

#[derive(Resource)]
pub struct FontScale {
    scale: f32,
    max: f32,
    min: f32,
    scale_step: f32,
}

#[derive(Resource)]
pub struct ImageIds{
    github: egui::TextureId,
    zoom_in: egui::TextureId,
    zoom_out: egui::TextureId,
    clipboard: egui::TextureId,
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
        // .insert_resource(NumberRegex::default())
        .init_resource::<NumberRegex>()
        .insert_resource(FontScale { scale: 1.0, max: 10.0, min: 0.1, scale_step: 0.1 })
        .add_systems(Startup, spawn_3d_camera)
        .add_systems(Startup, spawn_a_cube)
        .add_systems(Startup, get_texture_ids)
        .add_systems(Update, font_scale_inputs)
        .add_systems(EguiPrimaryContextPass, ui_system)
        // .add_systems(Update, test_system)
        .run();
}

fn get_texture_ids(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut egui_textures: ResMut<bevy_egui::EguiUserTextures>,
) {
    commands.insert_resource(ImageIds{
        github: egui_textures.add_image(EguiTextureHandle::Strong(asset_server.load("github-white.png"))),
        zoom_in: egui_textures.add_image(EguiTextureHandle::Strong(asset_server.load("zoom-in.png"))),
        zoom_out: egui_textures.add_image(EguiTextureHandle::Strong(asset_server.load("zoom-out.png"))),
        clipboard: egui_textures.add_image(EguiTextureHandle::Strong(asset_server.load("clipboard.png"))),
    });
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

fn get_sort_indices(values: &[f64]) -> Vec<usize> {
    // todo: sort indices by orinal value to get final positions
    let mut sorted_indices: Vec<usize> = (0..values.len()).collect();

    sorted_indices.sort_by(|&i, &j| values[i].partial_cmp(&values[j]).unwrap());

    sorted_indices
}

fn increase_font(font_scale: &mut FontScale){
    font_scale.scale = f32::min(font_scale.max, font_scale.scale + font_scale.scale_step);
}
fn decrease_font(font_scale: &mut FontScale){
    font_scale.scale = f32::max(font_scale.min, font_scale.scale - font_scale.scale_step);
}

fn font_scale_inputs(
    mut font_scale: ResMut<FontScale>,
    (mut mouse_scroll_event, keyboard_input): (MessageReader<MouseWheel>, Res<ButtonInput<KeyCode>>),
    ){
    if keyboard_input.pressed(KeyCode::ControlLeft) || keyboard_input.pressed(KeyCode::ControlRight){
        // Keyboard:
        if keyboard_input.pressed(KeyCode::Equal) {
            increase_font(&mut font_scale);
        }
        if keyboard_input.pressed(KeyCode::Minus) {
            decrease_font(&mut font_scale);
        }
        // Mouse scroll:
        for ev in mouse_scroll_event.read(){
            if ev.y > 0.0 {
                increase_font(&mut font_scale);
            } else {
                decrease_font(&mut font_scale);
            }
        }
    }
}

fn scale_ui(style: &mut egui::Style, scale : f32){
    use egui::FontFamily::Proportional;
    use egui::FontId;
    use egui::TextStyle::*;


    // go to definition of egui::Style for these defaults 
    style.text_styles = [
        (Heading, FontId::new(30.0 * scale, Proportional)),
        // (Name("Heading2".into()), FontId::new(25.0 * scale, Proportional)),
        // (Name("Context".into()), FontId::new(23.0 * scale, Proportional)),
        (Body, FontId::new(18.0 * scale, Proportional)),
        (Monospace, FontId::new(14.0 * scale, Proportional)),
        (Button, FontId::new(14.0 * scale, Proportional)),
        (Small, FontId::new(10.0 * scale, Proportional)),
    ].into();

    style.spacing = Spacing  {
            item_spacing: vec2(8.0, 3.0) * scale,
            window_margin: Margin::same(6 * scale as i8),
            menu_margin: Margin::same(6 * scale as i8),
            button_padding: vec2(4.0, 1.0) * scale,
            indent: 18.0 * scale,
            interact_size: vec2(40.0, 18.0) * scale,
            slider_width: 100.0 * scale,
            slider_rail_height: 8.0 * scale,
            combo_width: 100.0 * scale,
            text_edit_width: 280.0 * scale,
            icon_width: 14.0 * scale,
            icon_width_inner: 8.0 * scale,
            icon_spacing: 4.0 * scale,
            tooltip_width: 500.0 * scale,
            // default_area_size: vec2(600.0, 400.0) * scale,
            // menu_width: 400.0 * scale,
            // menu_spacing: 2.0 * scale,
            // combo_height: 200.0 * scale,
            ..Default::default()
    };
}


fn ui_system(
    mut contexts: EguiContexts,
    image_ids: Res<ImageIds>,
    mut my_text: Local<String>,
    number_regex: Res<NumberRegex>,
    mut clipboard: ResMut<bevy_egui::EguiClipboard>,
    mut font_scale: ResMut<FontScale>

) -> Result {
    let ctx = contexts.ctx_mut()?;
    let logo_size = 25.0 * font_scale.scale;

    egui::Window::new(PROGRAM_TITLE).show(ctx, |ui| {
        ui.horizontal(|ui|{
            if ui.add(egui::Button::image(egui::load::SizedTexture::new(
                        image_ids.github,
                        // images.github.1,
                        [logo_size, logo_size],
            ))).on_hover_text("https://github.com/ArshvirGoraya/3D-Sorting-Visualizer").clicked() {
                ui.ctx().open_url(egui::OpenUrl {
                    new_tab: true,
                    url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
                });
            }

            ui.with_layout(Layout::right_to_left(egui::Align::RIGHT), |ui|{
                if ui.add(egui::Button::image(egui::load::SizedTexture::new(
                            image_ids.zoom_in,
                            [logo_size, logo_size],
                ))).on_hover_text("Increase Font").clicked() {
                    increase_font(&mut font_scale);
                    log::info!("increased: {}", font_scale.scale)
                    // ui_state.font_size = f32::min(10.0, ui_state.font_size + 0.1);
                    // let ui_scale = ui_state.font_size;
                    // ui.ctx().all_styles_mut(move |style| {
                    //     scale_ui(style, ui_scale);
                    // });
                }
                if ui.add(egui::Button::image(egui::load::SizedTexture::new(
                            image_ids.zoom_out,
                            [logo_size, logo_size],
                ))).on_hover_text("Decrease Font").clicked() {
                    decrease_font(&mut font_scale);

                    log::info!("decreased: {}", font_scale.scale)

                    // ui_state.font_size = f32::max(0.1, ui_state.font_size - 0.1);
                    // let ui_scale = ui_state.font_size;
                    // ui.ctx().all_styles_mut(move |style| {
                    //     scale_ui(style, ui_scale);
                    // });
                }
            });
        });


        // Might be better to use a really small nerd font that in is smaller than all used images
        // if in aggregate the images are smaller than the font in size.
        // Can make a font with only the used logos too in order to get a very small font size?
        // if ui.button("").on_hover_text("go to github page").clicked(){
        //     ui.ctx().open_url(egui::OpenUrl {
        //         new_tab: true,
        //         url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
        //     });
        // }
        
        ui.horizontal(|ui|{
            if ui.add(egui::Button::image(egui::load::SizedTexture::new(
                        image_ids.clipboard,
                        [logo_size, logo_size],
            ))).on_hover_text("copy to clipboard").clicked() {
                clipboard.set_text(&my_text);
            }
            ui.colored_label(egui::Color32::from_rgb(255, 0, 0), "parse info");
        });

        if ui
            .add(egui::TextEdit::multiline(&mut *my_text).hint_text("numbers here").desired_width(ui.available_width())).on_hover_text("supports positive and negative ints, floats and scientific notations with the following regex expression: r\"-?\\d+(?:\\.\\d+)?(?:[eE]-?\\d+)?\"")
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

    if font_scale.is_changed() {
        log::info!("font is changed");
        let scale = font_scale.scale;
        ctx.all_styles_mut(move |style| {
            scale_ui(style, scale);
        });
    }

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
    // todo: replace with own camera
    // add normal keyboard controls
    // add own mouse controls: right click should also work
    // add touch controls: left-right is faster than up-down in this solution... Strange.
    // gamepad controls: unnecessary but add if easy
    // trackpad: zooms
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
