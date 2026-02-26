pub mod quick_sort;

use bevy::prelude::*;
use core::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum SortState {
    #[default]
    NotSorting,
    Sorting,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum Algorithms {
    #[default]
    QuickSort,
    // MergeSort,
}
impl fmt::Display for Algorithms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithms::QuickSort => write!(f, "Quick Sort"),
            // Algorithms::MergeSort => write!(f, "Merge Sort"),
        }
    }
}
impl Algorithms {
    pub const ALL: [Algorithms; 1] = [
        Algorithms::QuickSort,
        // Algorithms::MergeSort
    ];
}

#[derive(Resource)]
pub struct IncrementTimer {
    pub increment_timer: Timer,
    pub duration_f64: f64,
}
