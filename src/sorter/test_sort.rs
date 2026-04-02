use std::time::Duration;

use web_time::Instant;

use bevy::{
    ecs::{
        resource::Resource,
        system::{Commands, Res, ResMut},
    },
    state::state::NextState,
};

use crate::sorter;

#[derive(Resource, Clone)]
pub struct SortState {
    i: usize,
    finished: bool,
}

pub fn increment_sorting(
    sort_state: Option<ResMut<SortState>>,
    mut sorting_time: ResMut<crate::SortingTime>,
    mut commands: Commands,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    rng_values_controls: Res<crate::RNGValuesControls>,
) {
    let increment_time_start = Instant::now();

    if let Some(mut sort_state) = sort_state {
        sorter::increment_between(&mut sorting_time);

        while increment_time_start.elapsed() < sorting_time.frame_budget && !sort_state.finished {
            // log::info!(
            //     "accumulated_time: {}ms",
            //     increment_time_start.elapsed().as_millis()
            // );
            if (sorting_time.visual_pause.elapsed().as_millis() as u64)
                < sorting_time.visual_pause_target
            {
                continue;
            }
            sorting_time.visual_pause = Instant::now();

            // sort step:
            sort_state.i += 1;
            if sort_state.i >= rng_values_controls.amount {
                sort_select_set.set(sorter::SortState::NotSorting);
                sort_state.finished = true;
            }
        }
    } else {
        initialize(&mut commands);
        sorter::first_increment(&mut sorting_time);
    }

    sorter::end_increment(&mut sorting_time, increment_time_start);
    sorting_time.accumulated_time = Duration::default();
}

pub fn initialize(commands: &mut Commands) {
    commands.insert_resource(SortState {
        i: 0,
        finished: false,
    });
}

pub fn complete(
    mut sorting_time: ResMut<crate::SortingTime>,
    sort_state: Option<ResMut<SortState>>,
    mut commands: Commands,
    mut scanned_cube: ResMut<crate::ScannedCube>,
) {
    if let Some(_sort_state) = sort_state {
        sorter::complete(&mut sorting_time, &mut commands, &mut scanned_cube);
        commands.remove_resource::<SortState>();
    }
}
