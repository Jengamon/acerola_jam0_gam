use std::time::Duration;

use super::{post_process, LOGICAL_HEIGHT, LOGICAL_WIDTH};

use bevy::prelude::*;
use bevy_smooth_pixel_camera::components::PixelCamera;
use bevy_smooth_pixel_camera::viewport::ViewportSize;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // Shader plugins
            post_process::ChromaticAberrationPlugin,
        ))
        .init_resource::<CameraRig>()
        .add_systems(Startup, setup_camera)
        .add_systems(FixedUpdate, update_camera);
    }
}

#[derive(Resource)]
pub struct CameraRig {
    /// Which [`CameraTarget`](s) does the camera target?
    pub targetting: usize,
    precision: f32,
    /// how long should the lerp take?
    pub snap_duration: Duration,
}

impl CameraRig {
    /// Camera lag - [0.0, 1.0)
    ///
    /// The camera will nominally be within this percentage of the
    /// target within [`self::snap_duration`]
    pub fn lag(&self) -> f32 {
        self.precision
    }

    pub fn set_lag(&mut self, lag: f32) {
        self.precision = lag.clamp(0.0, 0.99);
    }
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            targetting: 0,
            precision: 0.7,
            snap_duration: Duration::from_secs_f32(10f32.recip()),
        }
    }
}

/// Attach this to something to make it the target of the camera
// TODO Maybe allow indexing, to switch targets? Might be cool for cutscenes.
#[derive(Component, Default)]
pub struct CameraTarget(pub usize);

#[derive(Component, Default)]
struct Camera;

#[derive(Bundle)]
struct CameraBundle {
    camera: Camera,
    camera_2d: Camera2dBundle,
    pixel_camera: PixelCamera,

    chroma_aberration: post_process::ChromaticAberattionSettings,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(CameraBundle {
        camera: Camera,
        camera_2d: Camera2dBundle::default(),
        pixel_camera: PixelCamera::from_size(ViewportSize::AutoMin {
            min_width: LOGICAL_WIDTH,
            min_height: LOGICAL_HEIGHT,
        }),
        chroma_aberration: post_process::ChromaticAberattionSettings {
            // intensity: 0.02,
            ..default()
        },
    });
}

fn update_camera(
    mut camera: Query<(&mut PixelCamera, &GlobalTransform), With<Camera>>,
    targets: Query<(&GlobalTransform, &CameraTarget)>,
    rig: Res<CameraRig>,
    time: Res<Time>,
) {
    // This is only 1 camera...
    let (mut camera, cam_transform) = camera.single_mut();
    // ...but soooo many targets
    let targeted = targets
        .iter()
        .filter(|(_, t)| t.0 == rig.targetting)
        .collect::<Vec<_>>();
    // Get the midpoint of all targeted items
    let goal = targeted
        .iter()
        .map(|(transform, _)| transform.translation())
        .reduce(|total, e| total + e)
        .map(|total| total / targeted.len() as f32);
    if let Some(goal) = goal {
        // Lerp the camera to player's global translation point
        let source = cam_transform.translation();
        // Taken from https://mastodon.social/@acegikmo/111931613710775864
        let lambda = -rig.snap_duration.as_secs_f32() / rig.precision.log2();
        let alpha = 2.0f32.powf(-time.delta_seconds() / lambda);
        let lerped = source * alpha + goal * (1.0 - alpha);
        camera.subpixel_pos = lerped.xy();
    }
}
