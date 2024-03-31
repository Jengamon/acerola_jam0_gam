// Bevy ends up triggering this for very basic types, so turn it off for now
#![allow(clippy::type_complexity)]

mod camera;
mod collision;
mod cutscene;
mod debug;
mod fundsp_kira;
mod movement;
mod movement_pointer;
mod player;
mod post_process;
mod sfxr;
mod simple_bt;

use bevy::{prelude::*, sprite::Anchor};
// use bevy::sprite::MaterialMesh2dBundle;
use bevy_asset_loader::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_kira_audio::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_smooth_pixel_camera::PixelCameraPlugin;
use bevy_tweening::TweeningPlugin;
use iyes_progress::prelude::*;
use leafwing_input_manager::prelude::*;
use moonshine_spawn::SpawnPlugin;

// How big is our screen in game?
const LOGICAL_WIDTH: u32 = 800;
const LOGICAL_HEIGHT: u32 = 600;

#[cfg(target_arch = "wasm32")]
fn app() -> App {
    // According to bevy issue 10157, this fixes the problem (and it does for me for now)
    let mut app = App::new();
    app.insert_resource(bevy::asset::AssetMetaCheck::Never);
    app
}

#[cfg(not(target_arch = "wasm32"))]
fn app() -> App {
    App::new()
}

fn main() {
    app()
        .add_plugins((
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#game-canvas".into()),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // some day...
                    // mode: AssetMode::Processed,
                    ..default()
                }),
            InputManagerPlugin::<player::PlayerAction>::default(),
            AudioPlugin,
            TweeningPlugin,
            PixelCameraPlugin,
            ShapePlugin,
            EguiPlugin,
            LdtkPlugin,
            SpawnPlugin,
        ))
        .add_plugins((
            // Our "subplugins"
            camera::CameraPlugin,
            debug::DebugPlugin,
            sfxr::SfxrPlugin,
            movement::MovementPlugin,
            collision::CollisionPlugin,
            movement_pointer::MovementPointerPlugin,
            // Fundsp?
            fundsp_kira::FundspAudioPlugin,
        ))
        .add_plugins(AcerolaGame0)
        .run()
}

pub(crate) mod utils {
    use bevy::prelude::Color;
    /// alpha is clamped to [0.0, 1.0]
    pub fn lerp_mix(a: Color, b: Color, alpha: f32) -> Color {
        use palette::{
            blend::{BlendWith, PreAlpha},
            LinSrgb, Srgba,
        };
        type PreRgba = PreAlpha<LinSrgb<f32>>;
        let alpha = alpha.clamp(0.0, 1.0);
        let a_rgb = Srgba::from(a.as_rgba_f32()).into_linear();
        let b_rgb = Srgba::from(b.as_rgba_f32()).into_linear();
        let mixed = a_rgb.blend_with(b_rgb, |a: PreRgba, b: PreRgba| {
            // lerp em colors togethter
            PreAlpha {
                color: LinSrgb::new(
                    a.red + (b.red - a.red) * alpha,
                    a.green + (b.green - a.green) * alpha,
                    a.blue + (b.blue - a.blue) * alpha,
                ),

                alpha: a.alpha + (b.alpha - a.alpha) * alpha,
            }
        });
        Color::rgba(mixed.red, mixed.green, mixed.blue, mixed.alpha)
    }
}

struct AcerolaGame0;

// TODO Don't be afraid of Plugins that add Plugins! Modularize!
impl Plugin for AcerolaGame0 {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::rgb(0.015, 0.01, 0.01)))
            .init_state::<GameState>()
            .insert_resource(LevelSelection::index(0))
            .init_resource::<You>()
            .register_ldtk_entity::<player::PlayerBlueprint>("PlayerStart")
            // Mark Loading as a loading state, and progress to InGame once done
            .add_plugins(ProgressPlugin::new(GameState::Booting).continue_to(GameState::StartRun))
            .add_plugins(ProgressPlugin::new(GameState::StartRun).continue_to(GameState::InRun))
            .add_loading_state(
                LoadingState::new(GameState::Booting)
                    .load_collection::<PlayerAssets>()
                    .load_collection::<AudioLoadTest>()
                    .on_failure_continue_to_state(GameState::FailBoot),
            )
            .add_systems(OnEnter(GameState::FailBoot), fail_boot)
            .add_systems(OnEnter(GameState::StartRun), new_run_new_you)
            .add_systems(OnEnter(GameState::InRun), setup_level)
            .add_systems(
                Update,
                (loading_screen_progress,)
                    .run_if(in_state(GameState::Booting))
                    .after(LoadingStateSet(GameState::Booting)),
            )
            .add_systems(OnEnter(GameState::Booting), create_loading_screen_bar)
            .add_systems(OnExit(GameState::Booting), clean_loading_screen_bar)
            .add_systems(
                Update,
                (
                    player::process_player_start,
                    player::handle_dashing,
                    player::move_player,
                )
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(Update, update_settings.run_if(in_state(GameState::InRun)))
            .add_systems(OnEnter(GameState::InRun), test_audio_loop)
            // configure our fixed timestep schedule to run 60 times per second
            .insert_resource(Time::<Fixed>::from_seconds(60.0f64.recip()));
    }
}
#[derive(Component)]
struct LoadingScreenBar;
fn create_loading_screen_bar(mut commands: Commands) {
    commands
        .spawn((
            LoadingScreenBar,
            ShapeBundle {
                path: GeometryBuilder::build_as(&shapes::Rectangle {
                    origin: shapes::RectangleOrigin::TopLeft,
                    extents: Vec2::new(LOGICAL_WIDTH as f32, 40.0),
                }),
                spatial: SpatialBundle {
                    transform: Transform::default()
                        .with_translation(Vec3::new(
                            LOGICAL_WIDTH as f32 / -2.0,
                            // LOGICAL_HEIGHT as f32 / 2.0,
                            LOGICAL_HEIGHT as f32 / -2.0 + 80.0 * 1.5,
                            -1.0,
                        ))
                        .with_scale(Vec3::ONE - Vec3::X),
                    ..default()
                },
                ..default()
            },
            Fill::color(Color::ANTIQUE_WHITE),
        ))
        .with_children(|cb| {
            cb.spawn(Text2dBundle {
                text: Text::from_section(
                    "Loading...",
                    TextStyle {
                        font_size: 30.0,
                        color: Color::BLACK,
                        ..default()
                    },
                ),
                text_anchor: Anchor::CenterLeft,
                transform: Transform::from_translation(Vec3::Z + Vec3::NEG_Y * 20.0),
                ..default()
            });
        });
}
fn clean_loading_screen_bar(mut commands: Commands, bar: Query<Entity, With<LoadingScreenBar>>) {
    let entity = bar.single();
    commands.entity(entity).despawn_recursive();
}
fn loading_screen_progress(
    progress: Option<Res<ProgressCounter>>,
    mut last_done: Local<u32>,
    mut lb: Query<&mut Transform, With<LoadingScreenBar>>,
) {
    if let Some(progress) = progress.map(|counter| counter.progress()) {
        if progress.done > *last_done {
            let mut lbt = lb.single_mut();
            *last_done = progress.done;
            info!("Changed progress: {:?}", progress);
            lbt.scale.x = progress.done as f32 / progress.total as f32;
        }
    }
}

fn fail_boot(mut commands: Commands) {
    commands.spawn(Text2dBundle {
        text: Text::from_section(
            "Something failed to load",
            TextStyle {
                font_size: 30.0,
                color: Color::RED,
                ..default()
            },
        ),
        ..default()
    });
}

#[derive(Resource, AssetCollection)]
struct AudioLoadTest {
    #[asset(path = "mfvart 2023-12-19 2023-12-19 1344.flac")]
    test_audio: Handle<AudioSource>,
}

fn test_audio_loop(audio: Res<Audio>, audio_load_test: Res<AudioLoadTest>) {
    audio
        .play(audio_load_test.test_audio.clone())
        .with_volume(0.3)
        .looped();
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    Booting,
    FailBoot,
    StartRun,
    InRun,
}

/// While in a run, this is who you are
#[derive(Resource, Default)]
struct You(Option<ulid::Ulid>);

impl You {
    pub fn new() -> Self {
        Self(Some(ulid::Ulid::new()))
    }
}

/// Create a new You every run
fn new_run_new_you(mut you: ResMut<You>) {
    *you = You::new();
}

#[derive(AssetCollection, Resource)]
struct PlayerAssets {
    #[asset(path = "dum_player.png")]
    texture: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 32., tile_size_y = 48., columns = 4, rows = 8))]
    layout: Handle<TextureAtlasLayout>,
}

// Change the intensity over time to show that the effect is controlled from the main world
fn update_settings(
    mut settings: Query<&mut post_process::ChromaticAberattionSettings>,
    time: Res<Time>,
) {
    for mut setting in &mut settings {
        let mut intensity = time.elapsed_seconds().sin();
        // Make it loop periodically
        intensity = intensity.sin();
        // Remap it to 0..1 because the intensity can't be negative
        intensity = intensity * 0.5 + 0.5;
        // Scale it to a more reasonable level
        intensity *= 0.015;

        // Set the intensity.
        // This will then be extracted to the render world and uploaded to the gpu automatically by the [`UniformComponentPlugin`]
        setting.intensity = intensity;

        // also for fun, make red rotate
        let angle = Vec2::from_angle(std::f32::consts::FRAC_PI_3 * time.delta_seconds());
        setting.red_offset = angle.rotate(setting.red_offset);
    }
}

pub(crate) fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("test_map.ldtk"),
        ..Default::default()
    });
}
