use crate::{camera, collision, movement};

use super::{movement_pointer, PlayerAssets};
use bevy::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;
use leafwing_input_manager::prelude::*;
use moonshine_spawn::{spawn_children, SpawnChildren};

#[derive(Component)]
pub(crate) struct DashState(pub bool);

pub(crate) fn handle_dashing(mut query: Query<(&mut DashState, &ActionState<PlayerAction>)>) {
    for (mut dash_state, action_state) in query.iter_mut() {
        // handle hold dash
        if action_state.just_pressed(&PlayerAction::HoldDash) {
            dash_state.0 = true;
        } else if action_state.just_released(&PlayerAction::HoldDash) {
            dash_state.0 = false;
        }

        // Handle toggle dash
        if action_state.just_pressed(&PlayerAction::ToggleDash) {
            dash_state.0 = !dash_state.0;
        }
    }
}

pub(crate) fn move_player(
    mut query: Query<
        (
            &ActionState<PlayerAction>,
            &mut movement::Movable,
            Option<&mut Sprite>,
            &DashState,
            Option<&mut movement_pointer::MovementDirection>,
        ),
        With<Player>,
    >,
) {
    for (action_state, mut movable, maybe_sprite, dash, maybe_pointer) in query.iter_mut() {
        // Right now, treat our joystick as a velocity.
        if let Some(dad) = action_state.clamped_axis_pair(&PlayerAction::Move) {
            const SPEED: f32 = 350.0;
            const DASH_SPEED: f32 = 700.0;

            let movement = dad.xy();

            if let Some(movement_norm) = movement.try_normalize() {
                let speed = if dash.0 { DASH_SPEED } else { SPEED };
                movable.velocity = movement_norm * speed;
                if let Some(mut sprite) = maybe_sprite {
                    // If we have a sprite, flip x if we moved in a negative x dir
                    // Might not be necessary
                    let x_movement = movement_norm.x;
                    match x_movement.signum() {
                        x if x == 1.0 && x_movement > f32::EPSILON => {
                            sprite.flip_x = false;
                        }
                        // cuz neg
                        x if x == -1.0 && x_movement < f32::EPSILON => {
                            sprite.flip_x = true;
                        }
                        _ => {}
                    }
                }
            } else {
                movable.velocity *= 0.1;
            }

            // If we have a movement pointer, update it
            if let Some(mut pointer) = maybe_pointer {
                pointer.0 = movable.velocity.normalize_or_zero();
                pointer.1 = movable.velocity.length();
            }
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub(crate) enum PlayerAction {
    Move,

    LightAttack,
    HeavyAttack,

    // Dash variants, both should just enable dash, but one
    // needs to be held, while the other toggles dash on and off
    HoldDash,
    ToggleDash,

    // X axis - switching between cards
    // +Y axis - above deadzone, use card
    // -Y axis - above deadzone, held, exhaust hand
    Hand,
}

#[derive(Component, Default)]
pub(crate) struct Player;

#[derive(Bundle, Default, LdtkEntity)]
pub(crate) struct PlayerBlueprint {
    pub(crate) player: Player,
}

#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    cam_target: camera::CameraTarget,
    input_manager_bundle: InputManagerBundle<PlayerAction>,
    // For now, use a mesh
    // mesh: MaterialMesh2dBundle<ColorMaterial>,
    sprite_sheet: SpriteSheetBundle,
    // used to render a pointed arrow
    movement_dir: movement_pointer::MovementDirection,
    dash: DashState,
    moveable: movement::Movable,
    collidable: collision::Collidable,
    // moonshine children
    children: SpawnChildren,
}

pub(crate) fn process_player_start(
    mut commands: Commands,
    new_entity_instances: Query<(Entity, &Transform), Added<Player>>,
    // mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    player_assets: Res<PlayerAssets>,
) {
    for (entity, transform) in new_entity_instances.iter() {
        commands.entity(entity).insert(PlayerBundle {
            // Make ourself the default cam target
            cam_target: camera::CameraTarget::default(),
            input_manager_bundle: InputManagerBundle::with_map(default_input_map()),
            movement_dir: movement_pointer::MovementDirection(Vec2::X, 0.0),
            dash: DashState(false),
            sprite_sheet: SpriteSheetBundle {
                texture: player_assets.texture.clone(),
                atlas: TextureAtlas {
                    layout: player_assets.layout.clone(),
                    index: 0,
                },
                transform: (*transform) * Transform::default().with_translation(Vec3::Y * 8.0),
                ..default()
            },
            moveable: default(),
            collidable: collision::Collidable {
                offset: Vec2::Y * 8.0,
                extents: Vec2::splat(32.0),
            },
            // mesh: MaterialMesh2dBundle {
            //     mesh: meshes
            //         .add(Rectangle {
            //             half_size: (16.0, 24.0).into(),
            //         })
            //         .into(),
            //     // Be careful not to overwrite the LDtk transform
            //     // Change our origin
            //     transform: (*transform) * Transform::default().with_translation(Vec3::Y * 8.0),
            //     material: materials.add(Color::PURPLE),
            //     ..default()
            // },
            children: spawn_children(|cb| {
                cb.spawn(movement_pointer::MovementPointerBundle::new(
                    materials.add(Color::ORANGE_RED),
                ));
            }),
        });
    }
}

pub(crate) fn default_input_map() -> InputMap<PlayerAction> {
    let mut map = InputMap::default();
    map.insert(PlayerAction::Move, DualAxis::left_stick());
    map.insert(PlayerAction::Move, VirtualDPad::wasd());
    map.insert(PlayerAction::LightAttack, KeyCode::KeyZ);
    map.insert(PlayerAction::HeavyAttack, KeyCode::KeyX);
    map.insert(PlayerAction::HoldDash, GamepadButtonType::RightTrigger);
    map.insert(PlayerAction::HoldDash, KeyCode::ShiftLeft);
    map.insert(PlayerAction::HoldDash, KeyCode::ShiftRight);
    map.insert(PlayerAction::ToggleDash, KeyCode::KeyC);
    map.insert(PlayerAction::Hand, VirtualDPad::gamepad_face_buttons());
    map.insert(PlayerAction::Hand, VirtualDPad::arrow_keys());
    map
}
