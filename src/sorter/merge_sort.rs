use std::collections::{HashSet, VecDeque};

use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        entity::Entity,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    render::alpha::AlphaMode,
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
}

#[derive(Resource, Clone)]
pub struct SortState {
    halves_start_idx: (usize, usize), // left and right "array" starts
    left_right_idx: (usize, usize), // current positions within the halves (the ones being compared
    // and swapped)
    width: usize,
    next_step: SortStep,
    k: usize,
    k_length: usize,
    sweep_index: usize,
    overwritten_i: VecDeque<usize>,
    overwritten_i_set: HashSet<usize>, // just used for O(1) contains check
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    // Covered,
    Range,
    K,
}

#[derive(Resource)]
pub struct SortColors {
    pub materials: HashMap<SortColor, MeshMaterial3d<StandardMaterial>>,
}

impl FromWorld for SortColors {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            materials: HashMap::from([
                (
                    SortColor::Range,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Yellow
                        // base_color: Color::srgb_u8(238, 212, 159),
                        base_color: Color::srgba_u8(238, 212, 159, 25),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                // (
                //     SortColor::Covered,
                //     MeshMaterial3d(materials.add(StandardMaterial {
                //         // Green
                //         base_color: Color::srgba_u8(166, 218, 149, 25),
                //         alpha_mode: AlphaMode::Blend,
                //         unlit: true,
                //         ..Default::default()
                //     })),
                // ),
                (
                    SortColor::K,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Purple:
                        base_color: Color::srgba_u8(198, 160, 246, 25),
                        // Black
                        // base_color: Color::srgb_u8(24, 25, 38),
                        alpha_mode: AlphaMode::Blend,
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
                shift_halves(
                    commands,
                    Some(sort_state),
                    parsed_values,
                    cubes_query,
                    sort_colors,
                );
            }
            SortStep::IncreaseWidth => {
                increase_width(
                    sort_state,
                    parsed_values,
                    sort_select_set,
                    cubes_query,
                    sort_colors,
                );
            }
            SortStep::Compare => {
                compare_left_right(
                    sort_state,
                    parsed_values,
                    sort_colors,
                    cubes_query,
                    cube_assets,
                    user_text,
                    rng_color_controls,
                );
            } // SortStep::Swap => {
              //     swap(sort_state, parsed_values, cubes_query, user_text);
              // }
        }
    } else {
        // INFO: add 2 spaces to string as ", " is added at the beginning of strings that move from
        // first position to somewhere else and those 2 extra indices are needed during the
        // algorithm.
        // NOT needed when swapping.
        // user_text.val.push_str("  ");

        log::info!(
            "\n-=-=-=-=-=-=\nstarting text: {}",
            parsed_values.vals[..parsed_values.end_index]
                .iter()
                .map(|x| { x.converted_value.to_string() })
                .collect::<Vec<_>>()
                .join(", ")
        );

        shift_halves(
            commands,
            sort_state,
            parsed_values,
            cubes_query,
            sort_colors,
        );
    }

    increment_timer.increment_timer.reset();
}

#[allow(clippy::too_many_arguments)]
pub fn increase_width(
    mut sort_state: ResMut<SortState>,
    parsed_values: ResMut<ParsedValues>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colors: Res<SortColors>,
    // commands: Commands,
    // cube_assets: Res<CubeAssets>,
    // materials: ResMut<Assets<StandardMaterial>>,
    // cube_assets: Res<CubeAssets>,
    // rng_color_controls: Res<crate::RNGColorControls>,
    // user_text: ResMut<UserText>,
) {
    sort_state.sweep_index = 0;
    sort_state.width *= 2;
    sort_state.k_length = (sort_state.width * 2).min(parsed_values.end_index);

    log::info!("\n=-\n=-width after *2: {}\n=-\n=-", sort_state.width);
    // log::info!(
    //     "-> current text: {}",
    //     parsed_values.vals[..parsed_values.end_index]
    //         .iter()
    //         .map(|x| { x.converted_value.to_string() })
    //         .collect::<Vec<_>>()
    //         .join(", ")
    // );

    if sort_state.width > parsed_values.end_index - 1 {
        sort_select_set.set(sorter::SortState::NotSorting);
    } else {
        sort_state.halves_start_idx = (0, sort_state.width);
        sort_state.left_right_idx = sort_state.halves_start_idx;

        color_range(
            (
                sort_state.left_right_idx.0,
                sort_state.left_right_idx.1 + sort_state.width,
            ),
            sort_colors,
            &parsed_values.into(),
            cubes_query,
        );

        sort_state.next_step = SortStep::Compare;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn shift_halves(
    mut commands: Commands,
    sort_state: Option<ResMut<SortState>>,
    parsed_values: ResMut<ParsedValues>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colors: Res<SortColors>,
) {
    if let Some(mut sort_state) = sort_state {
        let first_half_start = sort_state.halves_start_idx.1 + sort_state.width;
        let second_half_start = first_half_start + sort_state.width;
        if second_half_start >= parsed_values.end_index {
            // No need to merge as there is no second half.
            // Finished with this sweep: Increase width.
            sort_state.next_step = SortStep::IncreaseWidth;
            return;
        }
        sort_state.halves_start_idx = (first_half_start, second_half_start);
        sort_state.left_right_idx = sort_state.halves_start_idx; // copied

        if (sort_state.halves_start_idx.0 + sort_state.k_length) > parsed_values.end_index {
            // length of k must be reduced to not include values that would, in total, be above end
            // index.

            log::info!(
                "k would be above end_index: {} + {} = {} > {}",
                sort_state.halves_start_idx.0,
                sort_state.k_length,
                sort_state.halves_start_idx.0 + sort_state.k_length,
                parsed_values.end_index
            );

            sort_state.k_length -=
                (sort_state.halves_start_idx.0 + sort_state.k_length) - parsed_values.end_index;

            log::info!(
                "to reach end_index {}, k should be: {}",
                parsed_values.end_index,
                sort_state.k_length,
            );
        }

        log::info!(
            "shift width: {}-{} {}-{}",
            sort_state.left_right_idx.0,
            sort_state.left_right_idx.1,
            sort_state.left_right_idx.1,
            sort_state.left_right_idx.1 + sort_state.width
        );

        color_range(
            (
                sort_state.left_right_idx.0,
                sort_state.left_right_idx.1 + sort_state.width,
            ),
            sort_colors,
            &parsed_values.into(),
            cubes_query,
        );

        sort_state.next_step = SortStep::Compare;
    } else {
        commands.insert_resource(SortState {
            width: 1,
            next_step: SortStep::Compare,
            halves_start_idx: (0, 1),
            left_right_idx: (0, 1),
            k: 0,
            k_length: 2.min(parsed_values.end_index),
            sweep_index: 0,
            overwritten_i: VecDeque::new(),
            overwritten_i_set: HashSet::new(),
        });
        color_range((0, 1), sort_colors, &parsed_values.into(), cubes_query);
    }
}

pub fn color_range(
    range: (usize, usize),
    sort_colors: Res<SortColors>,
    parsed_values: &Res<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    for i in range.0..=range.1.min(parsed_values.end_index - 1) {
        set_cube_as_within_range(
            parsed_values.vals[i].cube_handle,
            &sort_colors,
            SortColor::Range,
            &mut cubes_query,
        );
    }
}

pub fn set_cube_as_within_range(
    cube_handle: Entity,
    sort_colors: &Res<SortColors>,
    sort_color: SortColor,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    let (_, mut cube_material, _) = cubes_query.get_mut(cube_handle).unwrap();
    *cube_material = sort_colors.materials.get(&sort_color).unwrap().clone();
}

pub fn complete(mut commands: Commands, sort_state: Option<Res<SortState>>) {
    if sort_state.is_none() {
        return;
    }
    // INFO: run on exit of sorting state when MergeSort is selected as the algorithm
    commands.remove_resource::<SortState>();
}

#[allow(clippy::too_many_arguments)]
pub fn compare_left_right(
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    sort_colors: Res<SortColors>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    user_text: ResMut<UserText>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    // Overwrite parsed_value/k index with I/J. Store overwritten value for next comparison.
    let virtual_k_index = sort_state.sweep_index; // the index that must be overwritten by i/j.

    // Choose I/J to overwrite:
    let moving_index;
    let mut moving_i = false;
    ////////////////////////////////////////////////////////////////////////////////////

    if sort_state.left_right_idx.0 == sort_state.halves_start_idx.1 {
        // I half is fully scanned. Put J's ParsedValue in K Vector.
        moving_index = sort_state.left_right_idx.1;
        sort_state.left_right_idx.1 += 1;
        log::info!(
            "i complete. put j: {}. sort_state.k: {} and length: {} and len: {}",
            moving_index,
            sort_state.k,
            sort_state.k_length,
            parsed_values.end_index
        );
    } else if sort_state.left_right_idx.1
        == (sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.end_index)
    {
        // j half is fully scanned. Put I's ParsedValue in K Vector.
        moving_i = true;
        moving_index = sort_state.left_right_idx.0;
        sort_state.left_right_idx.0 += 1;
        log::info!("j complete. put i: {}", moving_index);
    } else {
        // compare I/J with overwritten value.

        // TODO: Delete this (only for logging):
        let mut delete_me_i_val = parsed_values.vals[sort_state.left_right_idx.0].converted_value;

        let i_val = {
            if let Some(over_written_i) = sort_state.overwritten_i.front() {
                delete_me_i_val = parsed_values.vals[*over_written_i].converted_value;
                parsed_values.vals[*over_written_i].sorted_position
            } else {
                parsed_values.vals[sort_state.left_right_idx.0].sorted_position
            }
        };

        log::info!(
            "{} > {}: {}",
            delete_me_i_val,
            parsed_values.vals[sort_state.left_right_idx.1].converted_value,
            delete_me_i_val > parsed_values.vals[sort_state.left_right_idx.1].converted_value,
        );

        let j_val = parsed_values.vals[sort_state.left_right_idx.1].sorted_position;

        if i_val > j_val {
            log::info!("j is smaller");
            // j is smaller. Put J's ParsedValue in K Vector.
            moving_index = sort_state.left_right_idx.1;
            sort_state.left_right_idx.1 += 1;
        } else {
            log::info!("i is smaller");
            // i is smaller. Put I's ParsedValue in K Vector.
            moving_i = true;
            moving_index = sort_state.left_right_idx.0;
            sort_state.left_right_idx.0 += 1;
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////

    swap(
        moving_index,
        virtual_k_index,
        moving_i,
        &mut sort_state,
        &mut parsed_values,
        cubes_query,
        user_text,
        rng_color_controls,
        cube_assets,
        sort_colors,
    );

    ////////////////////////////////////////////////////////////////////////////////////

    let mut delete_me_k_visualize = vec![];
    log::info!("sort_state.k: {}", sort_state.k);
    for i in 0..=sort_state.k {
        let index = sort_state.sweep_index - (sort_state.k - i);
        delete_me_k_visualize.push(parsed_values.vals[index].converted_value.to_string());
    }
    log::info!("k: {}", delete_me_k_visualize.join(", "));

    sort_state.sweep_index += 1;
    sort_state.k += 1;

    if sort_state.k == sort_state.k_length {
        // INFO: checking if k is filled: is filled when has enough values to fill out the next
        // width (if current width = 1, then k is filled when the two halves of width 1 combine to
        // crate k of width 2. if width = 2, then they combine to create 4. if 4, they combine to
        // create 8, etc.). The Next width may be larger than all values, so we check it is not
        // larger than that too (in this case, the sorting is finished).
        sort_state.next_step = SortStep::ShiftHalves;
        // cleanup:
        sort_state.k = 0;
        // else will just call compare again.
    }
}

#[allow(clippy::too_many_arguments)]
pub fn swap(
    mut moving_index: usize,
    target_index: usize,
    moving_i: bool,
    sort_state: &mut SortState,
    //
    parsed_values: &mut ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
    rng_color_controls: Res<crate::RNGColorControls>,
    cube_assets: Res<CubeAssets>,
    sort_colors: Res<SortColors>,
) {
    ////////////////////////////////////////////////////////////////////////////////////

    log::info!("\n=================\n");

    log::info!("1. Using/Moving OverwrittenI?");

    if moving_i && let Some(i_index) = sort_state.overwritten_i.pop_front() {
        log::info!("-> yes. removing and using overwrittenI as moving index");
        // If overwritten I is being used pop it off.
        moving_index = i_index;
        sort_state.overwritten_i_set.remove(&i_index);
    } else {
        log::info!("-> no");
    }
    ////////////////////////////////////////////////////////////////////////////////////
    log::info!("2. OverwrittenI contains target value?");

    if sort_state.overwritten_i_set.contains(&target_index) {
        // if sort_state.overwritten_i_set.contains(&target_index) {
        log::info!("-> yes. Updating OverwrittenI");

        // If Overwritten I already contains the index we are moving to, change its value to where
        // it will be swapped to: moving_index.
        sort_state.overwritten_i_set.remove(&target_index);
        sort_state.overwritten_i_set.insert(moving_index);

        // INFO: this may be very slow if overwritten_i has many elements in it.
        // In cases where it is too large, it may be better to use something like a
        // LinkedList and HashMap<usize, LinkedListNode> or
        // Rc<RefCell<usize>> in VecDeque and HashSet<Rc<RefCell<usize>>>
        // And then just get the usize from the HashSet/Hashmap with O(1) and update it.
        // However, this may also be slow since the CPU would have to go get these usize's from the heap.
        // Assuming that most users will only have inputs that are ~100, this will be completely
        // fine. But there is no built-in upper-end to input, so if enough users are using large inputs,
        // should update this.
        for index in sort_state.overwritten_i.iter_mut() {
            if *index == target_index {
                // change index to moving_index.
                *index = moving_index;
                break;
            }
        }
    } else {
        log::info!("-> no.");
    }

    log::info!("3. moving and target is the same");

    if moving_index == target_index {
        log::info!("-> yes. returning");

        uncolor_cube(
            moving_index,
            parsed_values,
            rng_color_controls,
            cube_assets,
            &mut cubes_query,
        );
        log::info!("{}", user_text.val);
        return;
    } else {
        log::info!("-> no. continuing");
    }

    log::info!("4. overwriting i half index?");

    if target_index < sort_state.halves_start_idx.1 {
        log::info!("-> yes. saving index it will move to in overwritten I");
        sort_state.overwritten_i_set.insert(moving_index);
        sort_state.overwritten_i.push_back(moving_index);
        color_cube(
            target_index,
            SortColor::K,
            &sort_colors,
            parsed_values,
            &mut cubes_query,
        );
    } else {
        log::info!("-> no.");
    }

    log::info!(
        "overwritten_i: {}",
        sort_state
            .overwritten_i
            .iter()
            // .map(|x| { parsed_values.vals[*x].converted_value.to_string() })
            .map(|x| { x.to_string() })
            .collect::<Vec<_>>()
            .join(", ")
    );

    ////////////////////////////////////////////////////////////////////////////////////
    uncolor_cube(
        moving_index,
        parsed_values,
        rng_color_controls,
        cube_assets,
        &mut cubes_query,
    );
    // leftward index and rightward index actually matters:
    let mut left_index = moving_index;
    let mut right_index = target_index;
    if left_index > right_index {
        left_index = target_index;
        right_index = moving_index;
    }
    crate::sorter::swap(
        left_index,
        right_index,
        parsed_values,
        cubes_query,
        user_text,
    );
}

pub fn uncolor_cube(
    cube_index: usize,
    parsed_values: &ResMut<ParsedValues>,
    rng_color_controls: Res<crate::RNGColorControls>,
    cube_assets: Res<CubeAssets>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    let parsed_value = &parsed_values.vals[cube_index];
    let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();

    *cube_material = crate::ui::get_cube_material(
        rng_color_controls.rng_cubes_enabled,
        parsed_value.parsed_warning,
        &cube_assets,
        parsed_value.rng_color.clone(),
    );
}

fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    sort_colors: &Res<SortColors>,
    parsed_values: &ResMut<ParsedValues>,
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
