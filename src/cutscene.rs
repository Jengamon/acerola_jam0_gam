//! Cutscene handling
//!
//! Cuz I want to.
#![allow(dead_code)]

use std::time::Duration;

use bevy::prelude::*;

/// A cutscene is a number of tracks running in parallel
pub struct Cutscene {
    tracks: Vec<Track>,
}

pub struct Track {
    steps: Vec<Step>,
    ending: Ending,
}

pub struct Step {
    action: StepAction,
    duration: Duration,
    parallel: bool,
}

pub enum StepAction {
    /// Do nothing.
    Wait,
    // TODO *who?*
    /// Place a participant at a location
    Place(Vec2),
    /// Move a participant at a certain velocity
    Move { velocity: Vec2 },
}

#[derive(Default)]
pub enum Ending {
    /// Fade the scene out, then reset to scene state before the
    /// cutscene happened.
    #[default]
    End,
    /// End the cutscene, without resetting any state
    Cut,
}
