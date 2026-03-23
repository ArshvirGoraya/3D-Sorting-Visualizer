use std::{
    collections::HashSet,
    ops::Add,
    time::{Duration, Instant},
};

use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    // log::info_span,
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    state::state::NextState,
    time::Time,
    transform::components::Transform,
};

use crate::{
    AudioControls, sorter,
    ui::{CubeAssets, ParsedValues, StringInfo, UserText},
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
                        // Yellow
                        base_color: Color::srgb_u8(238, 212, 159),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Pivot,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Red
                        base_color: Color::srgb_u8(237, 135, 150),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Swap,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Blue
                        base_color: Color::srgb_u8(138, 173, 244),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::I,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Purple
                        base_color: Color::srgb_u8(198, 160, 246),
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::J,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Green
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
    sub_arrays: Vec<(usize, usize)>,
    current_array: (usize, usize),
    pivot: usize,
    j: usize,
    i: isize,
    swapped_cubes: Option<(usize, usize)>,
    next_step: SortStep,
}

pub fn increment_sorting(
    commands: Commands,
    sort_state: Option<ResMut<SortState>>,
    parsed_values: ResMut<ParsedValues>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
    sort_select_set: ResMut<NextState<sorter::SortState>>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
    audio_controls: Res<AudioControls>,
    scanned_cube: ResMut<crate::ScannedCube>,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
    (mut sorting_time, mut sorting_time_isolated): (
        ResMut<crate::SortingTime>,
        ResMut<crate::SortingTimeIsolated>,
    ),
) {
    // INFO: this system runs when in sorting state and in quick sort state.
    // this system calls other functions, all of which can be systems themselves which trigger on
    // events, but I want to avoid them running in parallel at all cost (which bevy may do), so just calling them one
    // by one here.

    // let qs_span = info_span!("Quick_Sort_Increment", name = "Quick_Sort_Increment").entered();

    let isolated_time = Instant::now();

    if let Some(sort_state) = sort_state {
        // sort already started, go to next step
        // each of these functions change the next_step to be something else.
        // compare system: gets out of the SortingState when sort is complete which stops this system from running

        // only call the next step once increment timer is complete

        sorting_time.time_elapsed = sorting_time.time_start.elapsed();

        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            sorting_time_isolated.time_elapsed = sorting_time_isolated
                .time_elapsed
                .add(isolated_time.elapsed());

            return;
        }
        increment_timer.increment_timer.reset();

        match sort_state.next_step {
            SortStep::SetupRange => {
                // let span =
                // info_span!("Quick_Sort_SetupRange", name = "Quick_Sort_SetupRange").entered();
                setup_range(
                    commands,
                    parsed_values.into(),
                    Some(sort_state),
                    cubes_query,
                    quick_sort_colors,
                    cube_assets,
                    rng_color_controls,
                    scanned_cube,
                    sort_colored_cubes,
                );
                // span.exit();
            }
            SortStep::Compare => {
                // let span = info_span!("Quick_Sort_Compare", name = "Quick_Sort_Compare").entered();
                compare(
                    cubes_query,
                    sort_state,
                    parsed_values,
                    quick_sort_colors,
                    sort_select_set,
                    audio_controls,
                    commands,
                    scanned_cube,
                    cube_assets,
                    rng_color_controls,
                    sort_colored_cubes.unwrap(),
                    user_text,
                );
                // span.exit();
            } // SortStep::Swap => {
              //     swap(sort_state, parsed_values, cubes_query, user_text);
              // }
        };
    } else {
        sorting_time_isolated.time_elapsed = Duration::default();
        sorting_time.time_start = Instant::now();

        // sort not started: start first step and begin timer
        // let span = info_span!("Quick_Sort_SetupRange", name = "Quick_Sort_SetupRange").entered();
        setup_range(
            commands,
            parsed_values.into(),
            sort_state,
            cubes_query,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
            scanned_cube,
            sort_colored_cubes,
        );
        // span.exit();
        increment_timer.increment_timer.reset();
    }

    sorting_time_isolated.time_elapsed = sorting_time_isolated
        .time_elapsed
        .add(isolated_time.elapsed());
    // qs_span.exit();
}

pub fn setup_range(
    mut commands: Commands,
    parsed_values: Res<ParsedValues>,
    sort_state: Option<ResMut<SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut scanned_cube: ResMut<crate::ScannedCube>,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
) {
    if let Some(mut sort_state) = sort_state {
        let previous_array = sort_state.current_array;
        sort_state.current_array = *sort_state.sub_arrays.last_mut().unwrap();
        sort_state.pivot = sort_state.current_array.1 - 1;
        sort_state.j = sort_state.current_array.0;
        sort_state.i = (sort_state.current_array.0 as isize) - 1;

        scanned_cube.transform = Some(
            *cubes_query
                .get(parsed_values.vals[sort_state.j].cube_handle)
                .unwrap()
                .0,
        );

        // log::info!(
        //     "
        //     current array: ({}, {})
        //     j: {}
        //     i: {}
        //     ",
        //     sort_state.current_array.0,
        //     sort_state.current_array.1,
        //     sort_state.j,
        //     sort_state.i
        // );

        setup_range_color(
            previous_array,
            sort_state.current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            &cube_assets,
            &rng_color_controls,
            &mut sort_colored_cubes.unwrap(),
        );
        sort_state.next_step = SortStep::Compare;
    } else {
        let current_array = (0, parsed_values.end_index);
        commands.insert_resource(SortState {
            sub_arrays: [current_array].to_vec(),
            current_array,
            pivot: current_array.1 - 1,
            j: current_array.0,
            i: (current_array.0 as isize) - 1,
            swapped_cubes: None,
            next_step: SortStep::Compare,
        });

        scanned_cube.transform = Some(
            *cubes_query
                .get(parsed_values.vals[current_array.0].cube_handle)
                .unwrap()
                .0,
        );

        let mut sort_colored_cubes = crate::SortColoredCubes {
            cubes: HashSet::new(),
        };

        commands.insert_resource(sort_colored_cubes.clone());

        // log::info!(
        //     "
        //     current array: ({}, {})
        //     j: {}
        //     i: {}
        //     ",
        //     current_array.0,
        //     current_array.1,
        //     current_array.0,
        //     (current_array.0 as isize) - 1
        // );

        setup_range_color(
            current_array,
            current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            &cube_assets,
            &rng_color_controls,
            &mut sort_colored_cubes,
        );
    }
}

pub fn increment_j(
    sort_state: &mut ResMut<SortState>,
    quick_sort_colors: &Res<QuickSortColors>,
    parsed_values: &ParsedValues,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    commands: &mut Commands,
    audio_controls: &Res<AudioControls>,
    scanned_cube: &mut ResMut<crate::ScannedCube>,
    sort_colored_cubes: &mut ResMut<crate::SortColoredCubes>,
) {
    sort_state.j += 1;
    if sort_state.j != sort_state.pivot + 1 {
        // INFO: j may be pivot + 1: This is the condition used for detecting when a subarray is finished.

        scanned_cube.transform = Some(
            *cubes_query
                .get(parsed_values.vals[sort_state.j].cube_handle)
                .unwrap()
                .0,
        );

        color_cube(
            sort_state.j,
            SortColor::J,
            quick_sort_colors,
            parsed_values,
            cubes_query,
            sort_colored_cubes,
        );
        crate::play_audio(
            commands,
            audio_controls,
            // sort_state.j,
            parsed_values.vals[sort_state.j].sorted_position,
            parsed_values.end_index,
        );
    }
}

pub fn compare(
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    mut sort_state: ResMut<SortState>,
    parsed_values: ResMut<ParsedValues>,
    quick_sort_colors: Res<QuickSortColors>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    audio_controls: Res<AudioControls>,
    mut commands: Commands,
    mut scanned_cube: ResMut<crate::ScannedCube>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut sort_colored_cubes: ResMut<crate::SortColoredCubes>,
    user_text: ResMut<UserText>,
) {
    if let Some((i, j)) = sort_state.swapped_cubes {
        // if just swapped, increment j and color the just swapped cubes the J/"covered" color.
        color_cube(
            i,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        color_cube(
            j,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        // j must increment after swapping.
        increment_j(
            &mut sort_state,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut commands,
            &audio_controls,
            &mut scanned_cube,
            &mut sort_colored_cubes,
        );
        sort_state.swapped_cubes = None;

        // after swapping i and j: need to color the next i.
        color_cube(
            sort_state.i as usize + 1,
            SortColor::I,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );

        // should wait a step after this for visualization.
        // return;
    }
    if sort_state.j == sort_state.pivot + 1 {
        // INFO: when pivot has been reached, j and i swap and j increments by 1. That is when new subarrays
        // are created.
        // INFO: j must be 1 larger than pivot.
        // pivot has been reached, create new subarrays or complete.

        // log::info!("pivot overreached last swap complete: {}", sort_state.j);

        sort_state.sub_arrays.pop();

        // check if next array will be valid: if the subarray will have more than 1 value in it.
        let (start, end) = sort_state.current_array;

        let i = sort_state.i as usize;
        let right_array_start = i + 1;

        if end.abs_diff(right_array_start) > 1 {
            // log::info!("right array is valid: {} to {}", right_array_start, end);
            // Right array is valid
            sort_state.sub_arrays.push((right_array_start, end));
        }
        if start.abs_diff(i) > 1 {
            // log::info!("left array is valid: {} to {}", start, i);
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
    // INFO: sort by sorted_position instead of the actual value to avoid cubes that have the same
    // value being put in different positions than their sorted position dictates (relevant when
    // hovering over cubes to see their final position and especially when using positional heights)
    let pivot_value = &parsed_values.vals[sort_state.pivot].sorted_position;
    let j_value = &parsed_values.vals[sort_state.j].sorted_position;

    if sort_state.j == sort_state.pivot || pivot_value > j_value {
        // if j reached the pivot do a last swap with i and j
        // if j value is smaller than pivot, swap i and j.

        // increment i before swapping

        if sort_state.i > -1 {
            // uncolor previous i
            if sort_state.i as usize >= sort_state.current_array.0 {
                // if i is within the current array, color as "covered"
                color_cube(
                    sort_state.i as usize,
                    SortColor::J,
                    &quick_sort_colors,
                    &parsed_values,
                    &mut cubes_query,
                    &mut sort_colored_cubes,
                );
            } else {
                // if i is not within the current array (current_array.0 -1), uncolor
                uncolor_cube(
                    sort_state.i as usize,
                    &parsed_values,
                    &mut cubes_query,
                    &cube_assets,
                    &rng_color_controls,
                    &mut sort_colored_cubes,
                );
            }
        }

        sort_state.i += 1;

        if sort_state.i as usize == sort_state.j {
            // if both indices point to the same thing, no need to swap them.
            // just increment j and return.
            increment_j(
                &mut sort_state,
                &quick_sort_colors,
                &parsed_values,
                &mut cubes_query,
                &mut commands,
                &audio_controls,
                &mut scanned_cube,
                &mut sort_colored_cubes,
            );
            sort_state.next_step = SortStep::Compare;
            return;
        }

        // NOTE: if swapping, j must be incremented after swap. In that case, we don't increment j in here yet.
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
            &mut sort_colored_cubes,
        );
        color_cube(
            sort_state.j,
            SortColor::Swap,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        swap(sort_state, parsed_values, cubes_query, user_text);
        // sort_state.next_step = SortStep::Swap;
    } else {
        // if pivot_value < j_value
        // do not swap. just increment j.
        increment_j(
            &mut sort_state,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut commands,
            &audio_controls,
            &mut scanned_cube,
            &mut sort_colored_cubes,
        );
        // sort_state.next_step = SortStep::Compare;
    }
}

pub fn complete(
    sort_state: Option<Res<SortState>>,
    cube_assets: Option<Res<CubeAssets>>,
    parsed_values: Res<ParsedValues>,
    mut cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut commands: Commands,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
    mut scanned_cube: ResMut<crate::ScannedCube>,

    (mut sorting_time, mut sorting_time_isolated): (
        ResMut<crate::SortingTime>,
        ResMut<crate::SortingTimeIsolated>,
    ),
) {
    // INFO: will run at startup: runs when Quicksort is the selected algorithm
    // (which is the default) and OnEnter for NotSorting (which is the default)
    // The if statement is false on startup so wont do anything
    if let Some(sort_state) = sort_state
        && let Some(cube_assets) = cube_assets
        && let Some(mut sort_colored_cubes) = sort_colored_cubes
    {
        let isolated_time = Instant::now();

        sorting_time.time_elapsed = sorting_time.time_start.elapsed();

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
            sort_colored_cubes.cubes.remove(&i);
        }
        commands.remove_resource::<SortState>();
        commands.remove_resource::<crate::SortColoredCubes>();
        scanned_cube.transform = None;

        sorting_time_isolated.time_elapsed = sorting_time_isolated
            .time_elapsed
            .add(isolated_time.elapsed());
    }
}

pub fn swap(
    sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
) {
    sorter::swap(
        sort_state.i as usize,
        sort_state.j,
        &mut parsed_values,
        &mut cubes_query,
        user_text,
    );
    // sort_state.next_step = SortStep::Compare;
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

fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    quick_sort_colors: &Res<QuickSortColors>,
    parsed_values: &ParsedValues,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: &mut crate::SortColoredCubes,
) {
    let (_, mut cube_material, _) = cubes_query
        .get_mut(parsed_values.vals[cube_index].cube_handle)
        .unwrap();

    *cube_material = quick_sort_colors
        .materials
        .get(&sort_color)
        .unwrap()
        .clone();

    sort_colored_cubes.cubes.insert(cube_index);
}

fn uncolor_cube(
    cube_index: usize,
    parsed_values: &ParsedValues,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: &Res<CubeAssets>,
    rng_color_controls: &Res<crate::RNGColorControls>,
    sort_colored_cubes: &mut crate::SortColoredCubes,
) {
    let parsed_value = &parsed_values.vals[cube_index];
    let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();

    *cube_material = crate::ui::get_cube_material(
        rng_color_controls.rng_cubes_enabled,
        parsed_value.parsed_warning,
        cube_assets,
        parsed_value.rng_color.clone(),
    );
    sort_colored_cubes.cubes.remove(&cube_index);
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
    cube_assets: &Res<CubeAssets>,
    rng_color_controls: &Res<crate::RNGColorControls>,
    sort_colored_cubes: &mut crate::SortColoredCubes,
) {
    let min = usize::min(previous_range.0, current_range.0);
    let max = usize::max(previous_range.1, current_range.1);

    // log::info!(
    //     "
    // previous_range: {} to {}
    // current_range: {} to {}
    // - pivot: {}
    // - j: {}",
    //     previous_range.0,
    //     previous_range.1,
    //     current_range.0,
    //     current_range.1,
    //     current_range.1 - 1,
    //     current_range.0,
    // );
    //
    for i in min..max {
        if i >= current_range.0 && i < current_range.1 {
            // exists in current range: give cube at this index the range color.
            let sort_color: SortColor = {
                if i == current_range.1 - 1 {
                    SortColor::Pivot
                } else if i == current_range.0 {
                    SortColor::J
                } else {
                    SortColor::Range
                }
            };
            color_cube(
                i,
                sort_color,
                &quick_sort_colors,
                &parsed_values,
                &mut cubes_query,
                sort_colored_cubes,
            );
        } else {
            // doesn't exist in current range, but did exist in previous one: reset this cube's
            // color to default.
            uncolor_cube(
                i,
                &parsed_values,
                &mut cubes_query,
                cube_assets,
                rng_color_controls,
                sort_colored_cubes,
            );
        }
    }
    // Also color I:
    let new_range_i = current_range.0 as isize - 1;
    if new_range_i > -1 {
        let parsed_value = &parsed_values.vals[new_range_i as usize];
        let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();
        *cube_material = quick_sort_colors
            .materials
            .get(&SortColor::I)
            .unwrap()
            .clone();
    }
}
