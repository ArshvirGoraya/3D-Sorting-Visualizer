pub mod bubble_sort;
pub mod merge_sort;
pub mod quick_sort;
pub mod test_sort;

use bevy::prelude::*;
use core::fmt;
use std::ops::Add;
use web_time::{Duration, Instant};

use crate::ui::{ParsedValues, StringInfo, UserText};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum SortState {
    #[default]
    NotSorting,
    Sorting,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum PausedState {
    #[default]
    NotPaused,
    Paused,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum Algorithms {
    #[default]
    QuickSort,
    MergeSort,
    BubbleSort,
    // TestSort,
}
impl fmt::Display for Algorithms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithms::QuickSort => write!(f, "Quick Sort"),
            Algorithms::MergeSort => write!(f, "Merge Sort"),
            Algorithms::BubbleSort => write!(f, "Bubble Sort"),
            // Algorithms::TestSort => write!(f, "Test Sort"),
        }
    }
}
impl Algorithms {
    pub const ALL: [Algorithms; 3] = [
        Algorithms::QuickSort,
        Algorithms::MergeSort,
        Algorithms::BubbleSort,
        // Algorithms::TestSort,
    ];
}

// Swapping

pub fn swap(
    i: usize,
    j: usize,
    parsed_values: &mut ResMut<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: &mut ResMut<UserText>,
) {
    let [i_data, j_data] = parsed_values.vals.get_disjoint_mut([i, j]).unwrap();

    let [
        (mut transform_i, _, mut cube_data_i),
        (mut transform_j, _, mut cube_data_j),
    ] = cubes_query
        .get_many_mut([i_data.cube_handle, j_data.cube_handle])
        .unwrap();
    // Swap cube positions:
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
        user_text,
    );
    update_text_indices(i, j, parsed_values, shift_left, text_shift_amount);

    // TODO remove this line:
    // log::info!("{}", user_text.val);
}

pub fn first_increment(sorting_time: &mut ResMut<crate::SortingTime>) {
    // called when a sorting algorithm first starts.
    if sorting_time.enable_ui {
        sorting_time.time_start = Instant::now();
        sorting_time.sorting_time_between.clear();
        sorting_time.sorting_time_between_sum = 0;
        sorting_time.sorting_time_between_greatest = 0;
        sorting_time.sorting_time_elapsed = Duration::default();
    }
    sorting_time.visual_pause = Instant::now();
}

pub fn increment_between(sorting_time: &mut ResMut<crate::SortingTime>) {
    if !sorting_time.enable_ui {
        return;
    }
    // called between increments.
    if let Some(time_between_start) = sorting_time.sorting_time_between_start {
        let time_between_elapsed = time_between_start.elapsed().as_millis();
        sorting_time.sorting_time_between.push(time_between_elapsed);
        sorting_time.sorting_time_between_sum += time_between_elapsed;
        if time_between_elapsed > sorting_time.sorting_time_between_greatest {
            sorting_time.sorting_time_between_greatest = time_between_elapsed;
        }
    }
    sorting_time.time_elapsed = sorting_time.time_start.elapsed();
}

pub fn end_increment(sorting_time: &mut ResMut<crate::SortingTime>, increment_time_start: Instant) {
    // called at the end of a sorting algorithm's increment (not necessary after increment timer is up and
    // one of its sort steps has ran). Not on complete.

    if !sorting_time.enable_ui {
        return;
    }
    sorting_time.sorting_time_elapsed = sorting_time
        .sorting_time_elapsed
        .add(increment_time_start.elapsed());
    sorting_time.sorting_time_between_start = Some(Instant::now());
}

pub fn complete(
    sorting_time: &mut ResMut<crate::SortingTime>,
    commands: &mut Commands,
    scanned_cube: &mut ResMut<crate::ScannedCube>,
) {
    // called by all sorting algorithms when they 'complete'.

    commands.remove_resource::<crate::SortColoredCubes>();
    scanned_cube.transform = None;

    increment_between(sorting_time);

    if sorting_time.enable_ui {
        sorting_time.sorting_time_between.sort();
        log::info!(
            "sorting_time_between (sorted): {}",
            sorting_time
                .sorting_time_between
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        sorting_time.sorting_time_between_start = None;
    }
}

pub fn swap_text(
    i_match_string: &mut StringInfo,
    j_match_string: &mut StringInfo,
    i_raw_string: &mut StringInfo,
    j_raw_string: &mut StringInfo,
    user_text: &mut ResMut<UserText>,
) -> (bool, usize) {
    let mut text_shift_amount: usize = 0;
    let mut shift_left = false;
    let i_match_string_length = i_match_string.end_index - i_match_string.start_index;
    let j_match_string_length = j_match_string.end_index - j_match_string.start_index;

    // UNSAFE: rust has no way to verify if UTF8 will be valid
    // But we are not changing any characters, just re-arranging them by fixed-length, so
    // shouldn't be a problem AS LONG AS THE CHARACTERS BEING SWAPPED DO NOT OVERLAP (which
    // they don't)
    let utf8_text = unsafe { user_text.val.as_bytes_mut() };

    if i_match_string_length == j_match_string_length {
        // In-place swap if same length text
        let [i_utf8, j_utf8] = utf8_text
            .get_disjoint_mut([
                i_match_string.start_index..i_match_string.end_index,
                j_match_string.start_index..j_match_string.end_index,
            ])
            .unwrap();
        i_utf8.swap_with_slice(j_utf8);
    } else if i_match_string_length > j_match_string_length {
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
    // raw string and match string end at the same exact place
    i_raw_string.end_index = i_match_string.end_index;
    j_raw_string.end_index = j_match_string.end_index;
    i_raw_string.start_index = i_match_string.start_index - i_raw_string_part_length;
    j_raw_string.start_index = j_match_string.start_index - j_raw_string_part_length;
}

pub fn update_text_indices(
    i: usize,
    j: usize,
    parsed_values: &mut ResMut<ParsedValues>,
    shift_left: bool,
    text_shift_amount: usize,
) {
    if text_shift_amount == 0 {
        return;
    }

    // ensure left and right are not right next to each other (since in that case, no indices
    // between them exist and hence don't need to be updated).
    let mut left = i;
    let mut right = j;

    if left + 1 == right {
        // log::info!(
        //     "no need to updated in between indices since left+1 == right: {}+1 = {}",
        //     left,
        //     right
        // );
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
