use crate::simple_bt::composite::{Repeated, Sequence};
use crate::simple_bt::{BehaviorNode, BehaviorRunner, NodeResult};
use crate::utils::lerp_mix;
use crate::{camera, collision, movement, movement_pointer, GameState};
use crate::{fundsp_kira, player::PlayerAction, You};
use crate::{player::DashState, player::Player};

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_asset_loader::prelude::*;
use bevy_egui::{egui, EguiContexts};
use leafwing_input_manager::prelude::*;
use moonshine_spawn::spawn_children;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MakeABoxTrigger>()
            .add_systems(Startup, setup_debug)
            .add_systems(Update, make_a_box)
            .add_systems(Update, make_polly_think.before(movement::Movement))
            .add_systems(Update, debug_view_window.run_if(in_state(GameState::InRun)));
    }

    fn finish(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(GameState::Booting)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("debug.assets.ron")
                .load_collection::<DebugAssets>(),
        );
    }
}

/// Your worst nightmare
#[derive(Component)]
struct Polly;
#[derive(Component)]
struct PollyBrain {
    thinking: bool,
    runner: BehaviorRunner<(Vec2, Vec2)>,
}

// Make Polly, your *nightmare*
fn setup_debug(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Polly,
        PollyBrain {
            thinking: false,
            runner: BehaviorRunner::new(
                Repeated::new(
                    [
                        MoveTo {
                            speed: 250.,
                            goal: Vec2::X * 300.0,
                        }
                        .arc(),
                        MoveTo {
                            speed: 250.,
                            goal: Vec2::NEG_X * 300.0,
                        }
                        .arc(),
                    ]
                    .into_iter()
                    .collect::<Sequence<_>>()
                    .arc(),
                )
                .arc(),
            ),
        },
        movement_pointer::MovementDirection(Vec2::ZERO, 0.0),
        MaterialMesh2dBundle {
            mesh: meshes
                .add(Rectangle {
                    half_size: (16.0, 24.0).into(),
                })
                .into(),
            // Be careful not to overwrite the LDtk transform
            // Change our origin
            material: materials.add(Color::PURPLE),
            // Polly is above us all.
            transform: Transform::from_translation(Vec3::Z * 100.0),
            ..default()
        },
        collision::Collidable {
            offset: Vec2::Y * 8.0,
            extents: Vec2::splat(32.0),
        },
        spawn_children(|cb| {
            cb.spawn(movement_pointer::MovementPointerBundle::new(
                materials.add(Color::ORANGE_RED),
            ));
        }),
    ));
}

fn make_polly_think(
    mut query: Query<(&mut movement::Movable, &Transform, &mut PollyBrain), With<Polly>>,
) {
    let Ok((mut movable, transform, mut thunk)) = query.get_single_mut() else {
        return;
    };

    if thunk.thinking {
        let mut context = (transform.translation.truncate(), movable.velocity);
        if let Some(res) = thunk.runner.proceed(&mut context) {
            info!("Thunking complete step: {context:?} {res}");
        }
        movable.velocity = context.1;
    }
}

#[derive(Event)]
struct MakeABoxTrigger {
    mass: f32,
}

const MIN_BOX_MASS: f32 = 0.25;
const MAX_BOX_MASS: f32 = 60.0;
fn make_a_box(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut reader: EventReader<MakeABoxTrigger>,
) {
    const LIGHT_COLOR: Color = Color::TURQUOISE;
    const HEAVY_COLOR: Color = Color::PURPLE;

    for box_trigger in reader.read() {
        commands.spawn((
            movement_pointer::MovementDirection(Vec2::ZERO, 0.0),
            MaterialMesh2dBundle {
                mesh: meshes
                    .add(Rectangle {
                        half_size: (16.0, 24.0).into(),
                    })
                    .into(),
                // Be careful not to overwrite the LDtk transform
                // Change our origin
                material: materials.add(lerp_mix(
                    LIGHT_COLOR,
                    HEAVY_COLOR,
                    (box_trigger.mass - MIN_BOX_MASS) / MAX_BOX_MASS,
                )),
                // Polly is above us all.
                transform: Transform::from_translation(Vec3::Z * 100.0),
                ..default()
            },
            collision::Collidable {
                offset: Vec2::Y * 8.0,
                extents: Vec2::splat(32.0),
            },
            movement::Movable {
                mass: dbg!(box_trigger.mass),
                ..default()
            },
            spawn_children(|cb| {
                cb.spawn(movement_pointer::MovementPointerBundle::new(
                    materials.add(Color::ORANGE_RED),
                ));
            }),
        ));
    }
}

#[derive(Resource, AssetCollection)]
struct DebugAssets {
    #[asset(key = "debug_sfxr", collection(typed))]
    debug_sfxr: Vec<Handle<fundsp_kira::Machine>>,
}

#[derive(Debug, Clone)]
struct MoveTo {
    speed: f32,
    goal: Vec2,
}

impl BehaviorNode<(Vec2, Vec2)> for MoveTo {
    fn tick(&self, (position, velocity): &mut (Vec2, Vec2)) -> NodeResult<(Vec2, Vec2)> {
        let dist = (self.goal - *position).length();
        const ERROR: f32 = 10.0;
        if dist <= ERROR {
            NodeResult::Success
        } else {
            let desired_vel = (self.goal - *position).normalize_or_zero() * self.speed;
            debug!("{desired_vel:?} {velocity:?}");
            let dvn = desired_vel.normalize_or_zero();
            let vn = velocity.normalize_or_zero();
            let dvl = desired_vel.length();
            let vl = velocity.length();
            const KEEP: f32 = 0.9;
            if dvn.dot(vn) <= 0.0 || vl < dvl {
                *velocity *= KEEP;
                *velocity += (desired_vel - *velocity) * (1.0 - KEEP);
            }
            NodeResult::Running(self.clone().arc())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn debug_view_window(
    query: Query<
        (
            Entity,
            &ActionState<PlayerAction>,
            &DashState,
            Option<&camera::CameraTarget>,
        ),
        With<Player>,
    >,
    mut polly: Query<
        (
            Entity,
            Option<&camera::CameraTarget>,
            Option<&mut movement::Movable>,
            &mut PollyBrain,
        ),
        With<Polly>,
    >,
    mut contexts: EguiContexts,
    you: Option<Res<You>>,
    mut track: ResMut<fundsp_kira::MainTrack>,
    mut rig: ResMut<camera::CameraRig>,
    assets: Res<DebugAssets>,

    machines: Res<Assets<fundsp_kira::Machine>>,
    mut commands: Commands,
    mut writer: EventWriter<MakeABoxTrigger>,

    mut last_debug_machine: Local<Option<Handle<fundsp_kira::Machine>>>,
    mut box_mass: Local<f32>,
) {
    egui::Window::new("Debug Window").show(contexts.ctx_mut(), |ui| {
        for handle in assets.debug_sfxr.iter() {
            if ui
                .button(
                    handle
                        .path()
                        .and_then(|ap| ap.path().to_str())
                        .unwrap_or("<unlabeled>"),
                )
                .clicked()
            {
                *last_debug_machine = Some(handle.clone());
                track.play(handle.clone());
            }
        }

        ui.collapsing("Box Mania", |ui| {
            ui.add(egui::Slider::new(&mut *box_mass, MIN_BOX_MASS..=MAX_BOX_MASS).text("Mass"));
            *box_mass = (*box_mass).clamp(MIN_BOX_MASS, MAX_BOX_MASS);
            if (*box_mass).is_nan() {
                *box_mass = MIN_BOX_MASS;
            }
            if ui.button("Make A Box").clicked() {
                writer.send(MakeABoxTrigger { mass: *box_mass });
            }
        });

        ui.collapsing("Polly Control", |ui| {
            let (polly_entity, cam_target, mut movable, mut thunk) = polly.single_mut();
            if cam_target.is_some() {
                if ui.button("Remove").clicked() {
                    commands
                        .entity(polly_entity)
                        .remove::<camera::CameraTarget>();
                }
            } else if ui.button("With You").clicked() {
                commands
                    .entity(polly_entity)
                    .insert(camera::CameraTarget::default());
            } else if ui.button("Without You").clicked() {
                commands
                    .entity(polly_entity)
                    .insert(camera::CameraTarget(1));
            }

            if let Some(movable) = movable.as_mut() {
                if ui.button("Static").clicked() {
                    commands.entity(polly_entity).remove::<movement::Movable>();
                }

                ui.add(
                    egui::Slider::new(&mut movable.mass, MIN_BOX_MASS..=MAX_BOX_MASS).text("Mass"),
                );
                movable.acceleration *= 0.1;
            } else if ui.button("Dynamic").clicked() {
                commands.entity(polly_entity).insert(movement::Movable {
                    mass: 20.0,
                    ..default()
                });
            }

            if ui
                .toggle_value(&mut thunk.thinking, "Has Thought")
                .changed()
            {
                if movable.is_none() && thunk.thinking {
                    commands.entity(polly_entity).insert(movement::Movable {
                        mass: 20.0,
                        ..default()
                    });
                }
            }
        });

        ui.collapsing("Debug Sfxr", |ui| {
            // Show our homework
            if let Some(sfxr) = last_debug_machine
                .as_ref()
                .and_then(|hnd| machines.get(hnd.clone()))
                .and_then(|m| m.userdata())
                .and_then(|ud| ud.downcast_ref::<crate::sfxr::Sfxr>())
            {
                ui.label(format!("{sfxr:#?}"));
            } else {
                ui.label("");
            }
        });

        ui.collapsing("Camera Rig", |ui| {
            let mut lag = rig.lag();
            if ui
                .add(egui::Slider::new(&mut lag, 0.0..=1.).text("Lag"))
                .changed()
            {
                rig.set_lag(lag);
            }

            ui.add(egui::Slider::new(&mut rig.targetting, 0..=1).text("Target"));

            let mut snap_dur_secs = rig.snap_duration.as_secs_f64();
            if ui
                .add(
                    egui::Slider::new(&mut snap_dur_secs, 60.0f64.recip()..=2.0).text("Snap Delta"),
                )
                .changed()
            {
                rig.snap_duration = std::time::Duration::from_secs_f64(snap_dur_secs);
            }
        });

        ui.collapsing("Audio Details", |ui| {
            // TODO Abstract this away into an egui widget for an *oscilloscope*
            use egui_plot::{Line, Plot, PlotPoints};
            let left_wave: PlotPoints = track
                .samples()
                .iter()
                .enumerate()
                .map(|(t, (s, _))| [t as f64, *s as f64])
                .collect();
            let left_line = Line::new(left_wave).color(egui::Color32::BLUE);
            let right_wave: PlotPoints = track
                .samples()
                .iter()
                .enumerate()
                .map(|(t, (_, s))| [t as f64, *s as f64])
                .collect();
            let right_line = Line::new(right_wave).color(egui::Color32::RED);
            ui.label("Main Track Oscilloscope");
            Plot::new("waveform")
                .view_aspect(4.0)
                .auto_bounds(egui::Vec2b::new(false, false))
                .show_grid(egui::Vec2b::new(false, true))
                .include_y(-std::f32::consts::SQRT_2.recip())
                .include_y(std::f32::consts::SQRT_2.recip())
                .include_x(0.0)
                .include_x(track.buffer_length() as f64)
                .show_axes(false)
                .show(ui, |plot_ui| {
                    plot_ui.line(left_line);
                    plot_ui.line(right_line);
                });
            let current_rms = track.rms().last().copied().unwrap_or((0.0, 0.0));
            ui.label(format!("Main Track RMS: {current_rms:?}"));
            ui.label(format!("Main Track SR: {}", track.sample_rate()));
        });

        if let Some(you) = you.and_then(|you| you.0) {
            ui.label(format!("You are {}.", you));
        } else {
            ui.label("There is no You.");
        }

        let Ok((player_entity, action_state, dash_state, maybe_cam_target)) = query.get_single()
        else {
            return;
        };

        if maybe_cam_target.is_some() {
            if ui.button("Detach Camera").clicked() {
                commands
                    .entity(player_entity)
                    .remove::<camera::CameraTarget>();
            }
        } else if ui.button("Attach Camera").clicked() {
            commands
                .entity(player_entity)
                .insert(camera::CameraTarget::default());
        }

        ui.label(if dash_state.0 {
            "Dashing"
        } else {
            "Not Dashing"
        });

        if let Some(dad) = action_state.clamped_axis_pair(&PlayerAction::Move) {
            ui.label(format!("X movement: {}", dad.x()));
            ui.label(format!("Y movement: {}", dad.y()));
        } else {
            ui.label("!! Move action not (properly) bound !!");
        }

        if let Some(hand_dad) = action_state.clamped_axis_pair(&PlayerAction::Hand) {
            _ = hand_dad;
            ui.label("Hand action connected!");
            ui.label(format!(
                "Hand actions: [{}]",
                [
                    (hand_dad.x() > 0.5).then_some("HandMoveRight"),
                    (hand_dad.x() < -0.5).then_some("HandMoveLeft"),
                    (hand_dad.y() > 0.5).then_some("HandUse"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ")
            ));
        } else {
            ui.label("!! Hand action not (properly) bound !!");
        }
    });
}
