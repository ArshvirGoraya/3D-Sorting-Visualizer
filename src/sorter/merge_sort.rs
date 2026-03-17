use std::collections::{HashSet, VecDeque};

use bevy::{
    asset::Assets,
    color::{Alpha, Color},
    ecs::{
        entity::Entity,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    math::Vec3,
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    render::alpha::AlphaMode,
    state::state::NextState,
    time::Time,
    transform::components::Transform,
};

use crate::{
    sorter,
    ui::{CubeAssets, ParsedValue, ParsedValues, StringInfo, UserText},
};

const DEFAULT_Z: f32 = 1.0;
const SELECTION_Z: f32 = 3.5;
// const SELECTION_Z: f32 = 100.0;
const DEFAULT_ALPHA: f32 = 1.0;
const SELECTION_ALPHA: f32 = 0.5;

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortStep {
    #[default]
    ShiftHalves,
    IncreaseWidth,
    Compare,
}

#[derive(Clone)]
pub struct KValue {
    parsed_value_clone: ParsedValue,
    k_handle: Entity,
    raw_string_text: String,
    virtual_k_index: usize,
    original_index: usize,
}

#[derive(Clone)]
pub struct OverWrittenI {
    parsed_value: ParsedValue,
    raw_text: String,
}

#[derive(Resource, Clone)]
pub struct SortState {
    halves_start_idx: (usize, usize), // left and right "array" starts
    left_right_idx: (usize, usize), // current positions within the halves (the ones being compared
    // and swapped)
    width: usize,
    next_step: SortStep,
    // k: Vec<KValue>,
    k: usize,
    k_length: usize,
    sweep_index: usize,
    overwritten_i: VecDeque<usize>,
    overwritten_i_set: HashSet<usize>,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    Covered,
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
                        base_color: Color::srgba_u8(238, 212, 159, 25),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Covered,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Green
                        base_color: Color::srgba_u8(166, 218, 149, 25),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::K,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Black
                        base_color: Color::srgb_u8(24, 25, 38),
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
    mut user_text: ResMut<UserText>,
    materials: ResMut<Assets<StandardMaterial>>,
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
                    // materials,
                    sort_colors,
                    cube_assets,
                    rng_color_controls,
                    user_text,
                );
            }
            SortStep::IncreaseWidth => {
                increase_width(
                    commands,
                    sort_state,
                    parsed_values,
                    sort_select_set,
                    cubes_query,
                    // cube_assets,
                    // rng_color_controls,
                    materials,
                    sort_colors,
                    cube_assets,
                    rng_color_controls,
                    user_text,
                );
            }
            SortStep::Compare => {
                compare_left_right(
                    commands,
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
            // materials,
            sort_colors,
            cube_assets,
            rng_color_controls,
            user_text,
        );
    }

    increment_timer.increment_timer.reset();
}

pub fn increase_width(
    mut commands: Commands,
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    // cube_assets: Res<CubeAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sort_colors: Res<SortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    user_text: ResMut<UserText>,
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
        // let previous_halves = sort_state.halves_start_idx;
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

pub fn shift_halves(
    mut commands: Commands,
    sort_state: Option<ResMut<SortState>>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    sort_colors: Res<SortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    user_text: ResMut<UserText>,
) {
    if let Some(mut sort_state) = sort_state {
        // sort_state.over_written_value = None;
        // overwrite_values(
        //     commands,
        //     &mut sort_state,
        //     &mut parsed_values,
        //     user_text,
        //     &mut cubes_query,
        //     &cube_assets,
        //     &rng_color_controls,
        // );

        let first_half_start = sort_state.halves_start_idx.1 + sort_state.width;
        let second_half_start = first_half_start + sort_state.width;
        if second_half_start >= parsed_values.end_index {
            // No need to merge as there is no second half.
            // Finished with this sweep: Increase width.
            sort_state.next_step = SortStep::IncreaseWidth;
            return;
        }
        // let previous_halves = sort_state.halves_start_idx;
        sort_state.halves_start_idx = (first_half_start, second_half_start);
        sort_state.left_right_idx = sort_state.halves_start_idx; // copied

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
    let (mut cube_transform, mut cube_material, _) = cubes_query.get_mut(cube_handle).unwrap();
    // cube_transform.scale.z = SELECTION_Z;
    *cube_material = sort_colors.materials.get(&sort_color).unwrap().clone();
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

pub fn complete(mut commands: Commands, sort_state: Option<Res<SortState>>) {
    if sort_state.is_none() {
        return;
    }
    // INFO: run on exit of sorting state when MergeSort is selected as the algorithm
    commands.remove_resource::<SortState>();
}

// pub fn overwrite_value(
//     moving_i: bool,
//     moving_index: usize,
//     mut parsed_values: &mut ResMut<ParsedValues>,
//     mut sort_state: &mut ResMut<SortState>,
//     mut user_text: ResMut<UserText>,
//     mut cubes_query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
//     cube_assets: Res<CubeAssets>,
//     rng_color_controls: Res<crate::RNGColorControls>,
// ) {
//     let target_index = sort_state.sweep_index;
//
//     if moving_index == target_index {
//         // TODO: is this valid?
//         log::info!("moving index and target index are the same. do nothing.");
//         // Don't need to overwrite anything if the same index.
//         return;
//     }
//     // Store where left string ends for text replacement:
//     let left_string_end = {
//         if target_index > 0 {
//             parsed_values
//                 .vals
//                 .get(target_index - 1)
//                 .unwrap()
//                 .raw_string
//                 .end_index
//         } else {
//             0
//         }
//     };
//     //////////////////////////////////////////////////////////////////////////
//     let mut moving_overwritten_i = false;
//     let mut target_parsed_value; // necessary to have a lasting &mut target_val
//     let mut target_val: &mut ParsedValue;
//     let mut moving_val: &mut ParsedValue;
//     let mut moving_text;
//
//     if moving_i && let Some(overwritten_i) = &mut sort_state.overwritten_i {
//         moving_overwritten_i = true;
//         target_parsed_value = parsed_values.vals.get_mut(target_index).unwrap();
//         target_val = &mut target_parsed_value;
//         moving_val = &mut overwritten_i.parsed_value;
//         moving_text = overwritten_i.raw_text.clone();
//     } else {
//         [target_val, moving_val] = parsed_values
//             .vals
//             .get_disjoint_mut([target_index, moving_index])
//             .unwrap();
//         moving_text = user_text.val
//             [moving_val.raw_string.start_index..moving_val.raw_string.end_index]
//             .to_string();
//     }
//
//     log::info!("moving i: {}", moving_i);
//     log::info!("moving overwritten i: {}", moving_overwritten_i);
//
//     log::info!(
//         "overwriting target_val: [{}]{} with moving_val: [{}]{}",
//         target_index,
//         target_val.converted_value,
//         moving_index,
//         moving_val.converted_value,
//     );
//     //////////////////////////////////////////////////////////////////////////
//     // INFO: these are overwritten more carefully later: target_val.raw_string, target_val.matched_string
//     target_val.converted_value = moving_val.converted_value;
//     target_val.sorted_position = moving_val.sorted_position;
//     target_val.parsed_warning = moving_val.parsed_warning;
//     target_val.rng_color = moving_val.rng_color.clone();
//     target_val.cube_handle = moving_val.cube_handle;
//     //////////////////////////////////////////////////////////////////////////
//     // Set Visuals.
//     let (mut transform_p, mut cube_mat_p, mut cube_data_p) =
//         cubes_query.get_mut(target_val.cube_handle).unwrap();
//
//     cube_data_p.index = target_index;
//     transform_p.translation.x = transform_p.scale.x * (target_index as f32);
//     *cube_mat_p = crate::ui::get_cube_material(
//         rng_color_controls.rng_cubes_enabled,
//         target_val.parsed_warning,
//         &cube_assets,
//         target_val.rng_color.clone(),
//     );
//     //////////////////////////////////////////////////////////////////////////
//     overwrite_text(
//         moving_overwritten_i,
//         left_string_end,
//         target_index,
//         moving_index,
//         // parsed_values,
//         target_val,
//         moving_val,
//         moving_text,
//         user_text,
//     );
//
//     if moving_overwritten_i {
//         sort_state.overwritten_i = None;
//     }
// }
//
// pub fn overwrite_text(
//     moving_overwritten_i: bool,
//     left_string_end: usize,
//     target_index: usize,
//     moving_index: usize,
//     // parsed_values: &mut ParsedValues,
//     target_val: &mut ParsedValue,
//     moving_val: &mut ParsedValue,
//     mut moving_text: String,
//     mut user_text: ResMut<UserText>,
// ) {
//     // target_val.raw_string.start_index = moving_val.raw_string.start_index;
//     target_val.raw_string.start_index = left_string_end;
//
//     let mut end_length = moving_val.raw_string.end_index - moving_val.raw_string.start_index;
//     let mut matched_length =
//         moving_val.matched_string.start_index - moving_val.raw_string.start_index;
//
//     //////////////////////////////////////////////////////////////////////////
//     // clear target string.
//     let text_bytes = unsafe { user_text.val.as_bytes_mut() };
//     text_bytes[target_val.raw_string.start_index..target_val.raw_string.end_index].fill(b' ');
//
//     // clear moving string text if its not from overwritten_i
//     if !moving_overwritten_i {
//         text_bytes[moving_val.raw_string.start_index..moving_val.raw_string.end_index].fill(b' ');
//     }
//
//     //////////////////////////////////////////////////////////////////////////
//     // Add or remove ", " from moving string.
//     let moving_to_first = target_index == 0;
//     let moving_from_first = moving_index == 0;
//
//     if moving_to_first && moving_text.starts_with(", ") {
//         moving_text = moving_text[2..].to_string();
//         matched_length -= 2;
//         end_length -= 2;
//     }
//     if moving_from_first {
//         moving_text = String::from(", ") + &moving_text;
//         matched_length += 2;
//         end_length += 2;
//     }
//     target_val.raw_string.end_index = target_val.raw_string.start_index + end_length;
//     target_val.matched_string.start_index = target_val.raw_string.start_index + matched_length;
//     target_val.matched_string.end_index = target_val.raw_string.end_index;
//     // //////////////////////////////////////////////////////////////////////////
//     // Update User text
//     text_bytes[target_val.raw_string.start_index..target_val.raw_string.end_index]
//         .copy_from_slice(moving_text.as_bytes());
//
//     log::info!("user text after update: \n{}", user_text.val)
// }

pub fn compare_left_right(
    mut commands: Commands,
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    sort_colors: Res<SortColors>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    user_text: ResMut<UserText>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    // Overwrite parsed_value/k index with I/J. Store overwritten value for next comparison.
    let virtual_k_index = sort_state.sweep_index; // the index that must be overwrited by i/j.

    // Choose I/J to overwrite:
    let moving_index;
    let mut moving_i = false;
    ////////////////////////////////////////////////////////////////////////////////////

    if sort_state.left_right_idx.0 == sort_state.halves_start_idx.1 {
        // I half is fully scanned. Put J's ParsedValue in K Vector.
        moving_index = sort_state.left_right_idx.1;
        sort_state.left_right_idx.1 += 1;
        log::info!("i complete. put j.");
    } else if sort_state.left_right_idx.1
        == (sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.end_index)
    {
        // j half is fully scanned. Put I's ParsedValue in K Vector.
        moving_i = true;
        moving_index = sort_state.left_right_idx.0;
        sort_state.left_right_idx.0 += 1;
        log::info!("j complete. put i.");
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
        parsed_values,
        cubes_query,
        user_text,
    );

    ////////////////////////////////////////////////////////////////////////////////////
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

pub fn swap(
    mut moving_index: usize,
    target_index: usize,
    moving_i: bool,
    sort_state: &mut SortState,
    //
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    mut user_text: ResMut<UserText>,
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
        // And then just get the usize from the Hashset/Hashmap with O(1) and update it.
        // However, this may also be slow since the CPU would have to go get these usizes from the heap.
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

// pub fn compare_left_right(
//     mut commands: Commands,
//     mut sort_state: ResMut<SortState>,
//     mut parsed_values: ResMut<ParsedValues>,
//     sort_colors: Res<SortColors>,
//     mut cubes_query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
//     cube_assets: Res<CubeAssets>,
//     user_text: ResMut<UserText>,
//     rng_color_controls: Res<crate::RNGColorControls>,
// ) {
//     // Overwrite parsed_value/k index with I/J. Store overwritten value for next comparison.
//     let virtual_k_index = sort_state.sweep_index; // the index that must be overwrited by i/j.
//
//     // Choose I/J to overwrite:
//     let moving_index;
//     let mut moving_i = false;
//
//     if sort_state.left_right_idx.0 == sort_state.halves_start_idx.1 {
//         // I half is fully scanned. Put J's ParsedValue in K Vector.
//         moving_index = sort_state.left_right_idx.1;
//         sort_state.left_right_idx.1 += 1;
//     } else if sort_state.left_right_idx.1
//         == (sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.end_index)
//     {
//         // j half is fully scanned. Put I's ParsedValue in K Vector.
//         moving_i = true;
//         moving_index = sort_state.left_right_idx.0;
//         sort_state.left_right_idx.0 += 1;
//     } else {
//         // compare I/J with overwritten value.
//         let i_val = {
//             if let Some(over_written_i) = &sort_state.overwritten_i {
//                 log::info!(
//                     "overwritten i in comparison: {}",
//                     over_written_i.parsed_value.converted_value
//                 );
//                 over_written_i.parsed_value.sorted_position
//             } else {
//                 log::info!(
//                     "i in comparison: {}",
//                     parsed_values.vals[sort_state.left_right_idx.0].converted_value
//                 );
//                 parsed_values.vals[sort_state.left_right_idx.0].sorted_position
//             }
//         };
//         log::info!(
//             "j in comparison: {}",
//             parsed_values.vals[sort_state.left_right_idx.1].converted_value
//         );
//
//         let j_val = parsed_values.vals[sort_state.left_right_idx.1].sorted_position;
//
//         if i_val > j_val {
//             log::info!("j is smaller");
//             // j is smaller. Put J's ParsedValue in K Vector.
//             moving_index = sort_state.left_right_idx.1;
//             sort_state.left_right_idx.1 += 1;
//             // store i (about to be overwritten) if not ALREADY stored:
//             if sort_state.overwritten_i.is_none() {
//                 let parsed_value = parsed_values.vals[sort_state.left_right_idx.0].clone();
//                 log::info!("storing i: {}", parsed_value.converted_value);
//                 sort_state.overwritten_i = Some(OverWrittenI {
//                     raw_text: user_text.val
//                         [parsed_value.raw_string.start_index..parsed_value.raw_string.end_index]
//                         .to_string(),
//                     parsed_value,
//                 })
//             }
//         } else {
//             log::info!("i is smaller");
//             // i is smaller. Put I's ParsedValue in K Vector.
//             moving_i = true;
//             moving_index = sort_state.left_right_idx.0;
//             sort_state.left_right_idx.0 += 1;
//         }
//     }
//     ////////////////////////////////////////////////////////////////////////////////////
//     // Overwrite K value.
//     overwrite_value(
//         moving_i,
//         moving_index,
//         &mut parsed_values,
//         &mut sort_state,
//         user_text,
//         cubes_query,
//         cube_assets,
//         rng_color_controls,
//     );
//
//     ////////////////////////////////////////////////////////////////////////////////////
//     sort_state.sweep_index += 1;
//     sort_state.k += 1;
//     if sort_state.k == sort_state.k_length {
//         // INFO: checking if k is filled: is filled when has enough values to fill out the next
//         // width (if current width = 1, then k is filled when the two halves of width 1 combine to
//         // crate k of width 2. if width = 2, then they combine to create 4. if 4, they combine to
//         // create 8, etc.). The Next width may be larger than all values, so we check it is not
//         // larger than that too (in this case, the sorting is finished).
//         sort_state.next_step = SortStep::ShiftHalves;
//         // cleanup:
//         sort_state.k = 0;
//         // else will just call compare again.
//     }
// }
