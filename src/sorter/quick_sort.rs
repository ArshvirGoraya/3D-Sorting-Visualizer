use std::ops::{Range, RangeBounds};

use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        event::Event,
        message::{Message, MessageReader, MessageWriter},
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        world::{FromWorld, World},
    },
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::HashMap,
    reflect::Set,
    state::state::States,
    transform::components::Transform,
};

use crate::ui::{CubeAssets, ParsedValues};

// #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SortStep {
    #[default]
    SetupRange,
    Compare,
    Swap,
    DetectComplete,
    Complete,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    Range,
    Pivot,
    Swap,
    J,
    I,
}

#[derive(Message)]
pub struct SetupRange;
#[derive(Message)]
pub struct Compare;
#[derive(Message)]
pub struct Swap;
#[derive(Message)]
pub struct DetectComplete;
#[derive(Message)]
pub struct Complete;

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

// fn init_colors_resource(mut commands: &mut Commands) {
//     // Only inserts if not already added:
//     commands.init_resource::<QuickSortColors>()
// }

// pub fn start(mut commands: Commands, parsed_values: Res<ParsedValues>) {
//     // Only inserts if not already added:
//     commands.init_resource::<QuickSortColors>();
//     // commands.remove_resource::<QuickSortColors>();
//
//     // parsed_values.end_index
//
//     // set all cube materials to the default color
//     println!("Quick Sorting...");
//
//     // Setup sorting before increment_sorting can be called.
//
//     // increment_sorting();
// }

#[derive(Resource)]
pub struct SortState {
    started: bool,
    sub_arrays: Vec<(usize, usize)>,
    current_array: (usize, usize),
    pivot: usize,
    j: usize,
    i: isize,
}

pub fn setup_range(
    mut commands: Commands,
    parsed_values: Res<ParsedValues>,
    mut sort_state: Option<ResMut<SortState>>,
    cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut next_event: MessageWriter<Compare>,
) {
    // Only inserts if not already added:
    // commands.init_resource::<QuickSortColors>();
    println!("Quick Sorting...");

    if let Some(mut sort_state) = sort_state {
        let previous_array = sort_state.current_array;
        sort_state.current_array = *sort_state.sub_arrays.last_mut().unwrap();
        color_range(
            previous_array,
            sort_state.current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
        );
    } else {
        let current_array = (0, parsed_values.end_index);
        commands.insert_resource(SortState {
            started: true, // TODO: not needed?
            sub_arrays: [current_array].to_vec(),
            current_array,
            pivot: current_array.1,
            j: current_array.0,
            i: (current_array.0 as isize) - 1,
        });
        color_range(
            current_array,
            current_array,
            cubes_query,
            parsed_values,
            quick_sort_colors,
            cube_assets,
            rng_color_controls,
        );
    }
    next_event.write(Compare);
}

pub fn compare(
    mut cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    mut sort_state: ResMut<SortState>,
    parsed_values: Res<ParsedValues>,
    quick_sort_colors: Res<QuickSortColors>,
    mut next_event: MessageWriter<Swap>,
) {
    let pivot_value = &parsed_values.vals[sort_state.pivot].converted_value;
    let j_value = &parsed_values.vals[sort_state.j].converted_value;

    if sort_state.pivot == sort_state.j || pivot_value > j_value {
        // color cube at what i used to be with the j/"covered" color.
        if sort_state.i > -1 {
            color_cube(
                sort_state.i as usize,
                SortColor::J,
                &quick_sort_colors,
                &parsed_values,
                &mut cubes_query,
            )
        }
        sort_state.i += 1;
        // color cubes j and i with "swap" color.
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
    } else {
        sort_state.j += 1;
        // color cube at j with j/"covered" color.
        color_cube(
            sort_state.j,
            SortColor::J,
            &quick_sort_colors,
            &parsed_values,
            &mut cubes_query,
        );
    }
    // next_event.write(Swap);
}

pub fn swap(
    sort_state: Res<SortState>,
    parsed_values: Res<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
) {
    // swap i and j.
    let i_data = &parsed_values.vals[sort_state.i as usize];
    let j_data = &parsed_values.vals[sort_state.j];

    // let mut temp_data;
    // if let Ok((mut t, mut m, mut d)) = cubes_query.get_mut(i_data.cube_handle) {
    //     //
    // }
    // if let Ok((mut t, mut m, mut d)) = cubes_query.get_mut(j_data.cube_handle) {
    //     //
    // }

    let (mut transform_i, mut material_i, mut cube_data_i) =
        cubes_query.get_mut(i_data.cube_handle).unwrap();
    let i_pos = transform_i.translation.x;

    let (mut transform_j, mut material_j, mut cube_data_j) =
        cubes_query.get_mut(j_data.cube_handle).unwrap();

    let j_pos = transform_j.translation.x;
    transform_j.translation.x = i_pos;

    let (mut transform_i, mut material_i, mut cube_data_i) =
        cubes_query.get_mut(i_data.cube_handle).unwrap();

    transform_i.translation.x = j_pos;

    // let j_pos_store = transform_j.translation.x;

    // let j_pos = transform_j.translation.x;
    // transform_j.translation.x = i_pos;
    // transform_i.translation.x = j_pos;

    // transform_i.translation.x = transform_j.translation.x;
    // transform_j.translation.x = temp;
}

pub fn clean_up(sort_state: Option<Res<SortState>>) {
    // Runs when Quicksort is the selected algorithm (which is the default) and OnEnter for NotSorting (which is the
    // default): so will run at startup.
    if let Some(sort_state) = sort_state {
        //
    }
}

fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    quick_sort_colors: &Res<QuickSortColors>,
    parsed_values: &Res<ParsedValues>,
    mut cubes_query: &mut Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
) {
    let mut cube_material = cubes_query
        .get_mut(parsed_values.vals[cube_index].cube_handle)
        .unwrap();

    *cube_material = quick_sort_colors
        .materials
        .get(&sort_color)
        .unwrap()
        .clone();
}

fn color_range(
    previous_range: (usize, usize),
    current_range: (usize, usize),
    mut cubes_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<crate::CubeData>>,
    parsed_values: Res<ParsedValues>,
    quick_sort_colors: Res<QuickSortColors>,
    cube_assets: Res<CubeAssets>,
    rng_color_controls: Res<crate::RNGColorControls>,
) {
    let min = usize::min(previous_range.0, current_range.0);
    let max = usize::max(previous_range.1, current_range.1);

    for i in min..max {
        let parsed_value = &parsed_values.vals[i];
        let mut cube_material = cubes_query.get_mut(parsed_value.cube_handle).unwrap();

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

//
// pub fn increment_sorting(mut quick_sort_event: MessageReader<SortIncrement>) {
//     let sort_step = quick_sort_event.read().last().unwrap().step;
//
//     // match sort_step{
//     //     SortStep::SetupRange => setup_range
//     // }
//
//     // if let Some(event) = quick_sort_event.read().last() {
//     //     //
//     // }
//     //
// }
