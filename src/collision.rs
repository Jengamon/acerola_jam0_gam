//! Handle collisionts

#[allow(unused)]
use bevy::math::bounding::{Aabb2d, RayCast2d};
use bevy::{math::bounding::BoundingVolume, prelude::*};

use crate::movement;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CollisionEvent>()
            // .add_systems(Update, debug_log_collisions)
            .add_systems(FixedUpdate, resolve_collisions.before(movement::Movement));
    }
}

#[derive(Debug, Event)]
pub struct CollisionEvent {
    pub object_a: Entity,
    pub object_b: Entity,
    pub mtv: Vec2,
}

#[derive(Component, Default)]
pub struct Collidable {
    pub offset: Vec2,
    pub extents: Vec2,
}

// fn debug_log_collisions(mut reader: EventReader<CollisionEvent>) {
//     for event in reader.read() {
//         trace!(?event, "Collision Event!");
//     }
// }

fn resolve_collisions(
    mut query: Query<(Entity, &GlobalTransform, &mut Collidable)>,
    mut writer: EventWriter<CollisionEvent>,
) {
    let mut iter = query.iter_combinations_mut();
    while let Some([(entity, gtransform, collidable), (oentity, ogtransform, ocollidable)]) =
        iter.fetch_next()
    {
        let aabb = Aabb2d::new(
            gtransform.translation().truncate() + collidable.offset,
            collidable.extents / 2.,
        );
        let oaabb = Aabb2d::new(
            ogtransform.translation().truncate() + ocollidable.offset,
            ocollidable.extents / 2.,
        );
        if let Some(mtv) = sat_collision_aabb(aabb, oaabb) {
            let (magnitude, axis) = (mtv.length(), mtv.normalize());
            let aabb_min_proj = aabb.min.project_onto_normalized(axis);
            let oaabb_min_proj = oaabb.min.project_onto_normalized(axis);
            let mtv = axis * magnitude * (oaabb_min_proj - aabb_min_proj).signum();
            writer.send(CollisionEvent {
                object_a: entity,
                object_b: oentity,
                mtv,
            });
        }
    }
}

// Projection is (min, max)
fn aabb_project(shape: Aabb2d, axis: Vec2) -> Option<(f32, f32)> {
    let norm = axis.try_normalize()?;
    let points = [
        shape.min,
        Vec2::new(shape.max.x, shape.min.y),
        Vec2::new(shape.min.x, shape.max.y),
        shape.max,
    ];
    let mut min = norm.dot(points[0]);
    let mut max = min;
    for point in &points[1..] {
        let v = norm.dot(*point);
        min = min.min(v);
        max = max.max(v);
    }
    Some((min, max))
}

// Implementation of SAT collision, but only for AABBs
// Based off https://dyn4j.org/2010/01/sat/
fn sat_collision_aabb(a: Aabb2d, b: Aabb2d) -> Option<Vec2> {
    // We can cheat on axises, cuz we only support Aabbs for now
    let axises = [Vec2::X, Vec2::Y];
    let min_trans_vec = axises.into_iter().try_fold(vec![], |mut mtvs, axis| {
        // These are only None if the axis passed in is not normalizable...
        let (min_a, max_a) = aabb_project(a, axis).unwrap();
        let (min_b, max_b) = aabb_project(b, axis).unwrap();
        // info!("{min_a} {max_a} {min_b} {max_b}");
        if max_a <= min_b || max_b <= min_a {
            // we have a non-overlapping axis, so we know these shapes aren't colliding
            Err(())
        } else {
            // Calc the overlap
            let mut overlap = (max_a - min_b).min(max_b - min_a);
            if a.contains(&b) || b.contains(&a) {
                // Handle containment
                let mins = (min_a - min_b).abs();
                let maxs = (max_a - max_b).abs();
                if mins < maxs {
                    overlap += mins;
                } else {
                    overlap += maxs;
                }
            }
            mtvs.push(axis * overlap);
            Ok(mtvs)
        }
    });
    min_trans_vec.ok().and_then(|mtvs| {
        mtvs.into_iter().reduce(|minv, v| {
            if minv.length().min(v.length()) == minv.length() {
                minv
            } else {
                v
            }
        })
    })
}
