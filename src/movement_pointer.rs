use bevy::prelude::*;
use bevy_prototype_lyon::{entity::ShapeBundle, geometry::GeometryBuilder, shapes};

use crate::{movement, utils::lerp_mix};

pub struct MovementPointerPlugin;

impl Plugin for MovementPointerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_movement_pointer);
    }
}

/// A component that indicates an entity is moving in a direction
#[derive(Component, Default)]
pub struct MovementDirection(pub Vec2, pub f32);

/// Indicates this entity is a movement pointer
#[derive(Component)]
pub struct MovementPointer;

#[derive(Bundle)]
pub struct MovementPointerBundle {
    pointer: MovementPointer,
    shape: ShapeBundle,
}

impl MovementPointerBundle {
    pub fn new(material: Handle<ColorMaterial>) -> Self {
        Self {
            pointer: MovementPointer,
            shape: ShapeBundle {
                spatial: SpatialBundle {
                    transform: Transform::default().with_scale(Vec3::new(0.5, 1.0, 1.0)),
                    ..default()
                },
                path: GeometryBuilder::build_as(&shapes::RegularPolygon {
                    sides: 3,
                    feature: shapes::RegularPolygonFeature::Apothem(5.0),
                    ..default()
                }),
                material,
                ..default()
            },
        }
    }
}

fn update_movement_pointer(
    mut query: Query<
        (
            Entity,
            &mut Transform,
            &mut Visibility,
            Option<&Handle<ColorMaterial>>,
        ),
        With<MovementPointer>,
    >,
    parent_query: Query<&Parent>,
    movement_direction: Query<&MovementDirection>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, mut transform, mut vis, maybe_tex) in query.iter_mut() {
        for ancestor in parent_query.iter_ancestors(entity) {
            let Ok(md) = movement_direction.get(ancestor) else {
                continue;
            };

            // Color shenanigans
            const MIN_COLOR: Color = Color::BLUE;
            const MAX_COLOR: Color = Color::RED;
            if let Some(tex) = maybe_tex {
                let mixed = lerp_mix(MIN_COLOR, MAX_COLOR, md.1 / movement::MAX_SPEED);
                // tex handle is present, so expect it points to a valid color material
                materials
                    .get_mut(tex)
                    .expect("MP Color Texture handle exists, but doesn't point to valid handle")
                    .color = mixed;
            }

            const MIN_POINTER_DISTANCE: f32 = 25.0;
            // How big is the pointer range between fastest and slowest
            const MAX_POINTER_DELTA: f32 = 35.0;

            if let Some(norm) = md.0.try_normalize() {
                // Fake depth by scaling y translation
                let faked_norm = norm * Vec2::new(1.0, 0.85);
                *vis = if md.1 > movement::MIN_SPEED * 10. {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                transform.translation = (faked_norm
                    * (MIN_POINTER_DISTANCE
                        + MAX_POINTER_DELTA * (md.1 / movement::MAX_SPEED).clamp(0.0, 1.0)))
                .extend(0.0);
                transform.rotation = Quat::from_rotation_z(
                    norm.y.signum() * Vec2::X.dot(norm).acos() - std::f32::consts::FRAC_PI_2,
                );
            } else {
                // Was 0 or close to 0
                *vis = Visibility::Hidden;
            }
        }
    }
}
