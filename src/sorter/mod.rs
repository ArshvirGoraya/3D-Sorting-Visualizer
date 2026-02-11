pub mod quick_sort;
// use crate::sorter::quick_sort;

use bevy::prelude::*;
use core::{fmt, time::Duration};

use crate::ui::ParsedValues;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum SortState {
    #[default]
    NotSorting,
    Sorting,
    // Paused,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum Algorithms {
    #[default]
    QuickSort,
    MergeSort,
}
impl fmt::Display for Algorithms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithms::QuickSort => write!(f, "Quick Sort"),
            Algorithms::MergeSort => write!(f, "Merge Sort"),
        }
    }
}
impl Algorithms {
    pub const ALL: [Algorithms; 2] = [Algorithms::QuickSort, Algorithms::MergeSort];
}

#[derive(Resource)]
pub struct IncrementTimer {
    pub increment_timer: Timer,
    pub duration: Duration,
    pub duration_f64: f64,
}

pub fn begin_sorting(
    sort_select_get: Res<State<Algorithms>>,
    mut quick_sort_event: MessageWriter<quick_sort::SetupRange>,
    // mut quick_sort_state: ResMut<NextState<quick_sort::SortStep>>,
) {
    match *sort_select_get.get() {
        Algorithms::QuickSort => {
            quick_sort_event.write(quick_sort::SetupRange);
        }
        Algorithms::MergeSort => {}
    };
}

// pub fn increment_sorting(
//     sort_select_get: Res<State<Algorithms>>,
//     mut quick_sort_event: MessageReader<quick_sort::SortIncrement>,
// ) {
//     // TODO: check if needs to stop.
//     // TODO: delay by timer setting.
//     match *sort_select_get.get() {
//         Algorithms::QuickSort => quick_sort::increment_sorting(quick_sort_event),
//         Algorithms::MergeSort => {}
//     };
// }
