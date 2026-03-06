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
    // TODO: remove this:
    original_index: usize,
}

#[derive(Resource, Clone)]
pub struct SortState {
    halves_start_idx: (usize, usize), // left and right "array" starts
    left_right_idx: (usize, usize), // current positions within the halves (the ones being compared
    // and swapped)
    width: usize,
    next_step: SortStep,
    k: Vec<KValue>,
    sweep_index: usize,
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
    user_text: ResMut<UserText>,
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
                    parsed_values.into(),
                    sort_colors,
                    cubes_query,
                    cube_assets,
                    user_text.into(),
                );
            } // SortStep::Swap => {
              //     swap(sort_state, parsed_values, cubes_query, user_text);
              // }
        }
    } else {
        log::info!(
            "-> current text: {}",
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
        // overwrite_values(
        //     commands,
        //     &mut sort_state,
        //     &mut parsed_values,
        //     user_text,
        //     &mut cubes_query,
        //     &cube_assets,
        //     &rng_color_controls,
        // );
        // color_halves(
        //     // uncolors previous halves only.
        //     Some(sort_state.halves_start_idx),
        //     None,
        //     sort_state.width,
        //     parsed_values.into(),
        //     sort_colors,
        //     cube_assets,
        //     cubes_query,
        //     rng_color_controls,
        // );

        sort_select_set.set(sorter::SortState::NotSorting);
    } else {
        // let previous_halves = sort_state.halves_start_idx;
        sort_state.halves_start_idx = (0, sort_state.width);
        sort_state.left_right_idx = sort_state.halves_start_idx;
        // log::info!(
        //     "new halves start: ({}, {})",
        //     sort_state.halves_start_idx.0,
        //     sort_state.halves_start_idx.1
        // );

        color_range(
            (
                sort_state.left_right_idx.0,
                sort_state.left_right_idx.1 + sort_state.width,
            ),
            sort_colors,
            &parsed_values.into(),
            cubes_query,
        );
        // color_halves(
        //     // uncolors the previous halves and colors the new halves
        //     Some(previous_halves),
        //     Some(sort_state.halves_start_idx),
        //     sort_state.width,
        //     parsed_values.into(),
        //     sort_colors,
        //     cube_assets,
        //     cubes_query,
        //     rng_color_controls,
        // );

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
        overwrite_values(
            commands,
            &mut sort_state,
            &mut parsed_values,
            user_text,
            &mut cubes_query,
            &cube_assets,
            &rng_color_controls,
        );

        let first_half_start = sort_state.halves_start_idx.1 + sort_state.width;
        let second_half_start = first_half_start + sort_state.width;
        if second_half_start >= parsed_values.vals.len() {
            // No need to merge as there is no second half.
            // Finished with this sweep: Increase width.
            sort_state.next_step = SortStep::IncreaseWidth;
            return;
        }
        // let previous_halves = sort_state.halves_start_idx;
        sort_state.halves_start_idx = (first_half_start, second_half_start);
        // log::info!(
        //     "new halves start: ({}, {})",
        //     sort_state.halves_start_idx.0,
        //     sort_state.halves_start_idx.1
        // );
        //
        sort_state.left_right_idx = sort_state.halves_start_idx; // copied
        //
        // color_halves(
        //     // uncolors the previous halves and colors the new halves
        //     Some(previous_halves),
        //     Some(sort_state.halves_start_idx),
        //     sort_state.width,
        //     parsed_values.into(),
        //     sort_colors,
        //     cube_assets,
        //     cubes_query,
        //     rng_color_controls,
        // );
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
            k: Vec::new(),
            sweep_index: 0,
        });
        // log::info!("new halves start: ({}, {})", 0, 1);
        color_range((0, 1), sort_colors, &parsed_values.into(), cubes_query);
        // color_halves(
        //     // colors new halves only
        //     None,
        //     Some((0, 1)),
        //     1,
        //     parsed_values.into(),
        //     sort_colors,
        //     cube_assets,
        //     cubes_query,
        //     rng_color_controls,
        // );
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
    // log::info!(
    //     "coloring as selected: {}..={}",
    //     range.0,
    //     range.1.min(parsed_values.vals.len() - 1)
    // );

    for i in range.0..=range.1.min(parsed_values.vals.len() - 1) {
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
    cube_transform.scale.z = SELECTION_Z;
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

// pub fn color_halves(
//     previous_halves: Option<(usize, usize)>,
//     new_halves: Option<(usize, usize)>,
//     width: usize,
//     parsed_values: Res<ParsedValues>,
//     sort_colors: Res<SortColors>,
//     cube_assets: Res<CubeAssets>,
//     mut cubes_query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
//     rng_color_controls: Res<crate::RNGColorControls>,
//     // materials: &mut ResMut<Assets<StandardMaterial>>,
// ) {
//     let range = (
//         previous_halves.unwrap_or(new_halves.unwrap()).0, // starts at previous_range_ start if exists or new_range_start.
//         (new_halves.unwrap_or(previous_halves.unwrap()).1 + width)
//             .min(parsed_values.vals.len() - 1), // ends at new_range_end if
//                                                           // exists or previous_range_end.
//     );
//     let uncolor_previous_range = previous_halves.is_some();
//
//     for i in range.0..=range.1 {
//         let parsed_value = &parsed_values.vals[i];
//         let (mut cube_transform, mut cube_material, _) =
//             cubes_query.get_mut(parsed_value.cube_handle).unwrap();
//         if uncolor_previous_range && i <= previous_halves.unwrap().1 {
//             cube_transform.scale.z = DEFAULT_Z;
//             *cube_material = crate::ui::get_cube_material(
//                 rng_color_controls.rng_cubes_enabled,
//                 parsed_value.parsed_warning,
//                 &cube_assets,
//                 parsed_value.rng_color.clone(),
//             );
//         } else {
//             cube_transform.scale.z = SELECTION_Z;
//             *cube_material = sort_colors
//                 .materials
//                 .get(&SortColor::Range)
//                 .unwrap()
//                 .clone();
//         }
//         // let mat = materials.get_mut(&cube_material.0).unwrap();
//         // mat.base_color.set_alpha(new_alpha);
//     }
// }

pub fn complete(mut commands: Commands, sort_state: Option<Res<SortState>>) {
    if sort_state.is_none() {
        return;
    }
    // INFO: run on exit of sorting state when MergeSort is selected as the algorithm
    commands.remove_resource::<SortState>();
}

pub fn overwrite_values(
    mut commands: Commands,
    sort_state: &mut ResMut<SortState>,
    mut parsed_values: &mut ResMut<ParsedValues>,
    mut user_text: ResMut<UserText>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: &Res<CubeAssets>,
    rng_color_controls: &Res<crate::RNGColorControls>,
) {
    // TODO: if going into the same index, just make the cube normal size/color no need to do
    // anything with text stuff.

    log::info!(
        "overwrite: \n\tparsed section:\t[{}]
        k array:\t[{}]",
        parsed_values.vals[sort_state.k[0].virtual_k_index
            ..=sort_state.k[sort_state.k.len() - 1].virtual_k_index]
            .iter()
            .map(|parsed_value| parsed_value.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        sort_state
            .k
            .iter()
            .map(|k_value| k_value.parsed_value_clone.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    for k_value in &sort_state.k {
        let mut left_string_end_match = 0;
        if k_value.virtual_k_index > 0 {
            left_string_end_match = parsed_values
                .vals
                .get(k_value.virtual_k_index - 1)
                .unwrap()
                .raw_string
                .end_index;
        }

        // Get the parsed_value that must be replaced by cloned parsed_value:
        let mut parsed_value = parsed_values.vals.get_mut(k_value.virtual_k_index).unwrap();

        log::info!(
            "->replacing index [{}]{} with index [{}]{}",
            k_value.virtual_k_index,
            parsed_value.converted_value,
            k_value.original_index,
            k_value.parsed_value_clone.converted_value
        );

        parsed_value.converted_value = k_value.parsed_value_clone.converted_value;
        parsed_value.sorted_position = k_value.parsed_value_clone.sorted_position;
        parsed_value.parsed_warning = k_value.parsed_value_clone.parsed_warning;
        parsed_value.rng_color = k_value.parsed_value_clone.rng_color.clone();
        parsed_value.cube_handle = k_value.parsed_value_clone.cube_handle;

        //////////////////////////////////////////////////////////////////////////
        // Visually, the cube at parsed_value_index is transparent and potentially not at the location of
        // target_index (which the k_value.k_handle cube is in). Visually set it so that it looks normal and where it should be.

        let [
            (mut transform_p, mut cube_mat_p, mut cube_data_p),
            (transform_k, _, _),
        ] = cubes_query
            .get_many_mut([parsed_value.cube_handle, k_value.k_handle])
            .unwrap();
        // Size and Position

        // log::info!(
        //     "setting cube at [{}] to k cube [{}]",
        //     cube_data_p.index,
        //     k_value.virtual_k_index,
        // );

        transform_p.translation = transform_k.translation;
        transform_p.scale = transform_k.scale;

        // Material:
        *cube_mat_p = crate::ui::get_cube_material(
            rng_color_controls.rng_cubes_enabled,
            parsed_value.parsed_warning,
            cube_assets,
            parsed_value.rng_color.clone(),
        );

        // Cube index (can also set it to cube_data_k.index but thats the same value as
        // virtual_k_value)
        cube_data_p.index = k_value.virtual_k_index;

        // K cube no longer needed:
        commands.entity(k_value.k_handle).despawn();

        //////////////////////////////////////////////////////////////////////////
        // Change raw_string and matched_string

        // TODO: if string moving from first to somewhere that isn't first, add a ", " to its
        // beginning.
        // TODO: if string moving from not-first to first, remove the ", " from its beginning.

        let end_length = k_value.parsed_value_clone.raw_string.end_index
            - k_value.parsed_value_clone.raw_string.start_index;
        let match_length = k_value.parsed_value_clone.matched_string.start_index
            - k_value.parsed_value_clone.raw_string.start_index;

        parsed_value.raw_string.start_index = left_string_end_match;
        parsed_value.raw_string.end_index = parsed_value.raw_string.start_index + end_length;
        parsed_value.matched_string.end_index = parsed_value.raw_string.end_index;
        parsed_value.matched_string.start_index =
            parsed_value.raw_string.start_index + match_length;

        //////////////////////////////////////////////////////////////////////////
        // Update User text
        unsafe {
            // safe (slower) version: user_text.val.replace_range
            let text_bytes = user_text.val.as_bytes_mut();
            let clone_bytes = k_value.raw_string_text.as_bytes();
            text_bytes[parsed_value.raw_string.start_index..parsed_value.raw_string.end_index]
                .copy_from_slice(clone_bytes);
        }
    }

    // log::info!(
    //     "\nparsed section after overwrite:[{}]",
    //     parsed_values.vals[sort_state.k[0].virtual_k_index
    //         ..=sort_state.k[sort_state.k.len() - 1].virtual_k_index]
    //         .iter()
    //         .map(|parsed_value| parsed_value.converted_value.to_string())
    //         .collect::<Vec<_>>()
    //         .join(", "),
    // );

    log::info!(
        "\nparsed values after overwrite values:[{}]",
        parsed_values
            .vals
            .iter()
            .map(|parsed_value| parsed_value.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );

    sort_state.k.clear();
}

pub fn compare_left_right(
    mut commands: Commands,
    mut sort_state: ResMut<SortState>,
    parsed_values: Res<ParsedValues>,
    sort_colors: Res<SortColors>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    cube_assets: Res<CubeAssets>,
    user_text: Res<UserText>,
) {
    // Choose what I/J to add to K.

    log::info!(
        "i half: [{}..{}][{}], j half: [{}..{}][{}]",
        sort_state.left_right_idx.0,
        sort_state.halves_start_idx.1,
        parsed_values.vals[sort_state.left_right_idx.0..sort_state.halves_start_idx.1]
            .iter()
            .map(|parsed_value| parsed_value.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        sort_state.left_right_idx.1,
        (sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.vals.len()),
        parsed_values.vals[sort_state.left_right_idx.1
            ..(sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.vals.len())]
            .iter()
            .map(|parsed_value| parsed_value.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );

    let virtual_k_index = sort_state.sweep_index; // the index that must be overwrited by i/j.
    let i_j_index; // The index of the ghost cube (i/j) that will be copied into the k index.

    if sort_state.left_right_idx.0 == sort_state.halves_start_idx.1 {
        // I half is fully scanned. Put J's ParsedValue in K Vector.
        i_j_index = sort_state.left_right_idx.1;
        log::info!(
            "only j remains add j: {}",
            parsed_values.vals[sort_state.left_right_idx.1].converted_value
        );
        sort_state.left_right_idx.1 += 1;
    } else if sort_state.left_right_idx.1
        == (sort_state.halves_start_idx.1 + sort_state.width).min(parsed_values.vals.len())
    {
        // j half is fully scanned. Put I's ParsedValue in K Vector.
        i_j_index = sort_state.left_right_idx.0;

        log::info!(
            "only i remains add i: {}",
            parsed_values.vals[sort_state.left_right_idx.0].converted_value
        );

        sort_state.left_right_idx.0 += 1;
    } else {
        let i_val = parsed_values.vals[sort_state.left_right_idx.0].sorted_position;
        let j_val = parsed_values.vals[sort_state.left_right_idx.1].sorted_position;
        if i_val > j_val {
            // j is smaller. Put J's ParsedValue in K Vector.
            i_j_index = sort_state.left_right_idx.1;

            log::info!(
                "{} > {}. J is smaller. add j: {}",
                parsed_values.vals[sort_state.left_right_idx.0].converted_value,
                parsed_values.vals[sort_state.left_right_idx.1].converted_value,
                parsed_values.vals[sort_state.left_right_idx.1].converted_value
            );

            sort_state.left_right_idx.1 += 1;
        } else {
            // i is smaller. Put I's ParsedValue in K Vector.
            i_j_index = sort_state.left_right_idx.0;

            log::info!(
                "{} > {}. i is smaller. add i: {}",
                parsed_values.vals[sort_state.left_right_idx.0].converted_value,
                parsed_values.vals[sort_state.left_right_idx.1].converted_value,
                parsed_values.vals[sort_state.left_right_idx.1].converted_value
            );

            sort_state.left_right_idx.0 += 1;
        }
    }

    // log::info!(
    //     "replace index [{}]{} with index [{}]{}",
    //     virtual_k_index,
    //     parsed_values.vals[virtual_k_index].converted_value,
    //     i_j_index,
    //     parsed_values.vals[i_j_index].converted_value,
    // );

    // Color the ghost cube at i/j position to mark it as covered.
    color_cube(
        i_j_index,
        SortColor::Covered,
        &sort_colors,
        &parsed_values,
        &mut cubes_query,
    );
    // Get the value of the target cube: will be stored so can be placed at the K index.
    let parsed_value: ParsedValue = parsed_values.vals[i_j_index].clone();

    // Spawn copy of the chosen i/j cube at the k index.
    let (target_cube_transform, _, _) = cubes_query.get(parsed_value.cube_handle).unwrap();
    let mut transform = *target_cube_transform;
    transform.translation.x = transform.scale.x * (virtual_k_index as f32);
    transform.scale.z = DEFAULT_Z;
    let k_handle = commands
        .spawn((
            cube_assets.mesh.clone(),
            sort_colors.materials.get(&SortColor::K).unwrap().clone(),
            transform,
            // TODO: ensure this is affected by height and width changes!
            crate::CubeData {
                index: virtual_k_index,
            },
        ))
        .id();

    sort_state.k.push(KValue {
        k_handle,
        raw_string_text: user_text.val
            [parsed_value.raw_string.start_index..parsed_value.raw_string.end_index]
            .to_string(),
        parsed_value_clone: parsed_value,
        virtual_k_index,
        original_index: i_j_index,
    });

    log::info!(
        "K: [{}]",
        sort_state
            .k
            .iter()
            .map(|k_value| k_value.parsed_value_clone.converted_value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );

    sort_state.sweep_index += 1;
    if sort_state.k.len() == (sort_state.width * 2).min(parsed_values.vals.len()) {
        // INFO: checking if k is filled: is filled when has enough values to fill out the next
        // width (if current width = 1, then k is filled when the two halves of width 1 combine to
        // crate k of width 2. if width = 2, then they combine to create 4. if 4, they combine to
        // create 8, etc.). The Next width may be larger than all values, so we check it is not
        // larger than that too (in this case, the sorting is finished).
        sort_state.next_step = SortStep::ShiftHalves;
        // else will just call compare again.
    }
}

// fn color_cubes(
//     cube_indices: Vec<usize>,
//     sort_color: SortColor,
//     sort_colors: &Res<SortColors>,
//     parsed_values: &Res<ParsedValues>,
//     cubes_query: &mut Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
// ) {
//     for i in cube_indices {
//         color_cube(i, sort_color, sort_colors, parsed_values, cubes_query);
//     }
// }

//
// pub fn uncolor_range(
//     range: (usize, usize),
//     parsed_values: &Res<ParsedValues>,
//     mut cubes_query: Query<(
//         &mut Transform,
//         &mut MeshMaterial3d<StandardMaterial>,
//         &mut crate::CubeData,
//     )>,
//     cube_assets: Res<CubeAssets>,
//     rng_color_controls: Res<crate::RNGColorControls>,
// ) {
//     log::info!(
//         "uncoloring: {}..={}",
//         range.0,
//         range.1.min(parsed_values.vals.len() - 1)
//     );
//
//     for i in range.0..=range.1.min(parsed_values.vals.len() - 1) {
//         let parsed_value = &parsed_values.vals[i];
//         let (_, mut cube_material, _) = cubes_query.get_mut(parsed_value.cube_handle).unwrap();
//         *cube_material = crate::ui::get_cube_material(
//             rng_color_controls.rng_cubes_enabled,
//             parsed_value.parsed_warning,
//             &cube_assets,
//             parsed_value.rng_color.clone(),
//         );
//     }
// }
