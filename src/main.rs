use core::{f64, fmt};

use bevy::{ecs::relationship::RelationshipSourceCollection, input::mouse::MouseWheel, platform::collections::HashMap, prelude::*, render::render_resource::encase::private::Truncate};

use bevy_egui::{
    EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiStartupSet, EguiTextureHandle, egui::{self, Layout, Margin, Spacing, vec2}
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use regex_lite::Regex;

const PROGRAM_TITLE: &str = "3D Sorting";

#[derive(Resource)]
pub struct FontScale {
    scale: f32,
    scale_step: f32,
    max: f32,
    min: f32,
}
impl Default for FontScale{
    fn default() -> Self {
        Self{
            scale: 1.0,
            scale_step: 0.1,
            max: 10.0,
            min: 0.1,
        }
    }
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
        // matches integers, floats, scientific notation
        Self {
            // re: Regex::new(r"-?\d+(?:\.\d+)?(?:[eE]-?\d+)?").expect("valid regex"),
            // no scientific notations included:
            // re: Regex::new(r"-?\d+(?:\.\d+)?").expect("valid regex"),
            // positive or minus, d.d, or .d, or d.
            re: Regex::new(r"-?(?:\d+\.\d*|\.\d+|\d+)").expect("valid regex"),
        }
    }
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
enum ParsedWarning{
    #[default] Ok,
    Error,
    PrecisionLoss
}

impl fmt::Display for ParsedWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsedWarning::Ok => write!(f, "Ok"),
            ParsedWarning::Error => write!(f, "Error"),
            ParsedWarning::PrecisionLoss => write!(f, "PrecisionLoss"),
        }
    }
}



#[derive(Default)]
pub struct StringInfo{
    start_index: usize,
    end_index: usize,
}

#[derive(Default)]
pub struct ParsedValue{
    raw_string: StringInfo,
    matched_string: StringInfo,
    converted_value: f64,
    parsed_warning: ParsedWarning,
    // final_position: int,
    // box_handle: int
}

#[derive(Resource)]
pub struct ProblemValues{
    // Stores indices in ParsedValues that have parse problems.
    map: HashMap<ParsedWarning, Vec<usize>>,
}

impl ProblemValues{
    fn new() -> Self{
        let mut map: HashMap<ParsedWarning, Vec<usize>> = HashMap::new();
        for key in [ParsedWarning::Ok, ParsedWarning::Error, ParsedWarning::PrecisionLoss]{
            map.insert(key, Vec::new());
        }
        Self{
            map
        }
    }
}


#[derive(Resource, Default)]
pub struct ParsedValues{
    vals: Vec<ParsedValue>,
    end_index: usize,
}

// impl ParsedValues{
//     // when iterating, use parsed_values.iter() instead of parsed_values.vals.iter() to get the 
//     // an iterator that iterates up to the end_index!
//     pub fn iter(&self) -> impl Iterator<Item = &ParsedValue>{
//         self.vals[..self.end_index].iter()
//     }
// }


#[derive(Default)]
pub struct NumString{
    requires_restring: bool,
    val: String,
}

#[derive(Default)]
pub struct ColoredStrings{
    requires_restring: bool,
    vals: Vec<egui::RichText>
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
        .init_resource::<ParsedValues>()
        .init_resource::<FontScale>()
        // .insert_resource(FontScale { scale: 1.0, max: 10.0, min: 0.1, scale_step: 0.1 })
        .insert_resource(ProblemValues::new())
        // .add_systems(Startup, tests)
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


fn tests(){
    // test_detect_precision_loss
    let regex_matches = [
        // positives:
        "1.1", ".1", "1", "0.0", ".0", "0",
        // negatives:
        "-1.1", "-.1", "-1", "-0.0", "-.0", "-0",
        // extras:
        "0000001.1", "0000001", "000000100000000",
        "1.0000000000000000", 
        // precision loss: this should NOT match
        "1.00000000000000000000001"
    ];
    regex_matches.iter().for_each(|original|{
        detect_precision_loss(original, original.parse::<f64>().unwrap());
    });
}

fn detect_precision_loss(original: &str, parsed: f64) -> bool {
    let num_string = parsed.to_string();
    let new_string = string_trim_zeros(original);
    log::info!("{original} turned to {new_string} == {parsed}");
    if new_string != num_string{
        log::info!("\tno match!")
    }
    new_string != num_string
}


fn string_trim_zeros(s: &str) -> String{
    let mut int = s;
    let mut frac: &str = "";
    let mut requires_minus = false;
    let mut requires_zero = false;

    // split into int and frac sections
    if let Some(pos) = s.find('.') {
        (int, frac) = s.split_at(pos);
        // for fractional part: remove all trailing 0's. If only '.' remains, remove that too (frac
        // will be empty in this case).
        frac = frac.trim_end_matches("0").trim_end_matches(".");
    }

    // remove minus from int slice if need to.
    if int.starts_with("-"){
        requires_minus = true;
        int = &int[1..int.len()];
    }
    // remove leading zeros from int slice.
    int = int.trim_start_matches("0");
    // check if a zero is required:
    if int.is_empty(){
        requires_zero = true;
    }

    // recombine int and frac. Add minus and zero as required.
    let requirements = (requires_minus, requires_zero);
    let new_string = match requirements{
        (true, true) => format!("-0{}{}", int, frac),
        (false, true) => format!("0{}{}", int, frac),
        (true, false) => format!("-{}{}", int, frac),
        (false, false) => format!("{}{}", int, frac),
    };

    return new_string;
}

fn get_sort_indices(values: &[f64]) -> Vec<usize> {
    // todo: sort indices by original value to get final positions
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


    // for (text_style, _font_id) in &style.text_styles{
    //     log::info!("textstyle: {}", text_style);
    // }

    // go to definition of egui::Style for these defaults 
    style.text_styles = [
        (Heading, FontId::new(30.0 * scale, Proportional)),
        // (Name("Heading2".into()), FontId::new(25.0 * scale, Proportional)),
        // (Name("Context".into()), FontId::new(23.0 * scale, Proportional)),
        (Body, FontId::new(18.0 * scale, Proportional)),
        (Monospace, FontId::new(14.0 * scale, Proportional)),
        (Button, FontId::new(14.0 * scale, Proportional)),
        (Small, FontId::new(10.0 * scale, Proportional)),
        // ((Name("symbol_font".into())), FontId::new(14.0 * scale, egui::FontFamily::Name("symbol_font".into())))
        ((Name("symbol_font".into())), FontId::new(25.0 * scale, Proportional))
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

fn add_parsed_value(parsed_values: &mut ParsedValues, converted_value: f64, parsed_warning: ParsedWarning, index: usize, match_start: usize, match_end: usize){
    let previous_raw_string_end_index = {
        if index == 0{
            0
        }else{
            parsed_values.vals.get(index - 1).unwrap().raw_string.end_index
        }
    };

    if let Some(parsed_value) = parsed_values.vals.get_mut(index) {
        // If vector already contains a parsed_value object at this index, just change its values:
        parsed_value.raw_string.start_index = previous_raw_string_end_index;
        parsed_value.raw_string.end_index = match_end;

        parsed_value.converted_value = converted_value;
        parsed_value.parsed_warning = parsed_warning;
        parsed_value.matched_string.start_index = match_start;
        parsed_value.matched_string.end_index = match_end;
    }else{
        log::info!("new Parsed value!");
        parsed_values.vals.push(ParsedValue{
            converted_value,
            parsed_warning,
            matched_string: StringInfo{start_index: match_start, end_index: match_end},
            raw_string: StringInfo{start_index: previous_raw_string_end_index, end_index: match_end}
        });
    }
}

#[allow(clippy::collapsible_if)]
fn set_worse_parse_problem(worse_parse_problem: &ParsedWarning, parsed_problem: ParsedWarning) -> ParsedWarning{
    if parsed_problem == ParsedWarning::PrecisionLoss{
        if *worse_parse_problem != ParsedWarning::Error {
            return ParsedWarning::PrecisionLoss;
        }
    }
    ParsedWarning::Error
}

fn get_parse_warning_color(worse_parse_problem: &ParsedWarning) -> (egui::Color32, &'static str){
    match *worse_parse_problem {
        ParsedWarning::Ok => (egui::Color32::LIGHT_GREEN, "No Parse Problems"),
        ParsedWarning::Error => (egui::Color32::LIGHT_RED, "Cannot Parse"),
        ParsedWarning::PrecisionLoss => (egui::Color32::ORANGE, "Precision Loss"),
    }
}

fn setup_font(ctx: &mut egui::Context){
    // push font data into the proportional font family. Egui will use it if it cant find the glyph it needs from
    // any other font in the family. You can make a new font family and put the font in that one
    // and choose when to use that font instead. But this is just for nerd font glyhps, so can just
    // add it to the normal font family and will be used when needed.

    // symbols in added from font: https://www.nerdfonts.com/cheat-sheet
        // font ONLY has these symbols in it and NOTHING else:
            //  (github - U+E709)
            //  (font increase - U+EB69)
            //  (font decrease - U+EB6A) // this glyph was changed from the standard nerd font glyph at this position to a flipped version of U+eb69.
            // 󰨸 (clipboard - U+F0A38)
            // 󰃢 (broom - U+F00E2)
        // this makes the font very small in size AND allows no pixelation when displaying these
        // symbols in different sizes (since they are essentially SVGs).

    let font_bytes = include_bytes!("../embedded_assets/fonts/symbol_font.ttf");
    let mut fonts = egui::FontDefinitions::default(); 
    fonts.font_data.insert("symbol_font".to_owned(), egui::FontData::from_static(font_bytes).into());
    fonts.families.entry(egui::FontFamily::Proportional).or_default().push("symbol_font".to_owned());
    ctx.set_fonts(fonts);
}


#[allow(clippy::too_many_arguments)]
fn ui_system(
    mut contexts: EguiContexts,
    image_ids: Res<ImageIds>,
    mut my_text: Local<String>,
    // mut my_parsed_text: Local<String>,
    number_regex: Res<NumberRegex>,
    mut clipboard: ResMut<bevy_egui::EguiClipboard>,
    mut font_scale: ResMut<FontScale>,
    mut font_added: Local<bool>,
    // mut parsed_numbers: Local<Vec<f64>>,
    mut text_is_dirty: Local<bool>,

    mut colored_strings: Local<ColoredStrings>,
    mut num_strings: Local<NumString>,

    mut parsed_values: ResMut<ParsedValues>,
    mut problem_values: ResMut<ProblemValues>,
    mut worse_parse_problem: Local<ParsedWarning>,

) -> Result {
    let ctx = contexts.ctx_mut()?;
    let logo_size = 30.0 * font_scale.scale;
    let logo_size_vec = vec2(logo_size, logo_size);


    if !*font_added{
        *font_added = true;
        setup_font(ctx);
        let scale = font_scale.scale;
        ctx.all_styles_mut(move |style| {
            scale_ui(style, scale);
        });
    }

    // log::info!("max_width: {max_width}");


    egui::Window::new(PROGRAM_TITLE).max_width(ctx.content_rect().width() * 0.37).show(ctx, |ui| {
        ui.horizontal(|ui|{
            ui.allocate_ui(logo_size_vec, |ui|{
                ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                if ui.add_sized(vec2(logo_size, logo_size), egui::Button::new("")).on_hover_text("https://github.com/ArshvirGoraya/3D-Sorting-Visualizer").clicked(){
                    ui.ctx().open_url(egui::OpenUrl {
                        new_tab: true,
                        url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
                    });
                }
                ui.style_mut().override_text_style = None;
            });
            ui.with_layout(Layout::right_to_left(egui::Align::RIGHT), |ui|{
                ui.allocate_ui(logo_size_vec, |ui|{
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                     if ui.add_sized(vec2(logo_size, logo_size), egui::Button::new("")).on_hover_text("Decrease Font").clicked(){
                        decrease_font(&mut font_scale);
                    }
                    if ui.add_sized(vec2(logo_size, logo_size), egui::Button::new("")).on_hover_text("Increase Font").clicked(){
                        increase_font(&mut font_scale);
                    }
                    ui.style_mut().override_text_style = None;
                });
            });
        });

        ui.horizontal(|ui|{
            ui.allocate_ui(vec2(logo_size, logo_size), |ui|{
                ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                if ui.add_sized(vec2(logo_size, logo_size), egui::Button::new("󰨸")).on_hover_text("copy text to clipboard").clicked(){
                    clipboard.set_text(&my_text);
                }; 
                ui.style_mut().override_text_style = None;
            });
            ui.with_layout(Layout::right_to_left(egui::Align::RIGHT), |ui|{
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                ui.centered_and_justified(|ui|{
                    let clean_button= ui.add_enabled(*text_is_dirty, egui::Button::new("clean 󰃢")).on_disabled_hover_text("replace text to internal representation of your numbers").on_hover_text("replace text to internal representation of numbers");
                    if clean_button.clicked(){
                        *text_is_dirty = false;
                        *my_text = parsed_values.vals.iter().map(|x|{
                            x.converted_value.to_string()
                        }).collect::<Vec<_>>().join(", ");
                        log::info!("cleaned text");
                    };
                });
                ui.style_mut().override_text_style = None;
            });
        });





        let (parse_warning_color, parse_warning_string) = get_parse_warning_color(&worse_parse_problem);

        let collapse_response = egui::CollapsingHeader::new(egui::RichText::new(parse_warning_string).color(parse_warning_color))
            .default_open(true)
            .show(ui, |_ui| {
        });

        if !collapse_response.fully_closed(){
            // ui.add(egui::RichText::new("am I inside the collapsible?").color(parse_warning_color))
            if *worse_parse_problem == ParsedWarning::Ok{
                // just the parsed numbers
                if num_strings.requires_restring {
                    num_strings.requires_restring = false;
                    num_strings.val = parsed_values.vals[..parsed_values.end_index].iter().map(|x| x.converted_value.to_string()).collect::<Vec<_>>().join(", ");
                }

                ui.allocate_ui(vec2(ui.available_width(), 200.0), |ui|{
                    ui.push_id("scroll_parsed", |ui|{
                        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui|{
                            ui.add_enabled(false, 
                                egui::TextEdit::multiline(&mut num_strings.val).hint_text("parsed numbers here").desired_width(ui.available_width())
                            );
                        });
                    });
                });

                // ui.add_enabled(false, 
                //     egui::TextEdit::multiline(&mut num_strings.val).hint_text("parsed numbers here").desired_width(ui.available_width())
                // );
            }
            else{
                //
                if colored_strings.requires_restring {
                    colored_strings.requires_restring = false;
                    colored_strings.vals.clear();
                    parsed_values.vals[..parsed_values.end_index].iter().for_each(|x|{
                        let (color, _) = get_parse_warning_color(&x.parsed_warning);

                        // log::info!("attempt to get raw string for number: {}", x.converted_value);
                        let raw_string = &my_text[x.raw_string.start_index..x.raw_string.end_index];
                        colored_strings.vals.push(egui::RichText::new(raw_string).color(color));
                    });
                }

                ui.group(|ui|{
                    ui.horizontal(|ui|{
                        let mut index: usize = 0;
                        parsed_values.vals[..parsed_values.end_index].iter().for_each(|x|{
                            let (_, msg) = get_parse_warning_color(&x.parsed_warning);
                            let colored_widget = colored_strings.vals.get(index).unwrap();
                            let label = ui.label(colored_widget.clone());
                            if x.parsed_warning != ParsedWarning::Ok{
                                label.on_hover_text(format!("{}{}[{}]", msg, ". Converted to: ", x.converted_value));
                            }else{
                                label.on_hover_text(msg);
                            }
                            index += 1;
                        });
                    })
                });
            }
        };

        ui.allocate_ui(vec2(ui.available_width(), 200.0), |ui|{
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui|{
                let text_edit = egui::TextEdit::multiline(&mut *my_text).hint_text("numbers here").desired_width(ui.available_width());

                if ui
                    .add(text_edit).on_hover_text("supports positive and negative ints and floats with the following regex expression: r\"-?\\d+(?:\\.\\d+)?\"")
                    .changed()
                {
                    // TODO: maybe add fancy stuff like remembering which parts of the string are already
                    // parsed, and parsing only new stuff and deleting any removed stuff.
                    // Could carry over to spawning cubes where not all cubes are respawned: instead only
                    // new cubes are added?


                    *text_is_dirty = true;
                    num_strings.requires_restring = true;
                    colored_strings.requires_restring = true;

                    problem_values.map.values_mut().for_each(|vec|{
                        vec.clear();
                    });
                    *worse_parse_problem = ParsedWarning::Ok;

                    let mut index: usize = 0;
                    log::info!("--");
                    number_regex
                        .re
                        .find_iter(&my_text)
                        .for_each(|m|{
                            let s = m.as_str();
                            match s.parse::<f64>(){
                                Ok(n) =>{
                                    let mut parsed_warning = ParsedWarning::Ok;
                                    let mut converted_val = n;
                     
                                    if n.is_infinite(){
                                        // log::warn!("over-flowed number: {}", s);
                                        parsed_warning = ParsedWarning::PrecisionLoss;
                                        *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::PrecisionLoss);
                                        converted_val = f64::MAX;
                                    }
                                    else if detect_precision_loss(s, n){
                                        // log::warn!("precision loss on number: {}", s);
                                        parsed_warning = ParsedWarning::PrecisionLoss;
                                        *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::PrecisionLoss);
                                    }
                                    else if n.is_nan(){
                                        // log::error!("number is NaN: {}", s);
                                        parsed_warning = ParsedWarning::Error;
                                        *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::Error);
                                        converted_val = 0.0;
                                    }
                                    add_parsed_value(&mut parsed_values, converted_val, parsed_warning, index, m.start(), m.end());
                                    // log::info!("added raw string: {}", &my_text[parsed_values.vals.get(index).unwrap().raw_string.start_index..parsed_values.vals.get(index).unwrap().raw_string.end_index])
                                },
                                Err(_err) => {
                                    // log::error!("failed to parse: {} with err {}", s, err);
                                    *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::Error);
                                    add_parsed_value(&mut parsed_values, 0.0, ParsedWarning::Error, index, m.start(), m.end());
                                }
                            }
                            index += 1;
                        });

                    parsed_values.end_index = index;  // truncates to end_index when 'sort' is clicked.
                    // parsed_values.vals.truncate(index);
                }


            });
        });


    });

    if font_scale.is_changed() {
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
