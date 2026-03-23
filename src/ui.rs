use crate::{AudioControls, PROGRAM_TITLE, sorter};

use core::{f64, fmt};
use std::time::Duration;

use bevy::{platform::collections::HashMap, prelude::*};

use bevy_egui::{
    EguiContexts,
    egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Spacing, Stroke, style::ScrollStyle, vec2},
};

use bevy_panorbit_camera::PanOrbitCamera;
use regex_lite::Regex;

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
    desired_screen_percentage: f32,
}
impl Default for FontScale {
    fn default() -> Self {
        Self {
            scale: 1.0,
            scale_step: 0.1,
            max: 5.0,
            min: 0.1,
            // these get set every frame (relative to the screen size which may change during
            // update)
            desired_screen_percentage: 0.20,
        }
    }
}
impl FontScale{
    const BASE_WIDTH: f32 = 380.0; // the base width necessary at scale 1 (with all collapsible's expanded)
}

#[derive(Resource)]
pub struct NumberRegex {
    re: Regex,
}

impl Default for NumberRegex {
    fn default() -> Self {
        Self {
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

#[derive(Resource)]
pub struct CubeAssets {
    pub mesh: Mesh3d,
    pub materials: HashMap<ParsedWarning, MeshMaterial3d<StandardMaterial>>,
}

#[derive(Default, Clone)]
pub struct StringInfo {
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Resource, Clone)]
pub struct ParsedValue {
    pub raw_string: StringInfo,
    pub matched_string: StringInfo,
    pub converted_value: f64,
    pub parsed_warning: ParsedWarning,
    pub cube_handle: Entity,
    pub rng_color: MeshMaterial3d<StandardMaterial>,
    pub sorted_position: usize, 
}

#[derive(Resource, Default)]
pub struct ParsedValues {
    pub vals: Vec<ParsedValue>,
    pub end_index: usize, // marks the position of visible and invisible cubes
}

#[derive(Default)]
pub struct NumString {
    cleaned_string: bool,
    val: String,
}

#[derive(Resource)]
pub struct CopyTimer {
    pub copy_timer: Timer,
}

// #[allow(dead_code)]
// fn tests() {
//     // test_detect_precision_loss
//     let regex_matches = [
//         // positives:
//         "1.1",
//         ".1",
//         "1",
//         "0.0",
//         ".0",
//         "0",
//         // negatives:
//         "-1.1",
//         "-.1",
//         "-1",
//         "-0.0",
//         "-.0",
//         "-0",
//         // extras:
//         "0000001.1",
//         "0000001",
//         "000000100000000",
//         "1.0000000000000000",
//         // precision loss: this should NOT match
//         "1.00000000000000000000001",
//     ];
//     regex_matches.iter().for_each(|original| {
//         detect_precision_loss(original, original.parse::<f64>().unwrap());
//     });
// }

pub fn generate_random_string_nums(amount: usize, min: f64, max: f64, max_decimals: usize, text: &mut String, random: &mut ResMut<Random>){
    text.clear();
    let range_helper = max - min;

    for index in 0..amount{
        text.push_str((
                format!("{:.1$}", min + random.rng.f64() * range_helper, max_decimals))
            .trim_end_matches("0")
            .trim_end_matches("."));
        if index != amount -1 {
            text.push_str(", ");
        }
    };
}

pub fn spawn_random_parsed_values(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut random: ResMut<Random>,
    mut user_text: ResMut<UserText>,
    worse_parse_problem: Local<ParsedWarning>, // no parse warning when spawning cubes in first, so
                                               // this doesn't need to turn into a ResMut!
                                               // If you are making precision loss numbers on
                                               // purpose when spawning here, its fine if the system detects it as no
                                               // parse warning on the first spawn.
    number_regex: Res<NumberRegex>,
    cube_assets: Res<CubeAssets>,

    rng_values_controls: Res<crate::RNGValuesControls>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>,
    mut camera_query: Query<&mut PanOrbitCamera>,

    camera_controls_follow_selected: Local<bool>,

    mut cube_scale_controls: ResMut<crate::CubeScaleControls>,
    sort_state_get: Res<State<sorter::SortState>>,
    (
        scanned_cube, 
        mut clicked_cube
    ): 
    (
        ResMut<crate::ScannedCube>, 
        ResMut<crate::ClickedCube>
    ),
) {
    // TODO
    // generate_random_string_nums(rng_values_controls.amount, rng_values_controls.min, rng_values_controls.max, rng_values_controls.max_decimals, &mut user_text.val, &mut random);
    // user_text.val = "1, 34, 93, 78, 37, 8, 87, 89, 81, 39, 56, 92, 47, 44, 33, 44, 74, 2, 93, 4, 78, 44, 26, 34, 1, 3, 29, 97, 14, 15, 4, 73, 41, 38, 1, 39, 54, 33, 75, 31, 91, 32, 52, 68, 17, 73, 52, 36, 24, 4, 52, 86, 26, 36, 71, 11, 64, 86, 7, 48, 7, 39, 78, 93, 37, 84, 88, 87, 69, 23, 74, 18, 95, 65, 49, 24, 18, 2, 24, 36, 4, 44, 65, 42, 81, 3, 38, 76, 56, 68, 87, 84, 37, 87, 53, 44, 77, 74, 71, 86".to_string();
    //
    // Quick sort worst case: merge sort should be faster
    // user_text.val = "1, 1, 1, 2, 2, 3, 3, 4, 4, 4, 4, 7, 7, 8, 11, 14, 15, 17, 18, 18, 23, 24".to_string();
    // Quick sort should be faster than merge sort with more randomized output:
    user_text.val = "42, 7, 19, 3, 88, 14, 55, 27, 61, 9".to_string();

    // user_text.val = "25, 23, 9, 5".to_string();
    // user_text.val = "10, 3, 19, 7, 18, 4, 15, 5, 12, 1, 16, 2".to_string();
    // user_text.val = "19, 3, 16, 3, 1, 6, 2, 2, 17, 9, 1".to_string();
    // user_text.val = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 1.0000000000000001".to_string();
    update_parsed_values(
        number_regex, 
        &user_text, 
        worse_parse_problem, 
        &mut commands, 
        & cube_assets, 
        &mut parsed_values, 
        &mut cubes_query, 
        &mut camera_query,
        &mut random, 
        &mut materials, 
        rng_color_controls.rng_cubes_enabled,
        &mut cube_scale_controls,
        &scanned_cube,
        &camera_controls_follow_selected,
        &sort_state_get,
        &mut clicked_cube,
    );
}

fn update_parsed_values(
    number_regex: Res<NumberRegex>,
    user_text: &ResMut<UserText>,
    mut worse_parse_problem: Local<ParsedWarning>,
    commands: &mut Commands,
    cube_assets: &Res<CubeAssets>,
    parsed_values: &mut ParsedValues,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>,
    camera_query: &mut Query<&mut PanOrbitCamera>,
    random: &mut ResMut<Random>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng_color_controls_enabled: bool,
    cube_scale_controls: &mut ResMut<crate::CubeScaleControls>,
    scanned_cube: &ResMut<crate::ScannedCube>,
    camera_controls_follow_selected: &Local<bool>,
    sort_state_get: &Res<State<sorter::SortState>>,
    clicked_cube: &mut ResMut<crate::ClickedCube>,
    ){
    *worse_parse_problem = ParsedWarning::Ok;
    let mut index: usize = 0;
    // log::info!("--");
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
                        log::error!("number is NaN: {}", s);
                        parsed_warning = ParsedWarning::Error;
                        *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::Error);
                        converted_val = 0.0;
                    }
                    add_parsed_value(
                        parsed_values, 
                        converted_val, 
                        parsed_warning, 
                        index, 
                        m.start(), 
                        m.end(), 
                        commands, 
                        cube_assets, 
                        cubes_query, 
                        random, 
                        materials, 
                        rng_color_controls_enabled,
                        cube_scale_controls,
                    );
                },
                Err(err) => {
                    log::error!("failed to parse: {} with err {}", s, err);
                    *worse_parse_problem = set_worse_parse_problem(&worse_parse_problem, ParsedWarning::Error);
                    add_parsed_value(
                        parsed_values, 
                        0.0, 
                        ParsedWarning::Error, 
                        index, 
                        m.start(), 
                        m.end(), 
                        commands, 
                        cube_assets, 
                        cubes_query, 
                        random, 
                        materials, 
                        rng_color_controls_enabled,
                        cube_scale_controls,
                    );
                }
            }
            index += 1;
        });
    let width_changed = parsed_values.end_index != index;
    parsed_values.end_index = index;
 
    // DO NOT PUT THIS IN SECOND LOOP: this loops from end_index to end, not beginning to
    // end_index.
    // parsed_values.vals.len() = total number of blocks.
    // parsed_values.end_index = index of block which must be invisible (everything after this must
    // also be invisible). cube before this is the last visible one.
    // check if the block is contained within the total number of blocks.
    //
    // log::info!("{} < {}: {}", parsed_values.end_index, parsed_values.vals.len(), parsed_values.end_index < parsed_values.vals.len());
    //
    if parsed_values.end_index < parsed_values.vals.len() {
        // log::info!("making cubes >= index {} invisible", parsed_values.end_index);
        for parsed_value in &parsed_values.vals[parsed_values.end_index..]{
            if let Ok((_, _, mut visibility, _)) = cubes_query.get_mut(parsed_value.cube_handle){
                if *visibility == Visibility::Hidden{
                    // break out when first cube that is hidden is found: we know that the rest are
                    // all hidden from the first encountered hidden.
                    // Also need to be sure that not just changing it to the same value or Bevy
                    // will do extra computations that is better avoided.
                    break;
                }
                *visibility = Visibility::Hidden;
                // log::info!("make cube {} invisible", parsed_value.converted_value);
            }
        }
    }

    let cube_width = get_cube_size_from_width_scale(parsed_values.end_index, cube_scale_controls);

 
    // On text update: only re-center camera if:
        // clicked cube is NO LONGER within range.
        // if no clicked cube, then re-center if amount of cubes have changed (center point
        // of cubes have changed)
    if let Some(clicked_cube_idx) = clicked_cube.index{
        if clicked_cube_idx >= parsed_values.end_index{
            center_camera(
                cube_width, 
                parsed_values.end_index, 
                camera_query,
                scanned_cube,
                camera_controls_follow_selected,
                sort_state_get,
            );
            clicked_cube.index = None;
        }
    }else if width_changed {
        center_camera(
            cube_width, 
            parsed_values.end_index, 
            camera_query,
            scanned_cube,
            camera_controls_follow_selected,
            sort_state_get,
        );
    }
    





    // INFO: update sorted positions regardless of whether we want positional_heights or not.
    // Because: all sorting algorithms use sorted_position instead of converted_value to sort
    // because it converted_value of the SAME value has NO grantee of being put in the same 
    // position as the sorted_position dictates (this is because same sort algorithms are 
    // NOT stable (e.g., if two elements are equal their relative order is not preserved)). 
    // This leads to boxes which are visually taller/shorter being in the wrong order
    // If we sort by sorted_position, then we don't have to deal with this.
    // 
    // ALSO: for hovering over elements: i want to see their final position on hover data even if
    // positional_heights is not used.
    update_sorted_positions(parsed_values);

    // Update 
    update_parsed_values_second_loop(parsed_values, cube_scale_controls, cubes_query, commands, cube_width);
}

fn update_sorted_positions(parsed_values: &mut ParsedValues){
    // create vec of indices of size up to end_index.
    let mut sorted_indices: Vec<usize> = (0..parsed_values.end_index).collect();

    // sort by the values inside of these indices.
    sorted_indices.sort_by(
        |&i, &j| {
            parsed_values.vals[i].converted_value.partial_cmp(&parsed_values.vals[j].converted_value)
    }.unwrap());


    for (final_position, current_position) in sorted_indices.iter().enumerate(){
        parsed_values.vals[*current_position].sorted_position = final_position;
    }
}



fn update_parsed_values_second_loop(
    parsed_values: &mut ParsedValues,
    cube_scale_controls: & ResMut<crate::CubeScaleControls>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>,
    commands: &mut Commands,
    cube_width: f32,
){
    // Logic that can only occur AFTER regex loop.
    // If cube_scale_controls.width_scale_enable, set widths and horizontal positions of the cubes up to end_index
    // If cube_scale_controls.positional_heights, set the heights and vertical position of cubes up
    // to end_index.

    // If all are false, return. If any are true, continue.
    if !cube_scale_controls.width_scale_enable && !cube_scale_controls.positional_heights{
        return;
    }

    let end_index = parsed_values.end_index;

    for (index, parsed_value) in parsed_values.vals[..end_index].iter_mut().enumerate() {
        if let Ok((mut transform, _, _, _)) = cubes_query.get_mut(parsed_value.cube_handle) {

            if cube_scale_controls.width_scale_enable{
                set_width_and_horizontal_position(index, &mut transform, cube_width);
            }
            if cube_scale_controls.positional_heights{
                set_position_height_and_vertical_position(parsed_value.sorted_position, &mut transform, cube_scale_controls);
            }
        }else{
            // Cube has just spawned. Not available to query. Must overwrite transform instead.
            // Requires re-creating the entire transform (even parts untouched by
            // width_scale and positional_heights)
            let mut cube = commands.get_entity(parsed_value.cube_handle).unwrap();
            let mut transform = Transform::from_translation(Vec3::ZERO);

            if cube_scale_controls.positional_heights{
                set_position_height_and_vertical_position(parsed_value.sorted_position, &mut transform, cube_scale_controls);
                if !cube_scale_controls.width_scale_enable{
                    set_width_and_horizontal_position(index, &mut transform, cube_width);
                }
            }
            if cube_scale_controls.width_scale_enable{
                set_width_and_horizontal_position(index, &mut transform, cube_width);
                if !cube_scale_controls.positional_heights{
                    set_height_and_vertical_position(parsed_value.converted_value, &mut transform, cube_scale_controls);
                }
            }

            cube.insert(transform);
        }
    }
}

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
        &mut crate::CubeData,
    )>,
    random: &mut ResMut<Random>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rng_color_controls_enabled: bool,
    cube_scale_controls: &mut ResMut<crate::CubeScaleControls>
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

        if let Ok((mut transform, mut material, mut visibility, _)) = cubes_query.get_mut(parsed_value.cube_handle)
        {
            if parsed_value.converted_value != converted_value{
                // change height to reflect new value - positional value is done later on after all
                // values are acquired.
                // transform.scale.y = converted_value as f32;
                // transform.translation.y = (converted_value / 2.0) as f32;
                if !cube_scale_controls.positional_heights{
                    // don't call this if using positional_heights. Can still add it but Will be overwritten later anyway.
                    set_height_and_vertical_position(converted_value, &mut transform, cube_scale_controls)
                }
            }
            if parsed_value.parsed_warning != parsed_warning{
                *material = get_cube_material(rng_color_controls_enabled, parsed_warning, cube_assets, parsed_value.rng_color.clone());
            }
            if *visibility != Visibility::Visible{
                // Need to make sure to check if not already visible first. Changing it to the same
                // value makes Bevy do extra things, which is computation cost easily avoided. 
                *visibility = Visibility::Visible;
            }
        }
        parsed_value.parsed_warning = parsed_warning;
        parsed_value.converted_value = converted_value;
        // log::info!("updating cube/value at index {}", index);
    } else {
        let rng_color = crate::spawn_and_get_random_color_handle(materials, random);
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
            rng_color: rng_color.clone(),
            cube_handle: spawn_a_cube(
                commands, 
                cube_assets, 
                index, 
                parsed_warning, 
                converted_value, 
                rng_color_controls_enabled, 
                rng_color, 
                cube_scale_controls
            ),
            sorted_position: 0, // this gets changed later
        });
        // log::info!("creating new cube/value at index {}", index);
    }
}

fn spawn_a_cube(
    commands: &mut Commands, 
    cube_assets: & Res<CubeAssets>, 
    index: usize, 
    parsed_warning: ParsedWarning, 
    converted_value: f64, 
    rng_color_controls_enabled: bool, 
    rng_color: MeshMaterial3d<StandardMaterial>,
    cube_scale_controls: & ResMut<crate::CubeScaleControls>
) -> Entity{
    // INFO: about cube horizontal size and positioning:
    // Must just use the default horizontal size and position here, If
    // cube_scale_controls.width_scale must be used, it used after the regex loop after total cube
    // count is known.

    let mut transform = Transform::from_translation(Vec3::ZERO).with_scale(Vec3::ONE);
    transform.translation.x = transform.scale.x * (index as f32); 

    if !(cube_scale_controls.width_scale_enable || cube_scale_controls.positional_heights){
        // if height_scale_enable, the set_height_and_vertical_position function gets called anyway
        // later on in the second loop for cubes that have JUST SPAWNED (due to their transforms needing to be overwritten), 
        // so no need to call it here. But still can if you want, it will get over-written.
        // same with positional_heights being true
        set_height_and_vertical_position(converted_value, &mut transform, cube_scale_controls);
    }

    commands.spawn((
        cube_assets.mesh.clone(), 
        get_cube_material(rng_color_controls_enabled, parsed_warning, cube_assets, rng_color), 
        transform,
        Pickable::default(),
        crate::CubeData{
            index
        }
    )).observe(crate::detect_cube_clicked)
    .id()
}

fn set_width_and_horizontal_position(index: usize, transform: &mut Transform, cube_width: f32){
    transform.scale.x = cube_width; 
    transform.translation.x = transform.scale.x * (index as f32);
}

pub fn get_cube_size_from_width_scale(end_index : usize, cube_scale_controls: &ResMut<crate::CubeScaleControls>) -> f32{
    if cube_scale_controls.width_scale_enable{
        return (cube_scale_controls.width_scale / ((end_index-1) as f64)) as f32
    }
    CUBE_WIDTH
}

fn control_cube_widths(
    parsed_values: &mut ResMut<ParsedValues>,
    cube_scale_controls: & ResMut<crate::CubeScaleControls>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>,
    camera_query: &mut Query<&mut PanOrbitCamera>,
    scanned_cube: &ResMut<crate::ScannedCube>,
    camera_controls_follow_selected: &Local<bool>,
    sort_state_get: &Res<State<sorter::SortState>>,
){
    let end_index = parsed_values.end_index;
    
    let cube_width = get_cube_size_from_width_scale(parsed_values.end_index, cube_scale_controls);

    for (index, parsed_value) in parsed_values.vals[..end_index].iter_mut().enumerate() {
        if let Ok((mut transform, _, _, _)) = cubes_query.get_mut(parsed_value.cube_handle) {
            set_width_and_horizontal_position(index, &mut transform, cube_width);
        }
    }
    center_camera(
        cube_width, 
        end_index, 
        camera_query, 
        scanned_cube, 
        camera_controls_follow_selected, 
        sort_state_get,
    );
}

fn center_camera(
    cube_width: f32,
    end_index: usize,
    camera_query: &mut Query<&mut PanOrbitCamera>,
    scanned_cube: &ResMut<crate::ScannedCube>,
    camera_controls_follow_selected: &Local<bool>,
    sort_state_get: &Res<State<sorter::SortState>>,
){
    if 
        **camera_controls_follow_selected
        && let Some(scanned_cube_transform) = scanned_cube.transform
        && *sort_state_get.get() == sorter::SortState::Sorting
    {
        crate::center_camera_on_cube(&scanned_cube_transform, camera_query, true);
    }else{
        crate::center_camera_on_all_cubes(camera_query, cube_width, end_index);
    }

}

fn set_position_height_and_vertical_position(positional_value: usize, transform: &mut Transform, cube_scale_controls: & ResMut<crate::CubeScaleControls>){
    let mut height_value = (positional_value + 1) as f64;
    if cube_scale_controls.height_scale_enable{
        height_value *= cube_scale_controls.height_scale;
    }
    transform.scale.y = height_value as f32;
    transform.translation.y = 0 as f32;
}

fn set_height_and_vertical_position(converted_value: f64, transform: &mut Transform, cube_scale_controls: & ResMut<crate::CubeScaleControls>){
    let mut height_value = converted_value;
    if cube_scale_controls.height_scale_enable{
        height_value *= cube_scale_controls.height_scale;
    }
    transform.scale.y = height_value as f32;
    transform.translation.y = (height_value / 2.0) as f32
}

fn control_cube_heights(
    parsed_values: &mut ResMut<ParsedValues>,
    cube_scale_controls: & ResMut<crate::CubeScaleControls>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>
){
    let end_index = parsed_values.end_index;

    if cube_scale_controls.positional_heights{
        update_sorted_positions(parsed_values);
        for parsed_value in &mut parsed_values.vals[..end_index] {
            if let Ok((mut transform, _, _, _)) = cubes_query.get_mut(parsed_value.cube_handle) {
                set_position_height_and_vertical_position(parsed_value.sorted_position, &mut transform, cube_scale_controls);
            }
        }
    }
    else{
        for parsed_value in &mut parsed_values.vals[..end_index] {
            if let Ok((mut transform, _, _, _)) = cubes_query.get_mut(parsed_value.cube_handle) {
                set_height_and_vertical_position(parsed_value.converted_value, &mut transform, cube_scale_controls);
            }
        }
    }
}

pub fn get_cube_material(rng_color_controls_enabled: bool, parsed_warning: ParsedWarning, cube_assets: & Res<CubeAssets>, rng_color: MeshMaterial3d<StandardMaterial>) -> MeshMaterial3d<StandardMaterial> {
    // if using rng AND no parse warning, use RNG material. Else get the parse warning material.
    if rng_color_controls_enabled && parsed_warning == ParsedWarning::Ok{
        rng_color
    }else{
        cube_assets.materials.get(&parsed_warning).unwrap().clone()
    }
}

fn set_cube_colors(
    rng_color_controls_enabled: bool,
    generate_new_random: bool,
    parsed_values: &mut ResMut<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut crate::CubeData,
    )>,
    cube_assets: &Res<CubeAssets>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    random: &mut ResMut<Random>,
    sort_colored_cubes: &Option<Res<crate::SortColoredCubes>>,
) {

    // set to random or default depending on rng_color_controls_enabled
    let end_index = parsed_values.end_index;

    for i in 0..end_index {
        if let Some(sort_colored_cubes) = sort_colored_cubes &&
            sort_colored_cubes.cubes.contains(&i)
        {
            // if cube is colored by sort algorithm, skip it.
            continue;
        }

        let parsed_value = &mut parsed_values.vals[i];

        // TODO:
        // ignore any cubes that are colored by sorting algorithm

        // generate new RNG values?
        if generate_new_random {
            materials.remove(parsed_value.rng_color.clone()); // not needed. bevy will remove it
                                                              // once all handles of it are
                                                              // removed (when rng_color is set to
                                                              // something else). but no harm in
                                                              // being explicit?
            parsed_value.rng_color = crate::spawn_and_get_random_color_handle(materials, random);
        }

        let parsed_warning = parsed_value.parsed_warning;
        if let Ok((_, mut material, _, _)) = cubes_query.get_mut(parsed_value.cube_handle) {
            *material = get_cube_material(rng_color_controls_enabled, parsed_warning, cube_assets, parsed_value.rng_color.clone());
        }
    }
}

#[derive(Resource, Default)]
pub struct UserText{
    pub val: String,
}

pub fn ui_system(
    (mut commands, mut contexts, keyboard_input): (Commands, EguiContexts, Res<ButtonInput<KeyCode>>),
    mut materials: ResMut<Assets<StandardMaterial>>,

    // text:
    (mut user_text, 
     number_regex, 
     mut clipboard, 
     mut font_scale, 
     mut font_added, 
     mut text_is_dirty,
     mut text_just_cleaned,
     mut wasm_on_mobile, // INFO: used in wasm. DO NOT REMOVE 'mut'. impacts if text is editable
     ): 
    (ResMut<UserText>, 
     Res<NumberRegex>, 
     ResMut<bevy_egui::EguiClipboard>, 
     ResMut<FontScale>, 
     Local<bool>, 
     Local<bool>,
     Local<bool>,
     Local<bool>
     ),

    // text parsing
    mut num_strings: Local<NumString>,
    mut parsed_values: ResMut<ParsedValues>,
    worse_parse_problem: Local<ParsedWarning>,

    // timers
    (time, 
     mut copy_timer, 
     mut increment_timer,
     sorting_time,
     sorting_time_isolated,
    ): (
    Res<Time>,
    ResMut<CopyTimer>, 
    ResMut<sorter::IncrementTimer>,
    Res<crate::SortingTime>,
    Res<crate::SortingTimeIsolated>
    ),

    // cubes:
    (cube_assets,
     mut cubes_query,
     hovered_cube,
     mut clicked_cube,
     scanned_cube,
     sort_colored_cubes,
    ):
    (
        Res<CubeAssets>, 
        Query<(
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
            &mut crate::CubeData,
        )>,
        Res<crate::HoveredCube>,
        ResMut<crate::ClickedCube>,
        ResMut<crate::ScannedCube>,
        Option<Res<crate::SortColoredCubes>>
    ),

    // camera:
    (mut camera_query, 
     mut camera_controls_auto_rotate,
     mut camera_controls_follow_selected,
     mut camera_controls_auto_rotate_set,
     mut camera_controls_follow_set,
     ): (
     Query<&mut PanOrbitCamera>, 
     Local<bool>,
     Local<bool>,
     ResMut<NextState<crate::CameraControlsAutoRotate>>,
     ResMut<NextState<crate::CameraControlsFollow>>,
     ),

    // audio:
    (mut audio_controls, 
     mut audio_assets,
     audio_receiver_listening_set, // INFO: USED IN WASM BUILD DO NOT REMOVE
     mut stop_all_audio_event,
    ): 
        (
            ResMut<AudioControls>,
            ResMut<Assets<AudioSource>>,
            Option<ResMut<NextState<crate::WasmAudioReceiverListening>>>,
            MessageWriter<crate::StopAllAudio>,
        ),
    // random:
    (mut random,
     mut rng_values_controls, 
     mut generated_rng_values, 
     mut rng_color_controls, 
     mut bg_color,
    ): 
        (ResMut<Random>,
         ResMut<crate::RNGValuesControls>, 
         Local<bool>,
         ResMut<crate::RNGColorControls>,
         ResMut<ClearColor>,
        ),

    // cube scale
    mut cube_scale_controls: ResMut<crate::CubeScaleControls>,

    // sort state
    (mut sort_state_set, 
     sort_state_get, 
     mut sort_select_set,
     sort_select_get,
     //
     mut paused_state_set,
     paused_state_get
    ): (
        ResMut<NextState<sorter::SortState>>, 
        Res<State<sorter::SortState>>, 
        ResMut<NextState<sorter::Algorithms>>, 
        Res<State<sorter::Algorithms>>, 
        //
        ResMut<NextState<sorter::PausedState>>, 
        Res<State<sorter::PausedState>>, 
    ),
) -> Result {
    let is_sorting = *sort_state_get.get() == sorter::SortState::Sorting;
    let is_paused = *paused_state_get.get() == sorter::PausedState::Paused;

    let ctx = contexts.ctx_mut()?;

    if !*font_added{
        *font_added = true;
        // INFO: can't put this in a startup system as it requires EguiPrimaryContextPass.
        setup_font(ctx);

        // Can find out if in mobile on startup, but putting it here since its only necessary in
        // this system (for now).
        #[cfg(any(target_arch = "wasm32", rust_analyzer))]
        {
            if crate::is_mobile(){
                *wasm_on_mobile = true;
            }
        }
    }

    let scroll_height = ctx.content_rect().height() * 0.85;
 
    // Find how much base width must scale to fit the desired screen width.
    let screen_width = ctx.content_rect().width();
    let desired_width = screen_width * font_scale.desired_screen_percentage;
    let required_scaling = desired_width / FontScale::BASE_WIDTH;
    // Scale the width to fit desired percentage of the screen (in this case we scale the font
    // scale by the percentage and then scale the width by the font scale (to have both the
    // user-defined font scale and screen defined font scale into one + it scales the fonts by
    // screen size as well))
    let scale = font_scale.scale * required_scaling;
    let width = FontScale::BASE_WIDTH * scale;
    ctx.all_styles_mut(move |style| {
        scale_ui(style, scale);
    });

    let mut first_button_size: egui::Vec2 = Default::default();

    let window_area = egui::Window::new(PROGRAM_TITLE)
        .max_width(width)
        .min_width(width)
        .resizable(false)
        .default_pos([0.0,0.0])
        .frame(
            Frame{
                fill: egui::Color32::from_rgba_premultiplied(27, 27, 27, 240),
                corner_radius: CornerRadius::from(5),
                inner_margin: Margin::from(5),
                stroke: Stroke{
                    width: 0.8,
                    color: Color32::DARK_GRAY,
                },
                // shadow: Shadow{
                //     offset: [0, 0],
                //     blur: 255,
                //     color: Color32::BLACK,
                //     spread: 0,
                // },
                ..Default::default()
            }
        )
        .show(ctx, |ui| {
        ui.allocate_ui(vec2(ui.available_width(), scroll_height), |ui|{
            egui::ScrollArea::vertical().auto_shrink([true, true]).show(ui, |ui|{
                ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                ui.columns(3, |cols|{
                    let response = cols[0].vertical_centered_justified(|ui|{
                        if ui.button("")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("https://github.com/ArshvirGoraya/3D-Sorting-Visualizer")
                            .clicked(){
                                ui.ctx().open_url(egui::OpenUrl {
                                    new_tab: true,
                                    url: "https://github.com/ArshvirGoraya/3D-Sorting-Visualizer".to_string(),
                                });
                        }
                    });
                    first_button_size = response.response.rect.size();
                    cols[1].vertical_centered_justified(|ui|{
                        if ui.button("")
                            .on_hover_cursor(egui::CursorIcon::ZoomOut)
                            .on_hover_text("Decrease Font")
                            .clicked(){
                                decrease_font(&mut font_scale);
                        }
                    });
                    cols[2].vertical_centered_justified(|ui|{
                        if ui.button("")
                            .on_hover_cursor(egui::CursorIcon::ZoomIn)
                            .on_hover_text("Increase Font")
                            .clicked(){
                                increase_font(&mut font_scale);
                        }
                    });
                });

                ui.separator();
                //

            
                ui.horizontal(|ui|{
                    // Sorting:
                    let button_size = first_button_size * 0.75;
                    // let mut sort_button_size = egui::Vec2::new(0.0, 0.0);
                    ui.horizontal_top(|ui|{
                        if !is_sorting{
                            ui.add_enabled_ui(parsed_values.end_index >= 2, |ui|{
                                if ui.add_sized(
                                    button_size,
                                    egui::Button::new(" Sort! ").fill(egui::Color32::from_rgb(48, 64, 43))
                                )
                                    .on_hover_text("click to begin sorting")
                                        .clicked(){
                                            sort_state_set.set(sorter::SortState::Sorting);

                                }
                            });
                        } else if ui.add_sized(
                            button_size,
                            egui::Button::new(" Stop! ").fill(egui::Color32::from_rgb(83, 47, 52))
                        )
                            .on_hover_text("click to stop sorting")
                                .clicked(){
                                    sort_state_set.set(sorter::SortState::NotSorting);
                                    paused_state_set.set(sorter::PausedState::NotPaused);
                        }
                    });
                    // Pausing:
                    ui.horizontal_top(|ui|{
                        ui.add_enabled_ui(is_sorting, |ui|{
                            if !is_paused{
                                if ui.add_sized(
                                    button_size, 
                                    egui::Button::new("󰏥 ")
                                        .fill(egui::Color32::from_rgb(64, 64, 43)))
                                        .on_hover_text("click to pause sorting")
                                        .clicked()
                                {
                                    paused_state_set.set(sorter::PausedState::Paused);
                                }
                            } else if ui.add_sized(
                                button_size, 
                                egui::Button::new("󰐌 ")
                                .fill(egui::Color32::from_rgb(43, 58, 64)))
                                .on_hover_text("click to resume sorting")
                                    .clicked()
                            {
                                paused_state_set.set(sorter::PausedState::NotPaused);
                            }
                        });
                    });
                    ui.horizontal_top(|ui|{
                        // Selecting:
                        // INFO: Must wrap around a rect for tooltip...
                        // UI will not auto-update vertically when ComboBox is
                        // wrapped, but can use truncate wrapping to not worry about this.
                        let hover_size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
                        let (rect, _) = ui.allocate_exact_size(hover_size, egui::Sense::hover());
                        let mut child = ui.new_child(egui::UiBuilder{
                            max_rect: Some(rect),
                            ..Default::default()
                        });
                        //
                        child.add_enabled_ui(!is_sorting, |ui|{
                            egui::ComboBox::from_id_salt("sort_select")
                                .width(ui.available_width())
                                .selected_text(sort_select_get.get().to_string())
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .show_ui(ui, |ui|{
                                    for algorithm in sorter::Algorithms::ALL{
                                        if ui.selectable_value(&mut sort_select_get.get().clone(), algorithm, algorithm.to_string()).clicked(){
                                            sort_select_set.set(algorithm);
                                        }
                                    }
                                });
                        });
                        child.interact(rect, "sort_select_hover".into(), egui::Sense::hover())
                            .on_hover_text("select sorting algorithm");
                        //
                    });
                });

                ui.style_mut().override_text_style = None;

                ui.horizontal(|ui|{
                    ui.label("Sort speed 󰔛 ");
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
                            let duration = increment_timer.duration_f64;
                            increment_timer.increment_timer.set_duration(
                                Duration::from_secs_f64(duration)
                            );
                    }
                });
                //
                ui.label(format!("Elapsed Time: {}ms", sorting_time.time_elapsed.as_millis()))
                    .on_hover_text("total time spent: including time spend sorting + time spent doing other things");
                ui.label(format!("Sorting Time: {}ms", sorting_time_isolated.time_elapsed.as_millis()))
                    .on_hover_text("time spent just on sorting functions (not any intermediate game engine functions)");

                ui.separator();

                //
                // CAMERA
                // 

                ui.style_mut().override_text_style = Some(egui::TextStyle::Name("medium".into()));
                ui.columns(3, |cols|{
                    cols[0].vertical_centered_justified(|ui|{
                        if ui.button("Reset ")
                            .on_hover_text("reset the camera to original position")
                            .clicked(){
                                clicked_cube.index = None;
                                center_camera(
                                    get_cube_size_from_width_scale(parsed_values.end_index, &cube_scale_controls), 
                                    parsed_values.end_index, 
                                    &mut camera_query, 
                                    &scanned_cube,
                                    &camera_controls_follow_selected,
                                    &sort_state_get
                                );
                                let mut pan_orbit = camera_query.single_mut().unwrap();
                                pan_orbit.target_yaw = 0.0;
                                pan_orbit.target_pitch = 0.0;
                        }
                    });
                    cols[1].vertical_centered_justified(|ui|{
                        if ui.checkbox(&mut camera_controls_auto_rotate, "Rotate 󱦙")
                            .on_hover_text("continuously rotate camera")
                            .changed(){
                                if *camera_controls_auto_rotate{
                                    camera_controls_auto_rotate_set.set(crate::CameraControlsAutoRotate::AutoRotate);
                                }else{
                                    camera_controls_auto_rotate_set.set(crate::CameraControlsAutoRotate::NotAutoRotate);
                                }
                        }
                    });
                    cols[2].vertical_centered_justified(|ui|{
                        if ui.checkbox(&mut camera_controls_follow_selected, "Follow 󰮄")
                            .on_hover_text("follow the sorting algorithm as it scans across the cubes")
                            .changed(){
                                if *camera_controls_follow_selected{
                                    camera_controls_follow_set.set(crate::CameraControlsFollow::Following);
                                } else {
                                    camera_controls_follow_set.set(crate::CameraControlsFollow::NotFollowing);
                                }
                        }
                    });
                });

                ui.separator();

                // 
                // AUDIO
                //

                ui.horizontal(|ui|{
                    if ui.checkbox(&mut audio_controls.enabled, "Audio ")
                        .on_hover_text("toggle audio").changed(){
                            // This deletes ALL running audio if toggle is EVER changed.
                            stop_all_audio_event.write(crate::StopAllAudio);
                    }

                    ui.vertical_centered_justified(|ui|{
                        ui.collapsing("Audio Settings", |ui|{
                            ui.add(
                                egui::Slider::new(&mut audio_controls.volume, 0.1..=10.0).text("Volume")
                                .max_decimals(1)
                                .step_by(0.1)
                            ).on_hover_text("adjust volume of selected sound");

                            if ui.add(
                                egui::Slider::new(&mut audio_controls.high_pitch, 0.1..=2.0).text("High Pitch")
                                .max_decimals(1)
                                .step_by(0.1)
                            ).on_hover_text("adjust pitch of selected sound at high range").changed(){
                                audio_controls.pitch_range = audio_controls.high_pitch - audio_controls.low_pitch;
                            };

                            if ui.add(
                                egui::Slider::new(&mut audio_controls.low_pitch, 0.1..=2.0).text("Low Pitch")
                                .max_decimals(1)
                                .step_by(0.1)
                            ).on_hover_text("adjust pitch of selected sound at low range").changed(){
                                audio_controls.pitch_range = audio_controls.high_pitch - audio_controls.low_pitch;
                            };

                            ui.columns(2, |cols|{
                                cols[0].add_enabled_ui(true, |ui|{
                                    ui.vertical_centered_justified(|ui|{
                                        if ui.button("Pick Audio")
                                            .on_hover_text("open file dialog to select supported audio file")
                                            .clicked(){

                                                #[cfg(target_arch = "x86_64")]
                                                {
                                                    #[allow(clippy::collapsible_if)]
                                                    if let Some(path) = rfd::FileDialog::new().add_filter("audio", &["aac", "flac", "wav", "ogg", "mp3"]).pick_file(){
                                                        if let Ok(bytes) = std::fs::read(&path){
                                                            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                                                            crate::change_audio_source(&mut audio_controls, &mut audio_assets, file_name, bytes);
                                                        }
                                                    }
                                                }
                                                #[cfg(any(target_arch = "wasm32", rust_analyzer))]
                                                {
                                                    use web_sys::{HtmlInputElement, wasm_bindgen::JsCast};
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
                                                }
                                            }
                                        );
                                    });
                                    cols[1].vertical_centered_justified(|ui|{
                                        ui.add_enabled_ui(audio_controls.audio_source_handle.is_some(), |ui|{
                                            if ui.button("Default")
                                                .on_hover_text("reset selected audio to default")
                                                    .clicked(){
                                                        if let Some(audio_handle) = &audio_controls.audio_source_handle{
                                                            audio_assets.remove(audio_handle);
                                                        }
                                                        audio_controls.selected_file_name = None;
                                                        audio_controls.audio_source_handle = None;
                                                    }
                                        });
                                    })
                                });
                                ui.vertical_centered_justified(|ui|{
                                    let file_name = audio_controls.selected_file_name.as_ref().unwrap_or(&audio_controls.default_file_name);
                                    if ui.add(egui::Button::new(file_name)
                                        .wrap_mode(egui::TextWrapMode::Truncate)
                                        .fill(egui::Color32::from_rgb(0, 0, 0)))
                                        .on_hover_text("play selected sound")
                                        .clicked(){
                                            // play base pitch by calling audio and making it think
                                            // we selected the middle cube (of 3 cubes (0, 1, 2), 1
                                            // is selected)
                                            crate::play_audio(&mut commands, &audio_controls.into(), 1, 2);
                                    }
                                });
                            });
                        });
                    });

                    ui.separator();

                    //
                    // RNG values
                    // 

                    ui.horizontal(|ui|{
                        ui.add_enabled_ui(!is_sorting, |ui|{
                            if ui.button("RNG ")
                                .on_hover_text("replace your text with random numbers")
                                    .clicked(){
                                        generate_random_string_nums(
                                            rng_values_controls.amount, 
                                            rng_values_controls.min, 
                                            rng_values_controls.max, 
                                            rng_values_controls.max_decimals,
                                            &mut user_text.val, 
                                            &mut random
                                        );
                                        *generated_rng_values = true;
                            }
                        });
                        ui.vertical_centered_justified(|ui|{
                            ui.collapsing("RNG Settings", |ui|{
                                if ui.add(
                                    egui::Slider::new(&mut rng_values_controls.amount, 2..=100)
                                    .clamping(egui::SliderClamping::Never)
                                    .text("amount")
                                )
                                    .on_hover_text("set amount of numbers to generate")
                                    .changed(){
                                        rng_values_controls.amount = rng_values_controls.amount.max(2); // at least 2
                                }
                                if ui.add(
                                    egui::Slider::new(&mut rng_values_controls.min, -100.0..=100.0)
                                    .clamping(egui::SliderClamping::Never)
                                    .min_decimals(1)
                                    .text("min")
                                )
                                    .on_hover_text("set smallest number that can be generated")
                                    .changed(){
                                        // set max to be the same as this, if this is bigger than max.
                                        rng_values_controls.max = rng_values_controls.max.max(rng_values_controls.min);
                                }
                                if ui.add(
                                    egui::Slider::new(&mut rng_values_controls.max, -100.0..=100.0)
                                    .clamping(egui::SliderClamping::Never)
                                    .min_decimals(1)
                                    .text("max")
                                )
                                    .on_hover_text("set largest number that can be generated")
                                    .changed(){
                                        // set min to be the same as this, if this is smaller than min.
                                        rng_values_controls.min = rng_values_controls.min.min(rng_values_controls.max);
                                }
                                if ui.add(
                                    egui::Slider::new(&mut rng_values_controls.max_decimals, 0..=10)
                                    .clamping(egui::SliderClamping::Never)
                                    .min_decimals(1)
                                    .text("decimals")
                                )
                                    .on_hover_text("max amount of decimal places after RNG value")
                                    .changed(){
                                        rng_values_controls.max_decimals = rng_values_controls.max_decimals.max(0); // at least 0
                                }
                            })
                        })
                    });

                    ui.separator();

                    //
                    // RNG colors 
                    //

                    ui.horizontal(|ui|{
                        if ui.button("RNG ")
                            .on_hover_text("replace cube colors with random colors")
                            .clicked(){
                                rng_color_controls.rng_cubes_enabled = true;
                                set_cube_colors(
                                    rng_color_controls.rng_cubes_enabled, 
                                    true,
                                    &mut parsed_values, 
                                    &mut cubes_query, 
                                    &cube_assets, 
                                    &mut materials, 
                                    &mut random,
                                    &sort_colored_cubes,
                                );
                        };
                        ui.vertical_centered_justified(|ui|{
                            ui.collapsing("Colors Settings", |ui|{
                                ui.horizontal(|ui|{
                                    if ui.color_edit_button_srgb(&mut rng_color_controls.background_color)
                                        .on_hover_text("replace background color")
                                        .changed(){
                                            bg_color.0 = Color::srgb_u8(
                                                rng_color_controls.background_color[0], 
                                                rng_color_controls.background_color[1],
                                                rng_color_controls.background_color[2]
                                            )
                                    }
                                    ui.label("background color");
                                });
                                if ui.checkbox(&mut rng_color_controls.rng_cubes_enabled, "Use RNG colors")
                                    .on_hover_text("toggle random color usage")
                                    .changed(){
                                        set_cube_colors(
                                            rng_color_controls.rng_cubes_enabled, 
                                            false,
                                            &mut parsed_values, 
                                            &mut cubes_query, 
                                            &cube_assets, 
                                            &mut materials, 
                                            &mut random,
                                            &sort_colored_cubes
                                        );
                                }
                            });
                        });
                    });

                    ui.separator();

                    // 
                    // Cube Scale: Width, Height
                    //

                    if ui.checkbox(&mut cube_scale_controls.positional_heights, "Positional Heights 󰄩")
                        .on_hover_text("set cube heights to be relative to the position they will be in their final sorted positions instead of simply being as tall as their value")
                        .changed(){
                            control_cube_heights(&mut parsed_values, &cube_scale_controls, &mut cubes_query);
                    }
                    ui.horizontal(|ui|{
                        if ui.checkbox(&mut cube_scale_controls.height_scale_enable, "Height Scale ")
                            .on_hover_text("use height scale to scale the heights of cubes")
                            .changed(){
                                control_cube_heights(&mut parsed_values, &cube_scale_controls, &mut cubes_query);
                        }
                        if ui.add_enabled(
                            cube_scale_controls.height_scale_enable, 
                            egui::Slider::new(&mut cube_scale_controls.height_scale, 0.0..=10.0)
                            .clamping(egui::SliderClamping::Never)
                        )
                            .on_hover_text("set height scale for cubes")
                            .changed(){
                                control_cube_heights(&mut parsed_values, &cube_scale_controls, &mut cubes_query);
                        }
                    });
                    ui.horizontal(|ui|{
                        if ui.checkbox(&mut cube_scale_controls.width_scale_enable, "Width Scale ")
                            .on_hover_text("use width scale to scale the widths of cubes")
                            .changed(){
                                control_cube_widths(
                                    &mut parsed_values, 
                                    &cube_scale_controls, 
                                    &mut cubes_query, 
                                    &mut camera_query, 
                                    &scanned_cube,
                                    &camera_controls_follow_selected,
                                    &sort_state_get,
                                );
                        }
                        if ui.add_enabled(
                            cube_scale_controls.width_scale_enable, 
                            egui::Slider::new(&mut cube_scale_controls.width_scale, 0.0..=100.0)
                            .clamping(egui::SliderClamping::Never)
                        )
                            .on_hover_text("set total width that the cubes can encompass")
                            .changed(){
                                control_cube_widths(
                                    &mut parsed_values, 
                                    &cube_scale_controls, 
                                    &mut cubes_query, 
                                    &mut camera_query, 
                                    &scanned_cube,
                                    &camera_controls_follow_selected,
                                    &sort_state_get,
                                );
                        }
                    });

                    ui.separator();

                    ui.style_mut().override_text_style = None;
                    ui.style_mut().visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                    let (parse_warning_color, parse_warning_string) = get_parse_warning_color(&worse_parse_problem);

                    egui::CollapsingHeader
                        ::new(egui::RichText::new(parse_warning_string)
                            .color(parse_warning_color))
                        .id_salt("scroll_parsed_collapsible")
                        .default_open(false)
                        .show_unindented(ui, |ui|{
                            ui.vertical_centered_justified(|ui|{
                                ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                                let clean_button = ui.add_enabled(*text_is_dirty && !is_sorting, egui::Button::new("Clean 󰃢"))
                                    .on_hover_text("replace text with internal representation of your numbers")
                                    .on_disabled_hover_text("replace text with internal representation of your numbers");
                                if clean_button.clicked(){
                                    *text_just_cleaned = true;
                                    *text_is_dirty = false;
                                    user_text.val = parsed_values.vals[..parsed_values.end_index].iter().map(|x|{
                                        x.converted_value.to_string()
                                    }).collect::<Vec<_>>().join(", ");
                                }
                                ui.style_mut().override_text_style = None;
                            });

                            if !num_strings.cleaned_string{
                                num_strings.cleaned_string = true;
                                num_strings.val = parsed_values.vals[..parsed_values.end_index].iter().map(|x| x.converted_value.to_string()).collect::<Vec<_>>().join(", ");
                            }
                            ui.allocate_ui(vec2(ui.available_width(), 200.0), |ui|{
                                ui.push_id("scroll_parsed", |ui|{
                                    egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui|{
                                        ui.add_enabled(false, 
                                            egui::TextEdit::multiline(&mut num_strings.val).hint_text("parsed numbers here").desired_width(ui.available_width())
                                        ).on_disabled_hover_text("shows internal representation of your numbers");
                                    });
                                });
                            });
                        });

                    ui.vertical_centered_justified(|ui|{
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Name("symbol_font".into()));
                        let copy_response = ui.button("Copy 󰨸").on_hover_text("copy text to clipboard");
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
                        ui.style_mut().override_text_style = None;
                    });

                    ui.allocate_ui(vec2(ui.available_width(), 200.0), |ui|{
                        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui|{
                            let text_edit_widget_enabled = !is_sorting && !*wasm_on_mobile;

                            let mut text_edit_widget = ui.add_enabled(text_edit_widget_enabled, egui::TextEdit::multiline(&mut user_text.val)
                                .hint_text("numbers here")
                                .desired_width(ui.available_width()))
                                .on_hover_text("supports positive and negative ints and floats with the following regex expression: r\"-?\\d+(?:\\.\\d+)?\"");

                            if *generated_rng_values{
                                text_edit_widget.mark_changed();
                            }
                            if *text_just_cleaned{
                                text_edit_widget.mark_changed();
                                *text_just_cleaned = false;
                            }

                            if !is_sorting && text_edit_widget.changed(){
                                if !*generated_rng_values{
                                    // if rng values were generated, already clean.
                                    // if text change without rng values generated, mark as dirty.
                                    *text_is_dirty = true;
                                }

                                // TODO: maybe add fancy stuff like remembering which parts of the string are already
                                // parsed, and parsing only new stuff and deleting any removed stuff.
                                // Could carry over to spawning cubes where not all cubes are respawned: instead only
                                // new cubes are added?
                                // TODO: instead of doing things within the regex loop, can do
                                // everything afterwards: removes the need for the second loop and
                                // allows using CubeData component for everything instead of ParsedValues.
                                num_strings.cleaned_string = false;
                                update_parsed_values(
                                    number_regex,
                                    &user_text,
                                    worse_parse_problem,
                                    &mut commands,
                                    & cube_assets,
                                    &mut parsed_values,
                                    &mut cubes_query,
                                    &mut camera_query,
                                    &mut random,
                                    &mut materials,
                                    rng_color_controls.rng_cubes_enabled,
                                    &mut cube_scale_controls,
                                    &scanned_cube,
                                    &camera_controls_follow_selected,
                                    &sort_state_get,
                                    &mut clicked_cube,
                                );
                            }
                            *generated_rng_values = false;
                        });
                    });
                    #[cfg(any(target_arch = "wasm32", rust_analyzer))]
                    {
                        if *wasm_on_mobile{
                            if ui.add(
                                egui::Label::new(
                                    RichText::new("Cannot use above TextEdit on mobile. Generate random values instead.")
                                    .color(get_parse_warning_color(&ParsedWarning::PrecisionLoss).0)
                                    .underline()
                                ).wrap_mode(egui::TextWrapMode::Wrap)
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Click to see issue why TextEdit cannot be used on mobile builds.")
                            .clicked(){
                                ui.ctx().open_url(egui::OpenUrl {
                                    new_tab: true,
                                    url: "https://github.com/vladbat00/bevy_egui/issues/246".to_string(),
                                });
                            };
                        }
                    }
            });
        });

    });

    let window_response = window_area.unwrap().response;
    let pointer_pos = ctx.pointer_latest_pos().unwrap_or_default();
    let window_contains_pointer = window_response.rect.contains(pointer_pos) || window_response.dragged();
    

    let mut pan_orbit = camera_query.single_mut().unwrap();
    pan_orbit.enabled = !ctx.is_using_pointer();

    let control_pressed = keyboard_input.pressed(KeyCode::ControlLeft) || keyboard_input.pressed(KeyCode::ControlRight);
    if window_contains_pointer || control_pressed{
        // disable camera zooming
        pan_orbit.zoom_sensitivity = 0.0;
    }else{
        // enable camera zooming; set to default sensitivity
        pan_orbit.zoom_sensitivity = 1.0;
    }

    // draw egui frame over hovered cube:
    if !window_contains_pointer && let Some(hover_cube_id) = hovered_cube.id{
        let mut pos = pointer_pos;
        if hovered_cube.using_touch{
            // position top right if touching over cube (instead of hovering with a mouse)
            pos.y = 0.0;
            pos.x = screen_width;
        }
        egui::Area::new(egui::Id::new("cube_hover_area"))
            .fixed_pos(pos)
            .interactable(false)
            .show(ctx, |ui|{
                egui::Frame::default()
                    .fill(egui::Color32::BLACK)
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(15))
                    .show(ui, |ui|{
                        if let Ok((_, _, _, cube_data)) = cubes_query.get(hover_cube_id){
                                let parsed_value = &parsed_values.vals[cube_data.index];
                                ui.add(
                                    egui::Label::new(
                                        format!("Value: {}", parsed_value.converted_value)
                                    ).wrap_mode(egui::TextWrapMode::Extend)
                                );
                                ui.add(egui::Label::new(
                                        format!("Start Position: {}", cube_data.index)
                                ).wrap_mode(egui::TextWrapMode::Extend)
                                );
                                ui.add(
                                    egui::Label::new(
                                        format!("End Position: {}", parsed_value.sorted_position)
                                    ).wrap_mode(egui::TextWrapMode::Extend)
                                );
                                if parsed_value.parsed_warning != ParsedWarning::Ok{
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!("Parse Warning: {}", parsed_value.parsed_warning))
                                            .color(get_parse_warning_color(&parsed_value.parsed_warning).0)
                                        ).wrap_mode(egui::TextWrapMode::Extend)
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!("Raw string: \"{}\"", 
                                                    &user_text.val[parsed_value.matched_string.start_index..parsed_value.matched_string.end_index]
                                                    ))
                                            .color(get_parse_warning_color(&parsed_value.parsed_warning).0)

                                        ).wrap_mode(egui::TextWrapMode::Wrap)
                                    );
                                }
                        }

                    })
            });
    }

    Ok(())
}

fn detect_precision_loss(original: &str, parsed: f64) -> bool {
    let num_string = parsed.to_string();
    let new_string = string_trim_zeros(original);
    // if new_string != num_string {
    //     log::info!("\tprecision loss detected!")
    // }
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
        (Body, FontId::new(18.0 * scale, Proportional)),
        (Monospace, FontId::new(14.0 * scale, Proportional)),
        (Button, FontId::new(14.0 * scale, Proportional)),
        (Small, FontId::new(10.0 * scale, Proportional)),
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
        scroll: ScrollStyle{
            bar_width: 6.0,
            ..Default::default()
        },
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

    // symbols added in from font: https://www.nerdfonts.com/cheat-sheet
    // font ONLY has these symbols in it and NOTHING else:
    //
    //  // (github) U+e709
    //  // (font increase) U+eb69
    //  // (font decrease) U+eb6a // this glyph was changed from the standard nerd font glyph at this position to a flipped version of U+eb69.
    // 󰔛 // U+f051b (sort speed)
    //  // (camera) U+f447
    // 󱦙 // (rotate camera) U+f1999
    // 󰮄 // (follow cube) U+f0b84
    //  // (audio toggle) U+e638
    //  // (random numbers) U+edec
    //  // (randomize color) U+e22b
    // 󰄩 // (positional heights) U+f0129
    //  // (height scale) U+f07d
    //  // (width scale) U+f07e
    // 󰨸 // (clipboard) U+f0a38
    // 󰃢 // (broom / clean) U+f00e2
    //
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
