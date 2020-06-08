use std::time;

use std::time::Instant;

/// Fixed timestep physics simulation using the following method:
/// https://gafferongames.com/post/fix_your_timestep/
/// With this method, the physics are always deterministic and work independently
/// of the performance of the game

pub trait Interpolatable {
    fn interpolate(&self, alpha: f32, other: &Self) -> Self;
}

impl Interpolatable for f32 {
    fn interpolate(&self, alpha: f32, other: &Self) -> Self {
        self * alpha + (1.0 - alpha) * other
    }
}

pub struct Interpolator<T: Clone + Interpolatable> {
    pub t: f32,
    pub dt: f32,
    pub current_time: time::Instant,
    pub accumulator: f32,
    pub previous_state: T,
    pub current_state: T,
    pub interpolated_state: T,
}

impl <T: Default + Clone + Interpolatable> Default for Interpolator<T> {
    fn default() -> Self {
        Self::new(0., T::default())
    }
}

impl<T: Clone + Interpolatable> Interpolator<T> {
    pub fn new(dt: f32, initial_state: T) -> Self {
        Self {
            t: 0.0,
            dt,
            current_time: time::Instant::now(),
            accumulator: 0.0,
            previous_state: initial_state.clone(),
            current_state: initial_state.clone(),
            interpolated_state: initial_state
        }
    }

    pub fn get_latest_state(&self) -> &T {
        &self.current_state
    }

    pub fn get_latest_state_mut(&mut self) -> &mut T {
        &mut self.current_state
    }

    pub fn get_interpolated_state(&self) -> &T {
        &self.interpolated_state
    }

    /// Advances the physics for a given state.
    pub fn step(&mut self, time: Instant, integrate: &mut dyn FnMut(&T, f32, f32) -> T) {
        let now = time;
        let mut frame_time = now.saturating_duration_since(self.current_time).as_secs_f32();
        if frame_time > 0.25 {
            frame_time = 0.25;
        }
        self.current_time = now;
        self.accumulator += frame_time;

        while self.accumulator >= self.dt {
            self.previous_state = self.current_state.clone();
            self.current_state = integrate(&self.previous_state, self.t, self.dt);
            self.t += self.dt;
            self.accumulator -= self.dt;
        }

        let alpha = self.accumulator / self.dt;
        self.interpolated_state = self.current_state.interpolate(alpha, &self.previous_state);
    }
}

impl Interpolator<f32> {
    pub fn interpolate_fov(&mut self, time: Instant, target_fov: f32) {
        self.step(time, &mut |&fov, _t, dt| {
            let convergence = 10.0;
            convergence * dt * target_fov + (1.0 - convergence * dt) * fov
        });
    }
}

impl Interpolator<f32> {
    pub fn interpolate_camera_height(&mut self, time: Instant, target_camera_height: f32) {
        self.step(time, &mut |&camera_height, _t, dt| {
            let convergence = 20.0;
            convergence * dt * target_camera_height + (1.0 - convergence * dt) * camera_height
        });
    }
}