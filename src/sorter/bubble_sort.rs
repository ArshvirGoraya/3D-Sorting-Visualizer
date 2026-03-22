use std::collections::HashSet;

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
    AudioControls, sorter,
    ui::{CubeAssets, ParsedValues, UserText},
};

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortStep {
    #[default]
    ShiftRight,
    RestartShifting,
    Compare,
}

#[derive(Resource, Clone)]
pub struct SortState {
    next_step: SortStep,
    i: usize,
    j: usize,
    bubble_range_start: usize,
    swapped_cubes: Option<(usize, usize)>,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SortColor {
    #[default]
    Covered,
    Bubbled,
    Swap,
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
                    SortColor::Covered,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Yellow (RoseWater)
                        // base_color: Color::srgb_u8(244, 219, 214),
                        // Yellow (Flamingo)
                        base_color: Color::srgb_u8(240, 198, 198),
                        // base_color: Color::srgba_u8(238, 212, 159, 25),
                        // alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..Default::default()
                    })),
                ),
                (
                    SortColor::Bubbled,
                    MeshMaterial3d(materials.add(StandardMaterial {
                        // Yellow
                        base_color: Color::srgb_u8(238, 212, 159),
                        // base_color: Color::srgba_u8(238, 212, 159, 25),
                        // alpha_mode: AlphaMode::Blend,
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
                // ()
            ]),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn increment_sorting(
    sort_state: Option<ResMut<SortState>>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
    commands: Commands,
    parsed_values: ResMut<ParsedValues>,
    sort_select_set: ResMut<NextState<sorter::SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
    audio_controls: Res<AudioControls>,
    scanned_cube: ResMut<crate::ScannedCube>,
    sort_colors: Res<SortColors>,
    rng_color_controls: Res<crate::RNGColorControls>,
    cube_assets: Res<CubeAssets>,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
) {
    if let Some(sort_state) = sort_state {
        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            return;
        }
        match sort_state.next_step {
            SortStep::ShiftRight => shift_right(
                Some(sort_state),
                commands,
                parsed_values,
                sort_colors,
                cubes_query,
                sort_colored_cubes,
            ),
            SortStep::RestartShifting => restart_shifting(
                sort_state,
                sort_select_set,
                sort_colors,
                parsed_values,
                cubes_query,
                rng_color_controls,
                cube_assets,
                sort_colored_cubes.unwrap(),
            ),
            SortStep::Compare => compare(
                sort_state,
                parsed_values,
                cubes_query,
                user_text,
                commands,
                audio_controls,
                scanned_cube,
                sort_colors,
                sort_colored_cubes.unwrap(),
            ),
        }
    } else {
        shift_right(
            None,
            commands,
            parsed_values,
            sort_colors,
            cubes_query,
            sort_colored_cubes,
        )
    }
    increment_timer.increment_timer.reset();
}

pub fn shift_right(
    sort_state: Option<ResMut<SortState>>,
    mut commands: Commands,
    parsed_values: ResMut<ParsedValues>,
    sort_colors: Res<SortColors>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
) {
    if let Some(mut sort_state) = sort_state {
        if let Some(swapped_cubes) = sort_state.swapped_cubes {
            let mut sort_colored_cubes = sort_colored_cubes.unwrap();
            color_cube(
                swapped_cubes.0,
                SortColor::Covered,
                &sort_colors,
                &parsed_values,
                &mut cubes_query,
                &mut sort_colored_cubes,
            );
            color_cube(
                swapped_cubes.1,
                SortColor::Covered,
                &sort_colors,
                &parsed_values,
                &mut cubes_query,
                &mut sort_colored_cubes,
            );
            sort_state.swapped_cubes = None;
        }

        // Check if next element is valid:
        let next_element = sort_state.j + 1;
        if next_element == sort_state.bubble_range_start {
            // next element is at a bubble sorted index. Restart from left.
            sort_state.next_step = SortStep::RestartShifting;
            return;
        }

        // Shift right
        sort_state.i += 1;
        sort_state.j += 1;
        sort_state.next_step = SortStep::Compare;
    } else {
        commands.insert_resource(crate::SortColoredCubes {
            cubes: HashSet::new(),
        });
        commands.insert_resource(SortState {
            next_step: SortStep::Compare,
            i: 0,
            j: 1,
            bubble_range_start: parsed_values.end_index,
            swapped_cubes: None,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn restart_shifting(
    mut sort_state: ResMut<SortState>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    sort_colors: Res<SortColors>,
    parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    rng_color_controls: Res<crate::RNGColorControls>,
    cube_assets: Res<CubeAssets>,
    mut sort_colored_cubes: ResMut<crate::SortColoredCubes>,
) {
    // Another element bubbled up to the end.
    sort_state.bubble_range_start -= 1;
    if sort_state.bubble_range_start <= 1 {
        // only 1 non-bubbled element remains: already know it is sorted.
        sort_select_set.set(sorter::SortState::NotSorting);
        return;
    }

    color_cube(
        sort_state.j,
        SortColor::Bubbled,
        &sort_colors,
        &parsed_values,
        &mut cubes_query,
        &mut sort_colored_cubes,
    );
    // Uncolor entire Covered Range
    uncolor_range(
        (0, sort_state.bubble_range_start),
        &parsed_values.into(),
        &rng_color_controls,
        &cube_assets,
        &mut cubes_query,
        &mut sort_colored_cubes,
    );

    sort_state.i = 0;
    sort_state.j = 1;

    sort_state.next_step = SortStep::Compare;
}

#[allow(clippy::too_many_arguments)]
pub fn compare(
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
    mut commands: Commands,
    audio_controls: Res<AudioControls>,
    mut scanned_cube: ResMut<crate::ScannedCube>,
    sort_colors: Res<SortColors>,
    mut sort_colored_cubes: ResMut<crate::SortColoredCubes>,
) {
    crate::play_audio(
        &mut commands,
        &audio_controls,
        parsed_values.vals[sort_state.j].sorted_position,
        parsed_values.end_index,
    );
    scanned_cube.transform = Some(
        *cubes_query
            .get(parsed_values.vals[sort_state.j].cube_handle)
            .unwrap()
            .0,
    );

    let i = parsed_values.vals[sort_state.i].sorted_position;
    let j = parsed_values.vals[sort_state.j].sorted_position;

    if i > j {
        color_cube(
            sort_state.i,
            SortColor::Swap,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        color_cube(
            sort_state.j,
            SortColor::Swap,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        crate::sorter::swap(
            sort_state.i,
            sort_state.j,
            &mut parsed_values,
            &mut cubes_query,
            user_text,
        );
        sort_state.swapped_cubes = Some((sort_state.i, sort_state.j));
    } else {
        if sort_state.i == 0 {
            color_cube(
                sort_state.i,
                SortColor::Covered,
                &sort_colors,
                &parsed_values,
                &mut cubes_query,
                &mut sort_colored_cubes,
            );
        }
        color_cube(
            sort_state.j,
            SortColor::Covered,
            &sort_colors,
            &parsed_values,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
    }
    sort_state.next_step = SortStep::ShiftRight;
}

#[allow(clippy::too_many_arguments)]
pub fn complete(
    mut commands: Commands,
    sort_state: Option<ResMut<SortState>>,
    cube_assets: Option<Res<CubeAssets>>,
    parsed_values: Res<ParsedValues>,
    rng_color_controls: Res<crate::RNGColorControls>,
    mut cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: Option<ResMut<crate::SortColoredCubes>>,
    mut scanned_cube: ResMut<crate::ScannedCube>,
) {
    if let Some(_sort_state) = sort_state
        && let Some(cube_assets) = cube_assets
        && let Some(mut sort_colored_cubes) = sort_colored_cubes
    {
        uncolor_range(
            (0, parsed_values.end_index),
            &parsed_values,
            &rng_color_controls,
            &cube_assets,
            &mut cubes_query,
            &mut sort_colored_cubes,
        );
        commands.remove_resource::<SortState>();
        commands.remove_resource::<crate::SortColoredCubes>();
        scanned_cube.transform = None;
    }
}

pub fn uncolor_range(
    range: (usize, usize),
    parsed_values: &Res<ParsedValues>,
    rng_color_controls: &Res<crate::RNGColorControls>,
    cube_assets: &Res<CubeAssets>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: &mut ResMut<crate::SortColoredCubes>,
) {
    for i in range.0..range.1 {
        uncolor_cube(
            i,
            parsed_values,
            rng_color_controls,
            cube_assets,
            cubes_query,
            sort_colored_cubes,
        );
    }
}

pub fn uncolor_cube(
    cube_index: usize,
    parsed_values: &Res<ParsedValues>,
    rng_color_controls: &Res<crate::RNGColorControls>,
    cube_assets: &Res<CubeAssets>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: &mut ResMut<crate::SortColoredCubes>,
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

pub fn color_cube(
    cube_index: usize,
    sort_color: SortColor,
    sort_colors: &Res<SortColors>,
    parsed_values: &ResMut<ParsedValues>,
    cubes_query: &mut Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    sort_colored_cubes: &mut ResMut<crate::SortColoredCubes>,
) {
    let (_, mut cube_material, _) = cubes_query
        .get_mut(parsed_values.vals[cube_index].cube_handle)
        .unwrap();

    *cube_material = sort_colors.materials.get(&sort_color).unwrap().clone();

    sort_colored_cubes.cubes.insert(cube_index);
}
