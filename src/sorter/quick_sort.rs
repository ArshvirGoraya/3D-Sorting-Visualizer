use std::ops::{Range, RangeBounds};

use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        entity::Entity,
        event::Event,
        message::{Message, MessageReader, MessageWriter},
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    reflect::{List, Set},
    state::state::{NextState, States},
    time::Time,
    transform::components::Transform,
};

use crate::{
    sorter,
    ui::{CubeAssets, ParsedValue, ParsedValues, StringInfo, UserText},
};

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    Range,
    Pivot,
    Swap,
    J,
    I,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortStep {
    #[default]
    SetupRange,
    Swap,
    Compare,
}

#[derive(Resource)]
pub struct QuickSortColors {
    pub materials: HashMap<SortColor, MeshMaterial3d<StandardMaterial>>,
}

impl FromWorld for QuickSortColors {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            materials: HashMap::from([
                (
                    SortColor::Range,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(238, 212, 159),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Pivot,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(237, 135, 150),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Swap,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(138, 173, 244),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::I,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(198, 160, 246),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::J,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(166, 218, 149),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
            ]),
        }
    }
}

#[derive(Resource)]
pub struct SortState {
    started: bool,
    sub_arrays: Vec<(usize, usize)>,
    current_array: (usize, usize),
    pivot: usize,
    j: usize,
    i: isize,
    swapped_cubes: Option<(usize, usize)>,
    next_step: SortStep,
}

pub fn setup_range(
    mut commands: Commands,
    // parsed_values: Res<ParsedValues>,
    parsed_values: Res<ParsedValues>,
    mut sort_state: Option<ResMut<SortState>>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    // cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    // mut next_event: MessageWriter<Compare>,
) {
    // Only inserts if not already added:
    // commands.init_resource::<QuickSortColors>();
    println!("Quick Sorting...");

    if let Some(mut sort_state) = sort_state {
        let previous_array = sort_state.current_array;
        sort_state.current_array = *sort_state.sub_arrays.last_mut().unwrap();
        sort_state.pivot = sort_state.current_array.1 - 1;
        sort_state.j = sort_state.current_array.0;
        sort_state.i = (sort_state.current_array.0 as isize) - 1;
        log::info!(
            "
            current array: ({}, {})
            j: {}
            i: {}
            ",
            sort_state.current_array.0,
            sort_state.current_array.1,
            sort_state.j,
            sort_state.i
        );

        setup_range_color(
            previous_array,
            sort_state.current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
        );
        sort_state.next_step = SortStep::Compare;
    } else {
        let current_array = (0, parsed_values.end_index);
        commands.insert_resource(SortState {
            started: true, // TODO: not needed?
            sub_arrays: [current_array].to_vec(),
            current_array,
            pivot: current_array.1 - 1,
            j: current_array.0,
            i: (current_array.0 as isize) - 1,
            swapped_cubes: None,
            next_step: SortStep::Compare,
        });
        log::info!(
            "
            current array: ({}, {})
            j: {}
            i: {}
            ",
            current_array.0,
            current_array.1,
            current_array.0,
            (current_array.0 as isize) - 1
        );

        setup_range_color(
            current_array,
            current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
        );
    }
}

pub fn compare(
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    // mut cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    mut sort_state: ResMut<SortState>,
    parsed_values: Res<ParsedValues>,
    quick_sort_colors: Res<QuickSortColors>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
) {
    if let Some((i, j)) = sort_state.swapped_cubes {
        // if just swapped, increment j and color the just swapped cubes the J/"covered" color.
        color_cube(
            i,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        color_cube(
            j,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        sort_state.swapped_cubes = None;

        // j must increment after swapping.
        sort_state.j += 1;
        if sort_state.j != sort_state.pivot + 1 {
            color_cube(
                sort_state.j,
                SortColor::J,
                &quick_sort_colors,
                &parsed_values,
                &mut cubes_query,
            );
        }
    }
    if sort_state.j == sort_state.pivot + 1 {
        // when pivot has been reached, j and i swap and j increments 1. That is when new subarrays
        // are creted. j must be 1 larger than pivot.
        // pivot has been reached, create new subarrays or complete.

        log::info!("pivot overreached last swap complete: {}", sort_state.j);

        sort_state.sub_arrays.pop();

        // check if next array will be valid: if the subarray will have more than 1 value in it.
        let (start, end) = sort_state.current_array;

        let i = sort_state.i as usize;
        let right_array_start = i + 1;

        if end.abs_diff(right_array_start) > 1 {
            log::info!("right array is valid: {} to {}", right_array_start, end);
            // Right array is valid
            sort_state.sub_arrays.push((right_array_start, end));
        }
        if start.abs_diff(i) > 1 {
            log::info!("left array is valid: {} to {}", start, i);
            // Left array is valid
            sort_state.sub_arrays.push((start, i));
        }

        if sort_state.sub_arrays.is_empty() {
            // if no arrays remain, done sorting.
            sort_select_set.set(sorter::SortState::NotSorting);
        } else {
            sort_state.next_step = SortStep::SetupRange;
        }
        return;
    }
    //
    let pivot_value = &parsed_values.vals[sort_state.pivot].converted_value;
    let j_value = &parsed_values.vals[sort_state.j].converted_value;
    if sort_state.j == sort_state.pivot || pivot_value > j_value {
        // if j reached the pivot do a last swap with i and j
        // if j value is smaller than pivot, swap i and j.

        // increment i before swapping
        sort_state.i += 1;

        if sort_state.i as usize == sort_state.j {
            // if both indices point to the same thing, no need to swap them.
            // just increment j and return.
            sort_state.j += 1;
            if sort_state.j != sort_state.pivot + 1 {
                color_cube(
                    sort_state.j,
                    SortColor::J,
                    &quick_sort_colors,
                    &parsed_values,
                    &mut cubes_query,
                );
            }
            sort_state.next_step = SortStep::Compare;
            return;
        }

        // NOTE: if swapping, j must be incremented after swap. We don't increment j in here though.
        // We wait for swap system to complete and then it re-calls this system and is then
        // incremented above once swap is complete.
        sort_state.swapped_cubes = Some((sort_state.i as usize, sort_state.j));
        // these cubes get set to J/"covered" color once swapping is done.
        color_cube(
            sort_state.i as usize,
            SortColor::Swap,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        color_cube(
            sort_state.j,
            SortColor::Swap,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        sort_state.next_step = SortStep::Swap;
    } else {
        // if pivot_value < j_value
        // do not swap. just increment j.
        sort_state.j += 1;
        color_cube(
            sort_state.j,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        sort_state.next_step = SortStep::Compare;
    }
}

pub fn complete(
    sort_state: Option<Res<SortState>>,
    cube_assets: Option<Res<CubeAssets>>,
    parsed_values: Res<ParsedValues>,
    mut cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut commands: Commands,
) {
    // INFO: will run at startup: runs when Quicksort is the selected algorithm
    // (which is the default) and OnEnter for NotSorting (which is the default)
    if let Some(sort_state) = sort_state
        && let Some(cube_assets) = cube_assets
    {
        // Reset previously selected range to normal colors.
        for i in sort_state.current_array.0..sort_state.current_array.1 {
            let parsed_value = &parsed_values.vals[i];
            let mut cube_material = cubes_query
                .get_mut(parsed_values.vals[i].cube_handle)
                .unwrap();

            *cube_material = crate::ui::get_cube_material(
                rng_color_controls.rng_cubes_enabled,
                parsed_value.parsed_warning,
                &cube_assets,
                parsed_value.rng_color.clone(),
            );
        }
        commands.remove_resource::<SortState>();
    }
}

pub fn swap(
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    mut user_text: ResMut<UserText>,
) {
    log::info!("\n\t\t=-=-=Swapping=-=-=");

    log::info!(
        "geting parsed value at: {} and {}",
        sort_state.i as usize,
        sort_state.j
    );

    let [mut i_data, mut j_data] = parsed_values
        .vals
        .get_disjoint_mut([sort_state.i as usize, sort_state.j])
        .unwrap();

    log::info!(
        "
        \ttext before: {}
        \ti index: [{}], value: {}, text value: \"{}\", raw start: {}, match start: {}, end: {}
        \tj index: [{}], value: {}, text value: \"{}\", , raw start: {}, match start: {}, end: {}
        ",
        user_text.val,
        sort_state.i,
        i_data.converted_value,
        &user_text.val[i_data.matched_string.start_index..i_data.matched_string.end_index],
        i_data.raw_string.start_index,
        i_data.matched_string.start_index,
        i_data.matched_string.end_index,
        sort_state.j,
        j_data.converted_value,
        &user_text.val[j_data.matched_string.start_index..j_data.matched_string.end_index],
        j_data.raw_string.start_index,
        j_data.matched_string.start_index,
        j_data.matched_string.end_index,
    );

    let [
        (mut transform_i, mut material_i, mut cube_data_i),
        (mut transform_j, mut material_j, mut cube_data_j),
    ] = cubes_query
        .get_many_mut([i_data.cube_handle, j_data.cube_handle])
        .unwrap();
    // Swap positions:
    std::mem::swap(
        &mut transform_i.translation.x,
        &mut transform_j.translation.x,
    );
    // Swap index pointer to ParsedValues.vals location
    std::mem::swap(&mut cube_data_i.index, &mut cube_data_j.index);
    // Swap ParsedValue Data (raw_string, matched_string get swapped later in a more careful way)
    std::mem::swap(&mut i_data.sorted_position, &mut j_data.sorted_position);
    std::mem::swap(&mut i_data.cube_handle, &mut j_data.cube_handle);
    std::mem::swap(&mut i_data.rng_color, &mut j_data.rng_color);
    std::mem::swap(&mut i_data.parsed_warning, &mut j_data.parsed_warning);
    std::mem::swap(&mut i_data.converted_value, &mut j_data.converted_value);
    // Swap & Update Text

    let (shift_left, text_shift_amount) = swap_text(
        &mut i_data.matched_string,
        &mut j_data.matched_string,
        &mut i_data.raw_string,
        &mut j_data.raw_string,
        &mut user_text,
    );

    log::info!(
        "
        \ttext after: {}
        \ti index: [{}], value: {}, text value: \"{}\", raw start: {}, match start: {}, end: {}
        \tj index: [{}], value: {}, text value: \"{}\", , raw start: {}, match start: {}, end: {}
        \t=-=-=-=-=-=-=-=-=
        ",
        user_text.val,
        sort_state.i,
        i_data.converted_value,
        &user_text.val[i_data.matched_string.start_index..i_data.matched_string.end_index],
        i_data.raw_string.start_index,
        i_data.matched_string.start_index,
        i_data.matched_string.end_index,
        sort_state.j,
        j_data.converted_value,
        &user_text.val[j_data.matched_string.start_index..j_data.matched_string.end_index],
        j_data.raw_string.start_index,
        j_data.matched_string.start_index,
        j_data.matched_string.end_index,
    );

    update_text_indices(
        &mut sort_state,
        parsed_values,
        shift_left,
        text_shift_amount,
        &mut user_text,
    );

    sort_state.next_step = SortStep::Compare;
}

pub fn update_text_indices(
    sort_state: &mut ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    shift_left: bool,
    text_shift_amount: usize,
    user_text: &mut ResMut<UserText>,
) {
    if text_shift_amount == 0 {
        return;
    }

    // ensure left and right are not right next to each other (since in that case, no indices
    // between them exist and hence don't need to be updated).
    let mut left = sort_state.i as usize;
    let mut right = sort_state.j;

    if left + 1 == right {
        log::info!(
            "no need to updated inbetween indices since left+1 == right: {}+1 = {}",
            left,
            right
        );
        return;
    }

    // need to update 1 ahead of left (since left itself is updated in swap_string_info)
    // need to update 1 before right(since right itself is updated in swap_string_info)
    left += 1;
    right -= 1;
    for parsed_value in &mut parsed_values.vals[left..=right] {
        if shift_left {
            parsed_value.matched_string.start_index -= text_shift_amount;
            parsed_value.matched_string.end_index -= text_shift_amount;
            parsed_value.raw_string.start_index -= text_shift_amount;
            parsed_value.raw_string.end_index -= text_shift_amount;
        } else {
            parsed_value.matched_string.start_index += text_shift_amount;
            parsed_value.matched_string.end_index += text_shift_amount;
            parsed_value.raw_string.start_index += text_shift_amount;
            parsed_value.raw_string.end_index += text_shift_amount;
        }
    }
}

pub fn swap_text(
    i_match_string: &mut StringInfo,
    j_match_string: &mut StringInfo,
    i_raw_string: &mut StringInfo,
    j_raw_string: &mut StringInfo,
    mut user_text: &mut ResMut<UserText>,
) -> (bool, usize) {
    let mut text_shift_amount: usize = 0;
    let mut shift_left = false;
    let i_match_string_length = i_match_string.end_index - i_match_string.start_index;
    let j_match_string_length = j_match_string.end_index - j_match_string.start_index;

    // UNSAFE: rust has no way to verify if UTF8 will be valid
    // But we are not changing any characters, just re-arranging them by fixed-length, so
    // shouldn't be a problem AS LONG AS THE CHARACTERS BEING SWAPPED DO NOT OVERLAP (which
    // they dont)
    let utf8_text = unsafe { user_text.val.as_bytes_mut() };

    if i_match_string_length == j_match_string_length {
        log::info!("\n\t\tswapping same length text");
        // In-place swap
        let [i_utf8, j_utf8] = utf8_text
            .get_disjoint_mut([
                i_match_string.start_index..i_match_string.end_index,
                j_match_string.start_index..j_match_string.end_index,
            ])
            .unwrap();
        i_utf8.swap_with_slice(j_utf8);
    } else if i_match_string_length > j_match_string_length {
        log::info!(
            "\n\t\tswapping: left text is bigger: \"{}\" > \"{}\"",
            std::str::from_utf8(&utf8_text[i_match_string.start_index..i_match_string.end_index])
                .unwrap(),
            std::str::from_utf8(&utf8_text[j_match_string.start_index..j_match_string.end_index])
                .unwrap(),
        );

        // left is bigger: copy left, put right text to left position, shift everything between
        // left and right text leftwards, insert copied text right of everything that just shifted.

        // store left-copy
        let utf8_temp = Vec::from(&utf8_text[i_match_string.start_index..i_match_string.end_index]);

        // move right to left
        let new_j_match_string_position = i_match_string.start_index;
        utf8_text.copy_within(
            j_match_string.start_index..j_match_string.end_index,
            new_j_match_string_position,
        );

        // shift all values in range leftwards
        let shift_range = i_match_string.end_index..j_match_string.start_index;
        let shift_range_length = shift_range.len();
        utf8_text.copy_within(
            // Every character within this range must be shifted leftwards
            shift_range,
            // to the end of the left-ward string which is j.
            i_match_string.start_index + j_match_string_length,
        );
        let previous_char_position = i_match_string.end_index;
        let new_char_position = i_match_string.start_index + j_match_string_length;
        text_shift_amount = previous_char_position.abs_diff(new_char_position);
        shift_left = true;

        // insert the saved utf8_temp at the position after everything shifted left.
        let new_i_match_string_position =
            i_match_string.start_index + j_match_string_length + shift_range_length;

        utf8_text[new_i_match_string_position..new_i_match_string_position + i_match_string_length]
            .copy_from_slice(&utf8_temp);

        // update raw and match string indices
        swap_string_info(
            i_match_string,
            j_match_string,
            i_raw_string,
            j_raw_string,
            new_i_match_string_position,
            new_j_match_string_position,
            i_match_string_length,
            j_match_string_length,
        );
    } else {
        log::info!(
            "\n\t\tswapping: right text is bigger: \"{}\" < \"{}\"",
            std::str::from_utf8(&utf8_text[i_match_string.start_index..i_match_string.end_index])
                .unwrap(),
            std::str::from_utf8(&utf8_text[j_match_string.start_index..j_match_string.end_index])
                .unwrap(),
        );
        // right is bigger: copy right, put left text to right position, shift everything between
        // left and right text rightwards, insert copied text left of everything that just shifted.

        // store right-copy
        let utf8_temp = Vec::from(&utf8_text[j_match_string.start_index..j_match_string.end_index]);

        // move left to right
        let new_i_match_string_position = j_match_string.end_index - i_match_string_length;
        utf8_text.copy_within(
            i_match_string.start_index..i_match_string.end_index,
            new_i_match_string_position,
        );

        // shift all values in range:
        let shift_range = i_match_string.end_index..j_match_string.start_index;
        utf8_text.copy_within(
            // Every character within this range must be shifted rightwards
            shift_range,
            // to the end of the left-ward string which is j.
            i_match_string.start_index + j_match_string_length,
        );
        let previous_char_position = i_match_string.end_index;
        let new_char_position = i_match_string.start_index + j_match_string_length;

        text_shift_amount = previous_char_position.abs_diff(new_char_position);

        let new_j_match_string_position = i_match_string.start_index;
        utf8_text[new_j_match_string_position..new_j_match_string_position + j_match_string_length]
            .copy_from_slice(&utf8_temp);

        swap_string_info(
            i_match_string,
            j_match_string,
            i_raw_string,
            j_raw_string,
            new_i_match_string_position,
            new_j_match_string_position,
            i_match_string_length,
            j_match_string_length,
        );
    }
    (shift_left, text_shift_amount)
}

pub fn swap_string_info(
    i_match_string: &mut StringInfo,
    j_match_string: &mut StringInfo,
    i_raw_string: &mut StringInfo,
    j_raw_string: &mut StringInfo,
    new_i_match_string_position: usize,
    new_j_match_string_position: usize,
    i_match_string_length: usize,
    j_match_string_length: usize,
) {
    // length from where raw string begins to where match string starts.
    // use previous start and end indices for this!
    let i_raw_string_part_length = i_match_string.start_index - i_raw_string.start_index;
    let j_raw_string_part_length = j_match_string.start_index - j_raw_string.start_index;
    // i now starts where j starts and vice versa
    j_match_string.start_index = new_i_match_string_position;
    i_match_string.start_index = new_j_match_string_position;
    i_match_string.end_index = i_match_string.start_index + j_match_string_length;
    j_match_string.end_index = j_match_string.start_index + i_match_string_length;
    // // raw string and match string end at the same exact place
    i_raw_string.end_index = i_match_string.end_index;
    j_raw_string.end_index = j_match_string.end_index;
    i_raw_string.start_index = i_match_string.start_index - i_raw_string_part_length;
    j_raw_string.start_index = j_match_string.start_index - j_raw_string_part_length;
}

fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    quick_sort_colors: &Res<QuickSortColors>,
    parsed_values: &Res<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    // mut cubes_query: &mut Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
) {
    let (_, mut cube_material, _) = cubes_query
        .get_mut(parsed_values.vals[cube_index].cube_handle)
        .unwrap();

    *cube_material = quick_sort_colors
        .materials
        .get(&sort_color)
        .unwrap()
        .clone();
}

fn setup_range_color(
    previous_range: (usize, usize),
    current_range: (usize, usize),
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    parsed_values: Res<ParsedValues>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    let min = usize::min(previous_range.0, current_range.0);
    let max = usize::max(previous_range.1, current_range.1);

    for i in min..max {
        let parsed_value = &parsed_values.vals[i];
        let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();

        if i >= current_range.0 && i < current_range.1 {
            // exists in current range: give cube at this index the range color.
            let sort_color: &SortColor = {
                if i == current_range.1 - 1 {
                    &SortColor::Pivot
                } else if i == current_range.0 {
                    &SortColor::J
                } else {
                    &SortColor::Range
                }
            };
            *cube_material = quick_sort_colors.materials.get(sort_color).unwrap().clone();
        } else {
            // doesn't exist in current range, but did exist in previous one: reset this cube's
            // color to default.
            *cube_material = crate::ui::get_cube_material(
                rng_color_controls.rng_cubes_enabled,
                parsed_value.parsed_warning,
                &cube_assets,
                parsed_value.rng_color.clone(),
            );
        }
    }
}

pub fn increment_sorting(
    mut commands: Commands,
    mut sort_state: Option<ResMut<SortState>>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    mut user_text: ResMut<UserText>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
) {
    // this system runs when in sorting state and in quick sort state.
    // this system calls other functions, all of which can be systems themselves which trigger on
    // events, but I want to avoid them running in parallel at all cost (which bevy may do), so just calling them one
    // by one here.
    if let Some(sort_state) = sort_state {
        // sort already started, go to next step
        // each of these functions change the next_step to be something else.
        // compare: get out of the SortingState when sort is complete which stops this system from running

        // only call the next step once increment timer is complete
        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            log::info!(
                "sorting : timer not yet finished: {}",
                increment_timer.increment_timer.elapsed_secs_f64()
            );
            return;
        }
        log::info!(
            "sorting : timer finished: {}",
            increment_timer.increment_timer.elapsed_secs_f64()
        );

        increment_timer.increment_timer.reset();

        match sort_state.next_step {
            SortStep::SetupRange => {
                setup_range(
                    commands,
                    parsed_values.into(),
                    Some(sort_state),
                    cubes_query,
                    quick_sort_colors,
                    cube_assets,
                    rng_color_controls,
                );
            }
            SortStep::Compare => {
                compare(
                    cubes_query,
                    sort_state,
                    parsed_values.into(),
                    quick_sort_colors,
                    sort_select_set,
                );
            }
            SortStep::Swap => {
                swap(sort_state, parsed_values, cubes_query, user_text);
            }
        };
    } else {
        // sort not started: start first step and begin timer
        setup_range(
            commands,
            parsed_values.into(),
            sort_state,
            cubes_query,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
        );
        increment_timer.increment_timer.reset();
        log::info!("sort started: reseting timer");
    }
}
