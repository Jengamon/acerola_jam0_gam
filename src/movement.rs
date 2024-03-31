//! Handles moving things *and* them colliding
use bevy::prelude::*;

use crate::collision;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                handle_collisions,
                fake_friction,
                update_velocity,
                update_transform,
            )
                .chain()
                .in_set(Movement),
        );
    }
}

/// System set representing all movement systems
#[derive(SystemSet, Debug, Hash, Clone, PartialEq, Eq)]
pub struct Movement;

#[derive(Component, Debug)]
pub struct Movable {
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub mass: f32,
}

impl Default for Movable {
    fn default() -> Self {
        Self {
            velocity: default(),
            acceleration: default(),
            mass: 1.0,
        }
    }
}

fn handle_collisions(
    mut movable: Query<(&mut Movable, &mut Transform)>,
    mut reader: EventReader<collision::CollisionEvent>,
) {
    for event in reader.read() {
        if let Some([adata, bdata]) = movable.get_many_mut([event.object_a, event.object_b]).ok() {
            let (am, mut at) = adata;
            let (bm, mut bt) = bdata;
            let ima = am.mass.recip();
            let imb = bm.mass.recip();
            let im = ima + imb;
            if ima.is_infinite() || imb.is_infinite() {
                continue;
            }
            at.translation -= event.mtv.extend(0.) * (ima / im);
            bt.translation += event.mtv.extend(0.) * (imb / im);
        } else if let Some(adata) = movable.get_mut(event.object_a).ok() {
            let (_am, mut at) = adata;
            at.translation += event.mtv.extend(0.);
        } else if let Some(bdata) = movable.get_mut(event.object_b).ok() {
            let (_bm, mut bt) = bdata;
            bt.translation += event.mtv.extend(0.);
        } else {
            // Nothing is movable of these events, do nothing
        }
    }
}

/// Speeds below this should be considered 0.
pub const MIN_SPEED: f32 = 1.0;
// highest speed we expect movement-pointered stuff to move at
pub const MAX_SPEED: f32 = 1000.0;

fn fake_friction(mut query: Query<&mut Movable>) {
    // TODO Allow this to be determined by tiles!
    // This should range from 0.0 to 1.0
    const FAKE_MU: f32 = 30.0;
    const FAKE_MU_STATIC: f32 = 35.0;
    for mut movable in query.iter_mut() {
        // info!(?movable);
        if movable.velocity.length() > MIN_SPEED {
            let vnorm = movable.velocity.normalize();
            // Friction = mu * N where N is the normal force
            // we assume the only normal force is gravity.
            // so friction in our system is
            // mu * mass * gravity
            let mass = movable.mass;
            let friction_magnitude = FAKE_MU * mass.max(1.0).recip();
            movable.velocity -= vnorm * friction_magnitude;
        } else {
            let vnorm = movable.velocity.normalize_or_zero();
            // Friction = mu * N where N is the normal force
            // we assume the only normal force is gravity.
            // so friction in our system is
            // mu * mass * gravity
            let mass = movable.mass;
            let friction_magnitude = FAKE_MU_STATIC * mass.max(1.0).recip();
            if vnorm != Vec2::ZERO {
                movable.velocity -= vnorm * friction_magnitude;
            }
        }
    }
}

fn update_velocity(mut query: Query<&mut Movable>, time: Res<Time>) {
    for mut movable in query.iter_mut() {
        let acceleration = movable.acceleration;
        if movable.velocity.length() < MIN_SPEED {
            movable.velocity = Vec2::ZERO;
        }
        movable.velocity += acceleration * time.delta_seconds();
        // Zero accel
        movable.acceleration *= 0.1;
    }
}

fn update_transform(mut query: Query<(&mut Transform, &Movable)>, time: Res<Time>) {
    for (mut transform, moveable) in query.iter_mut() {
        // Update transform
        transform.translation += moveable.velocity.extend(0.0) * time.delta_seconds();
    }
}
