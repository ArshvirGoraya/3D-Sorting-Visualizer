pub mod quick_sort;
// use crate::sorter::quick_sort;

use bevy::prelude::*;
use core::{fmt, time::Duration};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum SortState {
    #[default]
    NotSorting,
    Sorting,
    Paused,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
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

pub fn sort_control_button_clicked(sort_state: SortState) {
    let mut next_state = SortState::NotSorting;
    match sort_state {
        SortState::Sorting => {
            next_state = SortState::NotSorting;
        }
        SortState::NotSorting => {
            next_state = SortState::Sorting;
        }
        SortState::Paused => {
            next_state = SortState::Sorting;
        }
    }
}

pub fn begin_sorting() {
    // begin the correct sort sequence depending on the selected sorter (e.g., quicksort)
    // set all cube materials

    quick_sort::start();
    // quick_sort::start();
}

pub fn increment_sorting() {
    quick_sort::increment_sorting();

    //

    // check if need to pause
    // check if can stop
    // next step after some delay
}
