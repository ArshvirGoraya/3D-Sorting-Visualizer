use crate::{AudioControls, CameraControls, PROGRAM_TITLE, sorter};

use core::{f64, fmt};
use std::{ffi::OsStr, path::Path};

use bevy::{audio::{PlaybackMode, Volume}, platform::collections::HashMap, prelude::*, reflect::Enum};

use bevy_egui::{
    EguiContexts,
    egui::{self, Margin, Rangef, Spacing, vec2},
};

use bevy_panorbit_camera::PanOrbitCamera;
use regex_lite::Regex;

use core::time::Duration;

const CUBE_WIDTH : f32 = 1.0;

#[derive(Resource)]
pub struct Random {
    pub rng: fastrand::Rng,
}
impl Default for Random {
    fn default() -> Self {
        Self {
            rng: fastrand::Rng::new(),
        }
    }
}

#[derive(Resource)]
pub struct FontScale {
    scale: f32,
    scale_step: f32,
    max: f32,
    min: f32,
}
impl Default for FontScale {
    fn default() -> Self {
        Self {
            scale: 1.0,
            scale_step: 0.1,
            max: 10.0,
            min: 0.1,
        }
    }
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
pub enum ParsedWarning {
    #[default]
    Ok,
    Error,
    PrecisionLoss,
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

// Cube stuff. Maybe better if not in this file?
#[derive(Resource)]
pub struct CubeAssets {
    pub mesh: Mesh3d,
    pub materials: HashMap<ParsedWarning, MeshMaterial3d<StandardMaterial>>,
}

#[derive(Default)]
pub struct StringInfo {
    start_index: usize,
    end_index: usize,
}

#[derive(Resource)]
pub struct ParsedValue {
    raw_string: StringInfo,
    matched_string: StringInfo,
    converted_value: f64,
    parsed_warning: ParsedWarning,
    cube_handle: Entity,
    // final_position: int,
    // box_handle: int
}

#[derive(Resource, Default)]
pub struct ParsedValues {
    vals: Vec<ParsedValue>,
    end_index: usize, // marks the position of visible and invisible cubes
}

#[derive(Default)]
pub struct NumString {
    // requires_restring: bool,
    cleaned_string: bool,
    val: String,
}

#[derive(Resource)]
pub struct CopyTimer {
    pub copy_timer: Timer,
}

#[allow(dead_code)]
fn tests() {
    // test_detect_precision_loss
    let regex_matches = [
        // positives:
        "1.1",
        ".1",
        "1",
        "0.0",
        ".0",
        "0",
        // negatives:
        "-1.1",
        "-.1",
        "-1",
        "-0.0",
        "-.0",
        "-0",
        // extras:
        "0000001.1",
        "0000001",
        "000000100000000",
        "1.0000000000000000",
        // precision loss: this should NOT match
        "1.00000000000000000000001",
    ];
    regex_matches.iter().for_each(|original| {
        detect_precision_loss(original, original.parse::<f64>().unwrap());
    });
}

pub fn generate_random_string_nums(amount: usize, min: f64, max: f64, text: &mut String, random: &mut ResMut<Random>){
    text.clear();
    // let range_helper = (max - min) + min;
    let range_helper = max - min;
    log::info!("range_helper: {range_helper}");

    for index in 0..amount{
        // text.push_str((format!("{:.2}", random.rng.f64() * range_helper)).trim_end_matches("0").trim_end_matches("."));
        text.push_str((format!("{:.2}", min + random.rng.f64() * range_helper )).trim_end_matches("0").trim_end_matches("."));
        if index != amount -1 {
            text.push_str(", ");
        }
    };
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_random_parsed_values(
    mut commands: Commands,
    // meshes: ResMut<Assets<Mesh>>,
    // materials: ResMut<Assets<StandardMaterial>>,
    mut random: ResMut<Random>,
    mut user_text: ResMut<UserText>,
    worse_parse_problem: Local<ParsedWarning>, // no parse warning when spawning cubes in first, so
                                               // this doesn't need to turn into a ResMut!
                                               // If you are making precision loss numbers on
                                               // purpose when spawning here, its fine if the system detects it as no
                                               // parse warning on the first spawn.
    number_regex: Res<NumberRegex>,
    cube_assets: Res<CubeAssets>,
    // mut update_list: ResMut<UpdateList>,
    // update_cubes_event: MessageWriter<UpdateCubes>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    mut camera_query: Query<&mut PanOrbitCamera>
) {
    // generate_random_string_nums(5, -100.0, 100.0, &mut user_text.val, &mut random);

    // Just doing this for now: uncomment the above in release!
    user_text.val = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 1.0000000000000001".to_string();

    update_parsed_values(number_regex, user_text, worse_parse_problem, &mut commands, & cube_assets, &mut parsed_values, &mut cubes_query, &mut camera_query);
}

#[allow(clippy::too_many_arguments)]
fn update_parsed_values(
    number_regex: Res<NumberRegex>,
    user_text: ResMut<UserText>,
    mut worse_parse_problem: Local<ParsedWarning>,
    commands: &mut Commands,
    cube_assets: &Res<CubeAssets>,
    parsed_values: &mut ParsedValues,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    camera_query: &mut Query<&mut PanOrbitCamera>
    ){
    *worse_parse_problem = ParsedWarning::Ok;
    let mut index: usize = 0;
    let mut any_requires_change_material_height = (false, false);
    log::info!("--");
    number_regex
        .re
        .find_iter(&user_text.val)
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
                    add_parsed_value(parsed_values, converted_val, parsed_warning, index, m.start(), m.end(), commands, cube_assets, cubes_query, &mut any_requires_change_material_height);
                },
                Err(_err) => {
                    // log::error!("failed to parse: {} with err {}", s, err);
                    *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::Error);
                    add_parsed_value(parsed_values, 0.0, ParsedWarning::Error, index, m.start(), m.end(), commands, cube_assets, cubes_query, &mut any_requires_change_material_height);
                }
            }
            index += 1;
        });
    let visible_cube_count_changed = parsed_values.end_index != index;
    parsed_values.end_index = index;
 
    // parsed_values.vals.len() = total number of blocks.
    // parsed_values.end_index = index of block which must be invisible (everything after this must
    // also be invisible)
    // check if the block is contained within the total number of blocks.
    //
    log::info!("{} < {}: {}", parsed_values.end_index, parsed_values.vals.len(), parsed_values.end_index < parsed_values.vals.len());
    //
    if parsed_values.end_index < parsed_values.vals.len() {
        log::info!("making cubes >= index {} invisible", parsed_values.end_index);
        for parsed_value in &parsed_values.vals[parsed_values.end_index..]{
            if let Ok((_, _, mut visibility)) = cubes_query.get_mut(parsed_value.cube_handle){
                if *visibility == Visibility::Hidden{
                    // break out when first cube that is hidden is found: we know that the rest are
                    // all hidden from the first encountered hidden.
                    // Also need to be sure that not just changing it to the same value or Bevy
                    // will do extra computations that is better avoided.
                    break;
                }
                *visibility = Visibility::Hidden;
                log::info!("make cube {} invisible", parsed_value.converted_value);
            }
        }
    }
    // Update the camera if number of visible cubes changed 
    if visible_cube_count_changed{
        let mut pan_orbit = camera_query.single_mut().unwrap();
        let cubes_middle = ((parsed_values.end_index as f32) * CUBE_WIDTH) / 2.0;
        // pan_orbit.focus.x = cubes_middle; // this sets what the camera is at initially. first set of numbers will NOT be centered. 
            // Cant set focus and target_focus each time... just makes it stay still until egui is
            // unfocused... weird
        pan_orbit.target_focus.x = cubes_middle;
        log::info!("set camera to middle of cubes: {}", cubes_middle)
    }
}

#[allow(clippy::too_many_arguments)]
fn add_parsed_value(
    parsed_values: &mut ParsedValues,
    converted_value: f64,
    parsed_warning: ParsedWarning,
    index: usize,
    match_start: usize,
    match_end: usize,
    commands: &mut Commands,
    cube_assets: & Res<CubeAssets>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    any_requires_change_material_height: &mut (bool, bool)
) {
    let previous_raw_string_end_index = {
        if index == 0 {
            0
        } else {
            parsed_values
                .vals
                .get(index - 1)
                .unwrap()
                .raw_string
                .end_index
        }
    };

    if let Some(parsed_value) = parsed_values.vals.get_mut(index) {
        // If vector already contains a parsed_value object at this index, just change its values:
        parsed_value.raw_string.start_index = previous_raw_string_end_index;
        parsed_value.raw_string.end_index = match_end;
        parsed_value.matched_string.start_index = match_start;
        parsed_value.matched_string.end_index = match_end;

        if parsed_value.parsed_warning != parsed_warning{
            any_requires_change_material_height.0 = true;
        }
        if parsed_value.converted_value != converted_value{
            any_requires_change_material_height.1 = true;
        }

        if let Ok((mut transform, mut material, mut visibility)) = cubes_query.get_mut(parsed_value.cube_handle)
        {
            if parsed_value.converted_value != converted_value{
                // change height to reflect new value - relative value is done later on after all
                // values are acquired.
                transform.scale.y = converted_value as f32;
                transform.translation.y = (converted_value / 2.0) as f32;
            }
            if parsed_value.parsed_warning != parsed_warning{
                *material = cube_assets.materials.get(&parsed_warning).unwrap().clone();
            }
            if *visibility != Visibility::Visible{
                // Need to make sure to check if not already visible first. Changing it to the same
                // value makes Bevy do extra things, which is computation cost easily avoided. 
                *visibility = Visibility::Visible;
            }
        }
        parsed_value.parsed_warning = parsed_warning;
        parsed_value.converted_value = converted_value;
        log::info!("updating cube/value at index {}", index);
    } else {
        parsed_values.vals.push(ParsedValue {
            converted_value,
            parsed_warning,
            matched_string: StringInfo {
                start_index: match_start,
                end_index: match_end,
            },
            raw_string: StringInfo {
                start_index: previous_raw_string_end_index,
                end_index: match_end,
            },
            cube_handle: spawn_a_cube(commands, cube_assets, index, parsed_warning, converted_value),
            // ..Default::default()
        });
        log::info!("creating new cube/value at index {}", index);
    }
}

fn spawn_a_cube(commands: &mut Commands, cube_assets: & Res<CubeAssets>, index: usize, parsed_warning: ParsedWarning, converted_value: f64) -> Entity{
    let mut position = Vec3::ZERO;
    position.x += CUBE_WIDTH * (index as f32); 
    position.y = (converted_value / 2.0) as f32;
    let mut size = Vec3::ONE;
    size.y = converted_value as f32;

    commands.spawn((
            cube_assets.mesh.clone(), 
            cube_assets.materials.get(&parsed_warning).unwrap().clone(), 
            Transform::from_translation(position).with_scale(size)
            )).id()
}

#[derive(Resource, Default)]
pub struct UserText{
    val: String,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_system(
    (mut commands, mut contexts): (Commands, EguiContexts),
    // mut commands: Commands,
    // mut contexts: EguiContexts,

    // text:
    (mut user_text, 
     number_regex, 
     mut clipboard, 
     mut font_scale, 
     mut font_added, 
     mut text_is_dirty): 
    (ResMut<UserText>, 
     Res<NumberRegex>, 
     ResMut<bevy_egui::EguiClipboard>, 
     ResMut<FontScale>, 
     Local<bool>, 
     Local<bool>),

    mut empty_text: Local<String>,

    // text parsing
    mut num_strings: Local<NumString>,
    mut parsed_values: ResMut<ParsedValues>,
    worse_parse_problem: Local<ParsedWarning>,

    // timers
    (time, mut copy_timer, mut increment_timer): (Res<Time>, ResMut<CopyTimer>, ResMut<sorter::IncrementTimer>),

    // cubes:
    cube_assets: Res<CubeAssets>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,

    // camera:
    (mut camera_query, mut camera_select): (Query<&mut PanOrbitCamera>, Local<crate::CameraControls>),


    // music_controller: Query<&AudioSink, With<DefaultAudio>>,

    // audio:
    (mut audio_controls, 
     mut audio_assets,
     audio_receiver_listening_get,
     audio_receiver_listening_set
     ): (
     ResMut<AudioControls>,
     ResMut<Assets<AudioSource>>,
     Option<ResMut<State<crate::WasmAudioReceiverListening>>>,
     Option<ResMut<NextState<crate::WasmAudioReceiverListening>>>,
    ),
    // random:
    mut random: ResMut<Random>,
    mut rng_values_controls: Local<crate::RNGValuesControls>,
    mut generated_rng_values: Local<bool>,

    // sort state
    (mut sort_state_set, sort_state_get, mut sort_select): (ResMut<NextState<sorter::SortState>>, Res<State<sorter::SortState>>, Local<sorter::Algorithms>),
    mut is_sorting: Local<bool>,

    // for random materials, need to create with this:
    // mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    // if *sort_state == sorter::SortState::NotSorting{}

    let ctx = contexts.ctx_mut()?;

    if !*font_added{
        *font_added = true;
        setup_font(ctx);
        let scale = font_scale.scale;
        ctx.all_styles_mut(move |style| {
            scale_ui(style, scale);
        });
    }

    let max_width = ctx.content_rect().width() * 0.37;

    let mut first_button_size: egui::Vec2 = Default::default();

    egui::Window::new(PROGRAM_TITLE).max_width(max_width).show(ctx, |ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
        ui.columns(3, |cols|{
            let response = cols[0].vertical_centered_justified(|ui|{
                if ui.button("").on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("https://github.com/ArshvirGoraya/3D-Sorting-Visualizer").clicked(){
                    ui.ctx().open_url(egui::OpenUrl {
                        new_tab: true,
                        url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
                    });
                }
            });
            cols[1].vertical_centered_justified(|ui|{
                if ui.button("").on_hover_cursor(egui::CursorIcon::ZoomOut).on_hover_text("Decrease Font").clicked(){
                    decrease_font(&mut font_scale);
                }

            });
            cols[2].vertical_centered_justified(|ui|{
                if ui.button("").on_hover_cursor(egui::CursorIcon::ZoomIn).on_hover_text("Increase Font").clicked(){
                    increase_font(&mut font_scale);
                }
            });
            first_button_size = response.response.rect.size();
        });
        // ui.style_mut().override_text_style = None;
        //
        ui.separator();


        ui.columns(2, |cols|{
            cols[0].vertical_centered_justified(|ui|{
                // TODO: if already sorting, change this to stop!
                if !*is_sorting{
                    if ui.add(
                        egui::Button::new("Sort!").fill(egui::Color32::from_rgb(48, 64, 43))
                        )
                        .on_hover_text("click to begin sorting")
                        .clicked()
                    {
                            log::info!("Begin sort!");
                            *is_sorting = !*is_sorting;
                    }
                }else{
                    #[allow(clippy::collapsible_else_if)]
                    if ui.add(
                        egui::Button::new("Stop!").fill(egui::Color32::from_rgb(83, 47, 52))
                        )
                        .on_hover_text("click to stop sorting")
                        .clicked()
                    {
                            log::info!("Stop sort!");
                            *is_sorting = !*is_sorting;
                    }
                }
            });
            // cols[1].vertical_centered_justified(|ui|{
            cols[1].add_enabled_ui(true, |ui|{
                // INFO: Must wrap around a rect for tooltip...
                // But if wrapped in a exact size, UI will not update vertically when combox is
                // wrapped, but can use truncate wrapping to not worry about this.
                let hover_size = egui::vec2(
                    ui.available_width(), 
                    ui.spacing().interact_size.y,
                );
                let (rect, _) = ui.allocate_exact_size(hover_size, egui::Sense::hover());
                // let mut child = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
                let mut child = ui.new_child(egui::UiBuilder{
                    max_rect: Some(rect),
                    // layout: Some(egui::Layout::left_to_right(egui::Align::Center)),
                    ..Default::default()
                });

                child.add_enabled_ui(!*is_sorting, |ui|{
                    egui::ComboBox::from_id_salt("sort_select")
                        .width(ui.available_width())
                        .selected_text(sort_select.to_string())
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        // .wrap()
                        .show_ui(ui, |ui|{
                            for algorithm in sorter::Algorithms::ALL{
                                ui.selectable_value(&mut *sort_select, algorithm, algorithm.to_string());
                                // TODO: .clicked() here will tell you which value has been
                                // selected!
                            }
                        });
                });
                child.interact(rect, "sort_select_hover".into(), egui::Sense::hover())
                    .on_hover_text("select sorting algorithm");
            });
        });
        ui.style_mut().override_text_style = None;

        ui.horizontal(|ui|{
            ui.label("Sort speed: ");
            ui.spacing_mut().slider_width = ui.available_width() - ui.spacing().interact_size.x - ui.spacing().item_spacing.x - 1.0;
            if ui.add(
                    egui::Slider::new(&mut increment_timer.duration_f64, 0.0..=1.0)
                    .step_by(0.01)
                    .max_decimals(2)
                    .clamping(egui::SliderClamping::Never)
                )
                .on_hover_text("seconds waited between each increment when sorting")
                .changed(){
                    increment_timer.duration_f64 = increment_timer.duration_f64.max(0.0);
                    increment_timer.increment_timer.reset();
                    log::info!("increment speed changed {}", increment_timer.duration_f64);
            }
        });

        ui.vertical_centered_justified(|ui|{
            if ui.button("Increment (debug)")
                .on_hover_text("debug: increment the sort by 1 step")
                .clicked() {
                sorter::increment_sorting();
            }
        });
        ui.separator();

        //
        // CAMERA
        // 

        ui.style_mut().override_text_style = Some(egui::TextStyle::Name("medium".into()));
        ui.columns(2, |cols|{
            cols[0].vertical_centered_justified(|ui|{
                if ui.button("Reset Camera")
                    .on_hover_text("reset the camera to its original position")
                        .clicked()
                {
                    log::info!("Reset the camera!");
                }
            });

            cols[1].scope(|ui|{
                let hover_size = egui::vec2(
                    ui.available_width(), 
                    ui.spacing().interact_size.y,
                );
                let (rect, _) = ui.allocate_exact_size(hover_size, egui::Sense::hover());
                // let mut child = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);

                let mut child = ui.new_child(egui::UiBuilder{
                    max_rect: Some(rect),
                    // layout: Some(egui::Layout::left_to_right(egui::Align::Center)),
                    ..Default::default()
                });

                // wrapped in a add_enabled_ui because .add() required a response and ComboBox
                // doesnt give one.
                child.add_enabled_ui(true, |ui|{
                    egui::ComboBox::from_id_salt("camera_select")
                        .width(ui.available_width())
                        .selected_text(camera_select.to_string())
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .show_ui(ui, |ui|{
                            for control in crate::CameraControls::ALL{
                                ui.selectable_value(&mut *camera_select, control, control.to_string());
                            }
                        });
                });
                child.interact(rect, "camera_select_hover".into(), egui::Sense::hover())
                    .on_hover_text("select camera control");
            });
        });
        ui.separator();

        // 
        // AUDIO
        //
        ui.horizontal(|ui|{
            ui.checkbox(&mut audio_controls.enabled, "Audio");
            // let collapse_response = egui::CollapsingHeader::new(egui::RichText::new(parse_warning_string).color(parse_warning_color))
            //     .id_salt("scroll_parsed_collapsible")
            //     .default_open(false)
            //     .show(ui, |_ui| {
            // });
            //
            // if !collapse_response.fully_closed(){
            ui.vertical_centered_justified(|ui|{
                ui.collapsing("Audio Settings", |ui|{
                    ui.add(
                        egui::Slider::new(&mut audio_controls.volume, 0.1..=10.0).text("Volume")
                            .max_decimals(1)
                            .step_by(0.1)
                    );
                    ui.add(
                        egui::Slider::new(&mut audio_controls.pitch, 0.1..=2.0).text("Pitch")
                            .max_decimals(1)
                            .step_by(0.1)
                    );
                    ui.columns(2, |cols|{
                        cols[0].add_enabled_ui(true, |ui|{
                            ui.vertical_centered_justified(|ui|{
                                #[cfg(not(target_arch = "wasm32"))]
                                if ui.button("Select Audio").clicked(){
                                    #[allow(clippy::collapsible_if)]
                                    if let Some(path) = rfd::FileDialog::new().add_filter("audio", &["aac", "flac", "wav", "ogg", "mp3"]).pick_file(){
                                        if let Ok(bytes) = std::fs::read(&path){
                                            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                                            crate::change_audio_source(&mut audio_controls, &mut audio_assets, file_name, bytes);
                                        }
                                    }
                                }

                                // just adding rust_analyzer to cfg so code doesn't appear
                                // disabled in IDE.
                                #[cfg(any(target_arch = "wasm32", rust_analyzer))]
                                {
                                    use web_sys::{HtmlInputElement, wasm_bindgen::JsCast};
                                    let audio_reciever_state = audio_receiver_listening_get.expect("audio receiver state should exist");
                                    // never disable this: previously, was disabled when audio
                                    // receiver was listening for a file selection.
                                    // There is no way to stop the receiver if cancel is selected
                                    // in the file dialog (as there is not reliable way to detect 
                                    // file dialog is cancelled). So, if cancelled, this would just stay
                                    // disabled. Which we don't want. No bad consequences to leaving
                                    // this enabled while receiving files (only bad thing is
                                    // recevier is still running even tho file selection is
                                    // cancelled, but that's not that expensive, and receiver will
                                    // stop once a file is ever selected).
                                    // TODO: could stop the receiver if any other kind of input is
                                    // detected (camera input, button click, font scale, anything)
                                    // ui.add_enabled_ui(*audio_reciever_state == crate::WasmAudioReceiverListening::NotListening, |ui|{
                                    ui.add_enabled_ui(true, |ui|{
                                        if ui.button("Select Audio").clicked(){
                                            // audio_controls
                                            let input_element = web_sys::window()
                                                .expect("window should exist")
                                                .document()
                                                .expect("document should exist")
                                                .get_element_by_id("audio_picker")
                                                .expect("audio_picker input should exist in index.html")
                                                .dyn_into::<HtmlInputElement>()
                                                .expect("audio_picker id must be on a input element");

                                            input_element.click();
                                            // make receiver listen for selected file
                                            audio_receiver_listening_set.expect("audio receiver state should exist")
                                                .set(crate::WasmAudioReceiverListening::Listening);
                                        }
                                    });
                                }
                            });
                        });
                        cols[1].vertical_centered_justified(|ui|{
                            if ui.button("Default").clicked(){
                                // set to default
                                if let Some(audio_handle) = &audio_controls.audio_source_handle{
                                    audio_assets.remove(audio_handle);
                                }
                                audio_controls.selected_file_name = None;
                                audio_controls.audio_source_handle = None;
                            }
                        })
                    });
                    ui.vertical_centered_justified(|ui|{
                        let mut file_name = &audio_controls.default_file_name;
                        if let Some(selected_file_name) = &audio_controls.selected_file_name{
                            file_name = selected_file_name;
                        }
                        ui.add_enabled(false,
                            egui::Button::new(file_name)
                            .wrap_mode(egui::TextWrapMode::Truncate)
                        );
                    });
                    ui.vertical_centered_justified(|ui|{
                        if ui.button("debug play selected").clicked(){
                            crate::play_audio(&mut commands, &mut audio_controls);
                        }
                    })
                });
            });
        });

        ui.separator();


        //
        // RNG values
        // 


        ui.horizontal(|ui|{
            if ui.button("RNG #").clicked(){
                generate_random_string_nums(rng_values_controls.amount, rng_values_controls.min, rng_values_controls.max, &mut user_text.val, &mut random);
                *generated_rng_values = true;
            }
            ui.vertical_centered_justified(|ui|{
                ui.collapsing("RNG Settings", |ui|{
                    if ui.add(
                        egui::Slider::new(&mut rng_values_controls.amount, 2..=100)
                        .clamping(egui::SliderClamping::Never)
                        .text("amount")
                    ).changed(){
                        // must at least be 2
                        rng_values_controls.amount = rng_values_controls.amount.max(2);
                    }
                    if ui.add(
                        egui::Slider::new(&mut rng_values_controls.min, -100.0..=100.0)
                        .clamping(egui::SliderClamping::Never)
                        .min_decimals(1)
                        .text("min")
                    ).changed(){
                        // set max to be the same as this, if this is bigger than max.
                        rng_values_controls.max = rng_values_controls.max.max(rng_values_controls.min);
                    }
                    if ui.add(
                        egui::Slider::new(&mut rng_values_controls.max, -100.0..=100.0)
                        .clamping(egui::SliderClamping::Never)
                        .min_decimals(1)
                        .text("max")
                    ).changed(){
                        // set min to be the same as this, if this is smaller than min.
                        rng_values_controls.min = rng_values_controls.min.min(rng_values_controls.max);
                    }
                })
            })
        });


        // randomize colors
        // toggle cubes height mode

        ui.separator();
        //

        ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
        ui.columns(3, |cols|{
            // clipboard:
            cols[0].vertical_centered_justified(|ui|{
                let copy_response = ui.button("󰨸").on_hover_text("Copy text to clipboard");
                if copy_response.clicked(){
                    clipboard.set_text(&user_text.val);
                    copy_timer.copy_timer.reset();
                }
                if !copy_timer.copy_timer.is_finished(){
                    copy_timer.copy_timer.tick(time.delta());
                    // copy_response.show_tooltip_text("Copied!");

                    // Center the tool tip:
                    // Measure it first using a galley
                    let galley = ui.painter().layout_no_wrap("Copied!".to_owned(), 
                        ui.style().text_styles[&egui::TextStyle::Body].clone(), 
                        ui.style().visuals.text_color()
                    );
                    // Center with the measurement
                    let mut position = copy_response.rect.center_bottom();
                    position.x -= galley.rect.size().x / 2.0;
                    // Create in an area:
                    egui::Area::new("copied_tooltip".into())
                        .order(egui::Order::Tooltip)
                        .fixed_pos(position)
                        .show(ui.ctx(), |ui|{
                            egui::Frame::popup(ui.style()).show(ui, |ui|{
                                ui.add(egui::Label::new("Copied!").wrap_mode(egui::TextWrapMode::Extend));
                            });
                        });
                }
            });
            // clean text
            cols[1].vertical_centered_justified(|ui|{
                let clean_button = ui.add_enabled(*text_is_dirty, egui::Button::new("Clean 󰃢"))
                    .on_hover_text("Replace text with internal representation of your numbers")
                    .on_disabled_hover_text("Replace text with internal representation of your numbers");
                if clean_button.clicked(){
                    *text_is_dirty = false;
                    user_text.val = parsed_values.vals[..parsed_values.end_index].iter().map(|x|{
                        x.converted_value.to_string()
                    }).collect::<Vec<_>>().join(", ");
                }
            });
            // cols[2].vertical_centered_justified(|ui|{
            //     if ui.add_enabled(true, egui::Button::new("RNG #"))
            //         .on_hover_text("Replace text with random numbers")
            //         .on_disabled_hover_text("Replace text with random numbers")
            //         .clicked(){
            //             // generate_random_string_nums();
            //     }
            // });
        });
        ui.style_mut().override_text_style = None;

        let (parse_warning_color, parse_warning_string) = get_parse_warning_color(&worse_parse_problem);

        let collapse_response = egui::CollapsingHeader::new(egui::RichText::new(parse_warning_string).color(parse_warning_color))
            .id_salt("scroll_parsed_collapsible")
            .default_open(false)
            .show(ui, |_ui| {
        });

        if !collapse_response.fully_closed(){
            if !num_strings.cleaned_string{
                num_strings.cleaned_string = true;
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
        };

        ui.allocate_ui(vec2(ui.available_width(), 200.0), |ui|{
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui|{
                let mut text_edit_widget = ui.add(egui::TextEdit::multiline(&mut user_text.val)
                    .hint_text("numbers here")
                    .desired_width(ui.available_width()))
                    .on_hover_text("supports positive and negative ints and floats with the following regex expression: r\"-?\\d+(?:\\.\\d+)?\"");
                if *generated_rng_values{
                    text_edit_widget.mark_changed();
                }
                if text_edit_widget.changed(){
                    log::info!("text widget changed");
                    if !*generated_rng_values{
                        // if rng values were generated, already clean.
                        *text_is_dirty = true;
                    }

                    // TODO: maybe add fancy stuff like remembering which parts of the string are already
                    // parsed, and parsing only new stuff and deleting any removed stuff.
                    // Could carry over to spawning cubes where not all cubes are respawned: instead only
                    // new cubes are added?
                    num_strings.cleaned_string = false;
                    update_parsed_values(
                        number_regex,
                        user_text,
                        worse_parse_problem,
                        &mut commands,
                        & cube_assets,
                        &mut parsed_values,
                        &mut cubes_query,
                        &mut camera_query,
                    );
                }
                *generated_rng_values = false;
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

fn detect_precision_loss(original: &str, parsed: f64) -> bool {
    let num_string = parsed.to_string();
    let new_string = string_trim_zeros(original);
    // log::info!("{original} turned to {new_string} == {parsed}");
    if new_string != num_string {
        log::info!("\tno match!")
    }
    new_string != num_string
}

fn string_trim_zeros(s: &str) -> String {
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
    if int.starts_with("-") {
        requires_minus = true;
        int = &int[1..int.len()];
    }
    // remove leading zeros from int slice.
    int = int.trim_start_matches("0");
    // check if a zero is required:
    if int.is_empty() {
        requires_zero = true;
    }

    // recombine int and frac. Add minus and zero as required.
    let requirements = (requires_minus, requires_zero);
    match requirements {
        (true, true) => format!("-0{}{}", int, frac),
        (false, true) => format!("0{}{}", int, frac),
        (true, false) => format!("-{}{}", int, frac),
        (false, false) => format!("{}{}", int, frac),
    }
}

pub fn increase_font(font_scale: &mut FontScale) {
    font_scale.scale = f32::min(font_scale.max, font_scale.scale + font_scale.scale_step);
}
pub fn decrease_font(font_scale: &mut FontScale) {
    font_scale.scale = f32::max(font_scale.min, font_scale.scale - font_scale.scale_step);
}

fn scale_ui(style: &mut egui::Style, scale: f32) {
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
        // ((Name("symbol_font".into())), FontId::new(14.0 * scale, egui::FontFamily::Name("symbol_font".into())))
        (
            (Name("symbol_font".into())),
            FontId::new(25.0 * scale, Proportional),
        ),
        (
            (Name("medium".into())),
            FontId::new(20.0 * scale, Proportional),
        ),
    ]
    .into();

    style.spacing = Spacing {
        item_spacing: vec2(8.0, 3.0) * scale,
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
        //
        // window_margin: Margin::same(6 * scale as i8),
        // default_area_size: vec2(600.0, 400.0) * scale,
        // menu_width: 400.0 * scale,
        // menu_spacing: 2.0 * scale,
        // combo_height: 200.0 * scale,
        ..Default::default()
    };
}

#[allow(clippy::collapsible_if)]
fn set_worse_parse_problem(
    worse_parse_problem: &ParsedWarning,
    parsed_problem: ParsedWarning,
) -> ParsedWarning {
    if parsed_problem == ParsedWarning::PrecisionLoss {
        if *worse_parse_problem != ParsedWarning::Error {
            return ParsedWarning::PrecisionLoss;
        }
    }
    ParsedWarning::Error
}

fn get_parse_warning_color(worse_parse_problem: &ParsedWarning) -> (egui::Color32, &'static str) {
    match *worse_parse_problem {
        ParsedWarning::Ok => (egui::Color32::LIGHT_GREEN, "No Parse Problems"),
        ParsedWarning::Error => (egui::Color32::LIGHT_RED, "Cannot Parse"),
        ParsedWarning::PrecisionLoss => (egui::Color32::ORANGE, "Precision Loss"),
    }
}

fn setup_font(ctx: &mut egui::Context) {
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
    fonts.font_data.insert(
        "symbol_font".to_owned(),
        egui::FontData::from_static(font_bytes).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("symbol_font".to_owned());
    ctx.set_fonts(fonts);
}
