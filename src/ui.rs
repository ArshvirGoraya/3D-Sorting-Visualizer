use crate::PROGRAM_TITLE;

use core::{f64, fmt};

use bevy::{platform::collections::HashMap, prelude::*};

use bevy_egui::{
    EguiContexts,
    egui::{self, Margin, Spacing, vec2},
};

use regex_lite::Regex;


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

#[derive(Default)]
pub struct StringInfo {
    start_index: usize,
    end_index: usize,
}

#[derive(Default)]
pub struct ParsedValue {
    raw_string: StringInfo,
    matched_string: StringInfo,
    converted_value: f64,
    parsed_warning: ParsedWarning,
    // final_position: int,
    // box_handle: int
}

#[derive(Resource)]
pub struct ProblemValues {
    // Stores indices in ParsedValues that have parse problems.
    map: HashMap<ParsedWarning, Vec<usize>>,
}

impl ProblemValues {
    pub fn new() -> Self {
        let mut map: HashMap<ParsedWarning, Vec<usize>> = HashMap::new();
        for key in [
            ParsedWarning::Ok,
            ParsedWarning::Error,
            ParsedWarning::PrecisionLoss,
        ] {
            map.insert(key, Vec::new());
        }
        Self { map }
    }
}

#[derive(Resource, Default)]
pub struct ParsedValues {
    vals: Vec<ParsedValue>,
    end_index: usize,
}

#[derive(Default)]
pub struct NumString {
    requires_restring: bool,
    val: String,
}

#[derive(Resource)]
pub struct CopyTimer {
    pub copy_timer: Timer,
}


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

#[allow(clippy::too_many_arguments)]
pub fn ui_system(
    mut contexts: EguiContexts,
    // image_ids: Res<ImageIds>,
    mut my_text: Local<String>,
    // mut my_parsed_text: Local<String>,
    number_regex: Res<NumberRegex>,
    mut clipboard: ResMut<bevy_egui::EguiClipboard>,
    mut font_scale: ResMut<FontScale>,
    mut font_added: Local<bool>,
    // mut parsed_numbers: Local<Vec<f64>>,
    mut text_is_dirty: Local<bool>,

    mut num_strings: Local<NumString>,

    mut parsed_values: ResMut<ParsedValues>,
    mut problem_values: ResMut<ProblemValues>,
    mut worse_parse_problem: Local<ParsedWarning>,

    time: Res<Time>,
    mut copy_timer: ResMut<CopyTimer>,

) -> Result {
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
        ui.style_mut().override_text_style = None;

        ui.horizontal(|ui|{
            ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
            ui.allocate_ui(first_button_size, |ui|{
                let copy_response = ui.add_sized(first_button_size, egui::Button::new("󰨸")).on_hover_text("Copy text to clipboard");
                if copy_response.clicked(){
                    clipboard.set_text(&my_text);
                    copy_timer.copy_timer.reset();
                }; 
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
            ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
            ui.centered_and_justified(|ui|{
                let clean_button= ui.add_enabled(*text_is_dirty, egui::Button::new("clean 󰃢")).on_disabled_hover_text("Replace text with internal representation of your numbers").on_hover_text("Replace text with internal representation of numbers");
                if clean_button.clicked(){
                    *text_is_dirty = false;
                    *my_text = parsed_values.vals.iter().map(|x|{
                        x.converted_value.to_string()
                    }).collect::<Vec<_>>().join(", ");
                };
            });
            ui.style_mut().override_text_style = None;
        });


        let (parse_warning_color, parse_warning_string) = get_parse_warning_color(&worse_parse_problem);

        let collapse_response = egui::CollapsingHeader::new(egui::RichText::new(parse_warning_string).color(parse_warning_color))
            .id_salt("scroll_parsed_collapsible")
            .default_open(true)
            .show(ui, |_ui| {
        });

        if !collapse_response.fully_closed(){
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

fn detect_precision_loss(original: &str, parsed: f64) -> bool {
    let num_string = parsed.to_string();
    let new_string = string_trim_zeros(original);
    log::info!("{original} turned to {new_string} == {parsed}");
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
        (
            (Name("symbol_font".into())),
            FontId::new(25.0 * scale, Proportional),
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

fn add_parsed_value(
    parsed_values: &mut ParsedValues,
    converted_value: f64,
    parsed_warning: ParsedWarning,
    index: usize,
    match_start: usize,
    match_end: usize,
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

        parsed_value.converted_value = converted_value;
        parsed_value.parsed_warning = parsed_warning;
        parsed_value.matched_string.start_index = match_start;
        parsed_value.matched_string.end_index = match_end;
    } else {
        log::info!("new Parsed value!");
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
        });
    }
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
