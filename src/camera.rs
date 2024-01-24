use crate::prelude::*;

/// A resource for the state of the in-game smooth camera.
#[derive(Resource)]
pub struct TrackingCamera {
    /// The position in world space of the origin of this camera.
    pub position: Vec2,

    /// The current target of the camera; what it smoothly focuses on.
    pub target: Vec2,

    /// The half-size of the rectangle around the center of the screen where
    /// the player can move without the camera retargeting. When the player
    /// leaves this rectangle, the camera will retarget to include the player
    /// back into this region of the screen.
    pub tracking_size: Vec2,

    /// The half-size of the rectangle around the center of the screen where
    /// the camera will smoothly interpolate. If the player leaves this region,
    /// the camera will clamp to keep the player within it.
    pub clamp_size: Vec2,

    /// A dead distance from the edge of the tracking region to the player
    /// where the camera will not perform any tracking, even if the player is
    /// minutely outside of the tracking region. This is provided so that the
    /// camera can recenter even if the player has not moved since a track.
    pub dead_zone: Vec2,

    /// The proportion (between 0.0-1.0) that the camera reaches its target
    /// from its initial position during a second's time.
    pub speed: f64,

    /// A timeout to recenter the camera on the player even if the player has
    /// not left the tracking rectangle.
    pub recenter_timeout: f32,

    /// The duration in seconds since the player has left the tracking rectangle.
    ///
    /// When this duration reaches `recenter_timeout`, the player will be
    /// recentered.
    pub last_track: f32,
}

impl Default for TrackingCamera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            target: Vec2::ZERO,
            tracking_size: vec2(16.0, 9.0),
            clamp_size: vec2(48.0, 28.0),
            dead_zone: Vec2::splat(0.1),
            speed: 0.98,
            recenter_timeout: 3.0,
            last_track: 0.0,
        }
    }
}

impl TrackingCamera {
    /// Update the camera with the current position and this frame's delta time.
    pub fn update(&mut self, player_pos: Vec2, dt: f64) {
        // update target with player position
        self.track_player(player_pos);

        // track time since last time we had to track the player
        let new_last_track = self.last_track + dt as f32;

        // test if we've triggered a recenter
        if self.last_track < self.recenter_timeout && new_last_track > self.recenter_timeout {
            // target the player
            self.target = player_pos;
        }

        // update the duration since last track
        self.last_track = new_last_track;

        // lerp the current position towards the target
        // correct lerp degree using delta time
        // perform pow() with high precision
        let lerp = 1.0 - (1.0 - self.speed).powf(dt) as f32;
        self.position = self.position.lerp(self.target, lerp);
    }

    /// Helper function to clamp a rectangle (given as a half-size at the
    /// origin) so that a point lays within it. Returns an offset to apply to
    /// the rectangle, if any was required.
    pub fn clamp_rect(half_size: Vec2, point: Vec2) -> Option<Vec2> {
        let mut ox = None;
        let mut oy = None;

        if point.x > half_size.x {
            ox = Some(point.x - half_size.x);
        } else if point.x < -half_size.x {
            ox = Some(point.x + half_size.x);
        }

        if point.y > half_size.y {
            oy = Some(point.y - half_size.y);
        } else if point.y < -half_size.y {
            oy = Some(point.y + half_size.y);
        }

        if let (None, None) = (ox, oy) {
            None
        } else {
            Some(vec2(ox.unwrap_or(0.0), oy.unwrap_or(0.0)))
        }
    }

    pub fn track_player(&mut self, player_pos: Vec2) {
        // get current relative position to player
        let rel_pos = player_pos - self.position;

        // track the player and reset last track if change was necessary
        if let Some(offset) = Self::clamp_rect(self.tracking_size, rel_pos) {
            // skip tracking if it falls within the dead zone
            if !(self.dead_zone).cmpgt(offset.abs()).all() {
                self.target = self.position + offset;
                self.last_track = 0.0;
            }
        }

        // clamp the player within the screen
        if let Some(offset) = Self::clamp_rect(self.clamp_size, rel_pos) {
            self.position += offset;
        }
    }
}

pub fn update_camera(
    query: Query<&Transform, With<Player>>,
    mut camera_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut tracking: ResMut<TrackingCamera>,
    time: Res<Time>,
) {
    let transform = query.single();
    let mut camera_transform = camera_q.single_mut();
    let dt = time.delta_seconds_f64();
    tracking.update(transform.translation.xy(), dt);
    camera_transform.translation = tracking.position.extend(2.0);
}

//TODO Make this event based
fn on_resize_system(
    mut camera: Query<&mut Transform, With<Camera>>,
    window: Query<&Window>,
    zoom: Res<Zoom>,
) {
    let mut camera_transform = camera.single_mut();
    let Ok(window) = window.get_single() else {
        return;
    };

    let x = 1920. / window.width() * zoom.0;
    let y = 1080. / window.height() * zoom.0;

    camera_transform.scale.x = x.min(y);
    camera_transform.scale.y = x.min(y);
}

#[derive(Resource)]
pub struct Zoom(pub f32);

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_camera, on_resize_system).run_if(in_state(GameState::Game)),
        )
        .insert_resource(Zoom(0.23))
        .insert_resource(TrackingCamera::default());
    }
}
