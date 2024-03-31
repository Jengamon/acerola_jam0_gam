#![allow(dead_code)]
// This is gonna remain dead for some time...

use std::collections::HashMap;

use bevy::prelude::*;
use generic_array::ArrayLength;

use super::{Machine, MachinedHandle, Trackable};

/// Spatial Track, almost like [`super::Track`] except can emit audio
/// from a location
///
/// We follow almost the same rule for [`Machine`]s as tracks, in that it
/// is generally 1 per unit, but for our case, a unit is an emitting entity rather
/// than the entire track itself
#[derive(Resource)]
pub struct SpatialTrack<T, N: ArrayLength> {
    trackable: Trackable<T, N>,
    output: kira::OutputDestination,
    sample_rate: f64,
    active_handle: HashMap<Entity, MachinedHandle>,
    next_machine: HashMap<Entity, Handle<Machine>>,
    // TODO Manage emitters and listeners here
    // TODO Listeners can use channels (kira tracks), so allow for subchannel ids
}
