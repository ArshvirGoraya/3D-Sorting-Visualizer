use web_time::Instant;

use bevy::{
    ecs::{
        resource::Resource,
        system::{Commands, Res, ResMut},
    },
    state::state::NextState,
    time::Time,
};

use crate::sorter;

#[derive(Resource, Clone)]
pub struct SortState {
    i: usize,
}

pub fn increment_sorting(
    sort_state: Option<ResMut<SortState>>,
    (time, mut increment_timer): (Res<Time>, ResMut<sorter::IncrementTimer>),
    mut sorting_time: ResMut<crate::SortingTime>,
    mut commands: Commands,
    mut sort_select_set: ResMut<NextState<sorter::SortState>>,
    rng_values_controls: Res<crate::RNGValuesControls>,
) {
    let increment_time_start = Instant::now();

    if let Some(mut sort_state) = sort_state {
        sorter::increment_between(&mut sorting_time);
        increment_timer.increment_timer.tick(time.delta());
        if !increment_timer.increment_timer.is_finished() {
            sorter::end_increment(&mut sorting_time, increment_time_start);
            return;
        } else {
            // sort step:
            sort_state.i += 1;
            if sort_state.i >= rng_values_controls.amount {
                sort_select_set.set(sorter::SortState::NotSorting);
            }
        }
    } else {
        commands.insert_resource(SortState { i: 0 });
        sorter::first_increment(&mut sorting_time);
    }

    sorter::end_increment(&mut sorting_time, increment_time_start);
    increment_timer.increment_timer.reset();
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
