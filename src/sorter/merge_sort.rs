use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    state::state::NextState,
    time::Time,
    transform::components::Transform,
};

use crate::{
    sorter,
    ui::{CubeAssets, ParsedValues, UserText},
};

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortStep {
    #[default]
    ShiftHalves,
    IncreaseWidth,
    Compare,
    Swap,
}

#[derive(Resource)]
pub struct SortState {
    halves_start_idx: (usize, usize), // left and right "array" starts
    left_right_idx: (usize, usize), // current positions within the halves (the ones being compared
    // and swapped)
    width: usize,
    next_step: SortStep,
    swapped_cubes: Option<(usize, usize)>,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    Covered,
    Swap,
}

#[derive(Resource)]
pub struct SortColors {
    pub materials: HashMap<SortColor, MeshMaterial3d<StandardMaterial>>,
}

impl FromWorld for SortColors {
    // TODO: Combine Sort Colors into a single resource especially since they use the same colors?
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            materials: HashMap::from([
                (
                    SortColor::Covered,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(238, 212, 159),
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
                // ()
            ]),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn increment_sorting(
    commands: Commands,
    parsed_values: ResMut<ParsedValues>,
    sort_state: Option<ResMut<SortState>>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
    sort_colors: Res<SortColors>,
    sort_select_set: ResMut<NextState<sorter::SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    user_text: ResMut<UserText>,
) {
    if let Some(sort_state) = sort_state {
        //
        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            return;
        }
        match sort_state.next_step {
            SortStep::ShiftHalves => {
                shift_halves(commands, Some(sort_state), parsed_values.into());
            }
            SortStep::IncreaseWidth => {
                increase_width(
                    sort_state,
                    parsed_values.into(),
                    sort_select_set,
                    cubes_query,
                    cube_assets,
                    rng_color_controls,
                );
            }
            SortStep::Compare => {
                compare_left_right(sort_state, parsed_values.into(), sort_colors, cubes_query);
            }
            SortStep::Swap => {
                swap(sort_state, parsed_values, cubes_query, user_text);
            }
        }
    } else {
        shift_halves(commands, sort_state, parsed_values.into());
    }

    increment_timer.increment_timer.reset();
}

pub fn increase_width(
    mut sort_state: ResMut<SortState>,
    parsed_values: Res<ParsedValues>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    uncolor_range(
        // uncolor the entire previous sweep
        (0, sort_state.halves_start_idx.1 + sort_state.width),
        &parsed_values,
        cubes_query,
        cube_assets,
        rng_color_controls,
    );

    sort_state.width *= 2;
    log::info!("width after *2: {}", sort_state.width);
    log::info!(
        "-> current text: {}",
        parsed_values.vals[..parsed_values.end_index]
            .iter()
            .map(|x| { x.converted_value.to_string() })
            .collect::<Vec<_>>()
            .join(", ")
    );

    if sort_state.width >= parsed_values.vals.len() - 1 {
        log::info!(
            "width >= parsed_values.vals.len() - 1: {} >= {}",
            sort_state.width,
            parsed_values.vals.len() - 1
        );
        sort_select_set.set(sorter::SortState::NotSorting);
    } else {
        sort_state.halves_start_idx = (0, sort_state.width);
        sort_state.left_right_idx = sort_state.halves_start_idx;
        log::info!(
            "new halves start: ({}, {})",
            sort_state.halves_start_idx.0,
            sort_state.halves_start_idx.1
        );

        sort_state.next_step = SortStep::Compare;
    }
}

pub fn shift_halves(
    mut commands: Commands,
    sort_state: Option<ResMut<SortState>>,
    parsed_values: Res<ParsedValues>,
) {
    if let Some(mut sort_state) = sort_state {
        let first_half_start = sort_state.halves_start_idx.1 + sort_state.width;
        let second_half_start = first_half_start + sort_state.width;
        if second_half_start >= parsed_values.vals.len() {
            // No need to merge as there is no second half.
            // Finished with this sweep: Increase width.
            sort_state.next_step = SortStep::IncreaseWidth;
            return;
        }
        // let previous_range_start = sort_state.halves_start_idx.0;
        sort_state.halves_start_idx = (first_half_start, second_half_start);
        log::info!(
            "new halves start: ({}, {})",
            sort_state.halves_start_idx.0,
            sort_state.halves_start_idx.1
        );
        //
        sort_state.left_right_idx = sort_state.halves_start_idx; // copied

        // color_halves(
        //     previous_range_start,
        //     first_half_start,
        //     second_half_start,
        //     second_half_start + sort_state.width,
        //     None,
        //     sort_colors,
        //     parsed_values,
        //     cubes_query,
        // );
        sort_state.next_step = SortStep::Compare;
    } else {
        commands.insert_resource(SortState {
            width: 1,
            next_step: SortStep::Compare,
            halves_start_idx: (0, 1),
            left_right_idx: (0, 1),
            swapped_cubes: None,
        });
        log::info!("new halves start: ({}, {})", 0, 1);
    }
}

pub fn swap(
    mut sort_state: ResMut<SortState>,
    parsed_values: ResMut<ParsedValues>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
) {
    sorter::swap(
        sort_state.left_right_idx.0,
        sort_state.left_right_idx.1,
        parsed_values,
        cubes_query,
        user_text,
    );
    sort_state.next_step = SortStep::Compare;
}

pub fn complete(mut commands: Commands, sort_state: Option<Res<SortState>>) {
    if sort_state.is_none() {
        return;
    }
    // INFO: run on exit of sorting state when MergeSort is selected as the algorithm
    commands.remove_resource::<SortState>();
}

pub fn i_finished(sort_state: &ResMut<SortState>) -> bool {
    sort_state.left_right_idx.0 == sort_state.halves_start_idx.1
}
pub fn j_finished(sort_state: &ResMut<SortState>, parsed_values: Res<ParsedValues>) -> bool {
    sort_state.left_right_idx.1 == sort_state.halves_start_idx.1 + sort_state.width + 1
        || sort_state.left_right_idx.1 == parsed_values.vals.len()
}

pub fn compare_left_right(
    mut sort_state: ResMut<SortState>,
    parsed_values: Res<ParsedValues>,
    sort_colors: Res<SortColors>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    if let Some((i, j)) = sort_state.swapped_cubes {
        // remove swap color from swapped cubes.
        color_cubes(
            Vec::from([i, j]),
            SortColor::Covered,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        sort_state.swapped_cubes = None;
        sort_state.left_right_idx.1 += 1;
        if j_finished(&sort_state, parsed_values) {
            log::info!("j finished");
            sort_state.next_step = SortStep::ShiftHalves;
            // else will just call compare again.
        } else {
            log::info!("j not finished");
        }
        return;
    }
    let i_val = parsed_values.vals[sort_state.left_right_idx.0].sorted_position;
    let j_val = parsed_values.vals[sort_state.left_right_idx.1].sorted_position;
    log::info!(
        "[{}]{} > [{}]{}",
        sort_state.left_right_idx.0,
        parsed_values.vals[sort_state.left_right_idx.0].converted_value,
        sort_state.left_right_idx.1,
        parsed_values.vals[sort_state.left_right_idx.1].converted_value,
    );
    // TODO: fix comparisons: swapping messes things up!
    if i_val > j_val {
        color_cubes(
            Vec::from([sort_state.left_right_idx.0, sort_state.left_right_idx.1]),
            SortColor::Swap,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        log::info!("swapping");
        sort_state.swapped_cubes = Some((sort_state.left_right_idx.0, sort_state.left_right_idx.1));
        sort_state.next_step = SortStep::Swap;
        // INFO: after swap: will increment j. And check if j is finished and call ShiftHalves if
        // so (check else statement as i does the equivalent without swapping).
    } else {
        // no swap.
        color_cubes(
            Vec::from([sort_state.left_right_idx.0, sort_state.left_right_idx.1]),
            SortColor::Covered,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
        log::info!("incrementing i");
        sort_state.left_right_idx.0 += 1;
        if i_finished(&sort_state) {
            log::info!("i finished");
            sort_state.next_step = SortStep::ShiftHalves;
            // INFO: else will just compare left_right again.
        } else {
            log::info!("i not finished");
        }
    }
}

fn color_cubes(
    cube_indices: Vec<usize>,
    sort_color: SortColor,
    sort_colors: &Res<SortColors>,
    parsed_values: &Res<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    for i in cube_indices {
        color_cube(i, sort_color, sort_colors, parsed_values, cubes_query);
    }
}

fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    sort_colors: &Res<SortColors>,
    parsed_values: &Res<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    let (_, mut cube_material, _) = cubes_query
        .get_mut(parsed_values.vals[cube_index].cube_handle)
        .unwrap();

    *cube_material = sort_colors.materials.get(&sort_color).unwrap().clone();
}

pub fn uncolor_range(
    range: (usize, usize),
    parsed_values: &Res<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    log::info!(
        "uncoloring: {}..={}",
        range.0,
        range.1.min(parsed_values.vals.len() - 1)
    );

    for i in range.0..=range.1.min(parsed_values.vals.len() - 1) {
        let parsed_value = &parsed_values.vals[i];
        let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();
        *cube_material = crate::ui::get_cube_material(
            rng_color_controls.rng_cubes_enabled,
            parsed_value.parsed_warning,
            &cube_assets,
            parsed_value.rng_color.clone(),
        );
    }
}

// pub fn color_halves(
//     previous_range_start: usize,
//     new_range_start: usize,
//     second_half_start: usize,
//     new_range_end: usize,
//     color: Option<SortColor>,
//     sort_colors: Res<SortColors>,
//     parsed_values: Res<ParsedValues>,
//     mut cubes_query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
// ) {
//     // TODO: another function that clears the previous range when width increments
//
//     // could also color the next range in the same for loop if:
//     // we know that the for loops are right next to each other or overlap.
//
//     for i in previous_range_start..=new_range_end {
//         let parsed_value = &parsed_values.vals[i];
//         let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();
//
//         if i >= new_range_start {
//             match i {
//                 new_range_start => {
//                     //
//                 }
//                 second_half_start => {
//                     //
//                 }
//                 _ => {
//                     //
//                 }
//             }
//             // color as selected.
//         } else {
//             // uncolor
//         }
//         //
//     }
// }

pub fn merge_halves() {
    //
}
