use bevy::{
    ecs::{
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    pbr::{MeshMaterial3d, StandardMaterial},
    state::state::NextState,
    time::Time,
    transform::components::Transform,
};

use crate::{
    AudioControls, sorter,
    ui::{ParsedValues, UserText},
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
    end_index: usize,
}

pub fn increment_sorting(
    sort_state: Option<ResMut<SortState>>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
    mut commands: Commands,
    parsed_values: ResMut<ParsedValues>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
) {
    if let Some(sort_state) = sort_state {
        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            return;
        }
        match sort_state.next_step {
            SortStep::ShiftRight => shift_right(Some(sort_state), commands, parsed_values.into()),
            SortStep::RestartShifting => restart_shifting(sort_state, sort_select_set),
            SortStep::Compare => compare(sort_state, parsed_values, cubes_query, user_text),
        }
    } else {
        shift_right(None, commands, parsed_values.into())
    }
    increment_timer.increment_timer.reset();
}

pub fn shift_right(
    sort_state: Option<ResMut<SortState>>,
    mut commands: Commands,
    parsed_values: Res<ParsedValues>,
) {
    if let Some(mut sort_state) = sort_state {
        // Check if next element is valid:
        let next_element = sort_state.j + 1;
        if next_element == sort_state.end_index {
            // next element is at a bubble sorted index. Restart from left.

            log::info!("bubbled range reached: {}", next_element);

            sort_state.next_step = SortStep::RestartShifting;
            return;
        }

        // Shift right
        increment_i_j(&mut sort_state);
        sort_state.next_step = SortStep::Compare;
    } else {
        commands.insert_resource(SortState {
            next_step: SortStep::Compare,
            i: 0,
            j: 1,
            end_index: parsed_values.end_index,
        })
    }
}

pub fn increment_i_j(sort_state: &mut ResMut<SortState>) {
    sort_state.i += 1;
    sort_state.j += 1;
    log::info!("increment i and j: {} {}", sort_state.i, sort_state.j);
    // TODO: Color new i and j! (uncolor previous ones?)
}

pub fn restart_shifting(
    mut sort_state: ResMut<SortState>,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
) {
    // Another element bubbled up to the end.
    sort_state.end_index -= 1;

    log::info!(
        "increasing amount of bubbled elements: now starts at: {}",
        sort_state.end_index
    );

    if sort_state.end_index <= 2 {
        // only 1 non-bubbled element remains: already know it is sorted.
        sort_select_set.set(sorter::SortState::NotSorting);
        return;
    }

    // TODO: recolor?
    sort_state.i = 0;
    sort_state.j = 1;

    sort_state.next_step = SortStep::Compare;
}

pub fn compare(
    mut sort_state: ResMut<SortState>,
    mut parsed_values: ResMut<ParsedValues>,
    // audio_controls: Res<AudioControls>,
    // mut scanned_cube: ResMut<crate::ScannedCube>,
    cubes_query: Query<(
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut crate::CubeData,
    )>,
    user_text: ResMut<UserText>,
) {
    let i = parsed_values.vals[sort_state.i].sorted_position;
    let j = parsed_values.vals[sort_state.j].sorted_position;

    if i > j {
        log::info!("swapping: {} with {}", i, j);
        crate::sorter::swap(
            sort_state.i,
            sort_state.j,
            &mut parsed_values,
            cubes_query,
            user_text,
        );
    }
    sort_state.next_step = SortStep::ShiftRight;
}

pub fn complete(mut commands: Commands, sort_state: Option<Res<SortState>>) {
    if let Some(sort_state) = sort_state {
        commands.remove_resource::<SortState>();
    }
}
