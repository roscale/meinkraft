use nalgebra::{Vector3, clamp};
use nalgebra_glm::{vec2, Vec3, vec3, pi};
use num_traits::Zero;

use crate::aabb::{AABB, get_block_aabb};
use crate::chunk_manager::ChunkManager;
use crate::constants::{HORIZONTAL_ACCELERATION, JUMP_IMPULSE, MAX_VERTICAL_VELOCITY, PLAYER_EYES_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH, WALKING_SPEED, ON_GROUND_FRICTION, IN_AIR_FRICTION, MOUSE_SENSITIVITY_X, MOUSE_SENSITIVITY_Y, FLYING_SPEED, SNEAKING_SPEED, SPRINTING_SPEED, FLYING_SPRINTING_SPEED, FLYING_TRIGGER_INTERVAL, SPRINTING_TRIGGER_INTERVAL, FOV};
use crate::input::InputCache;
use crate::util::Forward;
use crate::physics::{Interpolatable, Interpolator};
use std::time::Instant;

pub struct PlayerState {
    pub rotation: Vec3,
    pub camera_height: Interpolator<f32>,
    pub fov: Interpolator<f32>,
    pub is_sneaking: bool,
    pub is_sprinting: bool,
    pub is_flying: bool,

    pub(crate) jump_last_executed: Instant,
    pub(crate) fly_throttle: bool,
    pub(crate) fly_last_toggled: Instant,
    pub(crate) sprint_throttle: bool,
    pub(crate) sprint_last_toggled: Instant,
}

impl PlayerState {
    pub fn new() -> Self {
        PlayerState {
            rotation: vec3(0.0, 0.0, 0.0), // In radians
            camera_height: Interpolator::new(1. / 30., PLAYER_EYES_HEIGHT),
            fov: Interpolator::new(1.0 / 30.0, FOV),
            is_sneaking: false,
            is_sprinting: false,
            is_flying: false,

            jump_last_executed: Instant::now(),
            fly_throttle: false,
            fly_last_toggled: Instant::now(),
            sprint_throttle: false,
            sprint_last_toggled: Instant::now(),
        }
    }

    pub fn rotate_camera(&mut self, horizontal: f32, vertical: f32) {
        self.rotation.y += horizontal / 100.0 * MOUSE_SENSITIVITY_X;
        self.rotation.x -= vertical / 100.0 * MOUSE_SENSITIVITY_Y;
        // Limit vertical movement
        self.rotation.x = clamp(
            self.rotation.x,
            -pi::<f32>() / 2.0 + 0.0001,
            pi::<f32>() / 2.0 - 0.0001);
    }

    // pub fn on_update(&mut self, t: Instant, input_cache: &InputCache, player_physics_state: &PlayerPhysicsState) {
    //
    // }
}

#[derive(Clone)]
pub struct PlayerPhysicsState {
    pub position: Vec3,
    pub aabb: AABB,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub is_on_ground: bool,
}

impl PlayerPhysicsState {
    pub fn new_at_position(position: Vec3) -> Self {
        PlayerPhysicsState {
            position,
            aabb: {
                let mins = vec3(position.x - PLAYER_HALF_WIDTH, position.y, position.z - PLAYER_HALF_WIDTH);
                let maxs = vec3(position.x + PLAYER_HALF_WIDTH, position.y + PLAYER_HEIGHT, position.z + PLAYER_HALF_WIDTH);
                AABB::new(mins, maxs)
            },
            velocity: vec3(0.0, 0.0, 0.0),
            acceleration: vec3(0.0, 0.0, 0.0),
            is_on_ground: false,
        }
    }
}

impl Interpolatable for PlayerPhysicsState {
    fn interpolate(&self, alpha: f32, other: &Self) -> Self {
        let interpolate_vec3 = |from: &Vec3, to: &Vec3| {
            alpha * from + (1.0 - alpha) * to
        };

        Self {
            position: interpolate_vec3(&self.position, &other.position),
            aabb: AABB {
                mins: interpolate_vec3(&self.aabb.mins, &other.aabb.mins),
                maxs: interpolate_vec3(&self.aabb.maxs, &other.aabb.maxs),
            },
            velocity: interpolate_vec3(&self.velocity, &other.velocity),
            acceleration: interpolate_vec3(&self.acceleration, &other.acceleration),
            is_on_ground: other.is_on_ground,
        }
    }
}

impl PlayerPhysicsState {
    pub fn apply_keyboard_mouvement(&mut self, player_properties: &mut PlayerState, input_cache: &InputCache) {
        let rotation = &player_properties.rotation;
        if player_properties.is_flying {
            if input_cache.is_key_pressed(glfw::Key::Space) {
                self.acceleration = vec3(0.0, 100.0, 0.0);
            }
            if input_cache.is_key_pressed(glfw::Key::LeftShift) {
                self.acceleration = vec3(0.0, -100.0, 0.0);
            }
        }

        // Jump
        if input_cache.is_key_pressed(glfw::Key::Space) {
            let now = Instant::now();
            if now.duration_since(player_properties.jump_last_executed).as_secs_f32() >= 0.475 {
                if self.is_on_ground {
                    self.velocity.y = *JUMP_IMPULSE;
                    player_properties.jump_last_executed = now;
                }
            }
        }
        // Walk
        let mut horizontal_acceleration = vec3(0.0, 0.0, 0.0);

        if input_cache.is_key_pressed(glfw::Key::W) {
            horizontal_acceleration += -rotation.forward().cross(&Vector3::y()).cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(glfw::Key::S) {
            horizontal_acceleration += rotation.forward().cross(&Vector3::y()).cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(glfw::Key::A) {
            horizontal_acceleration += -rotation.forward().cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(glfw::Key::D) {
            horizontal_acceleration += rotation.forward().cross(&Vector3::y())
        }

        if horizontal_acceleration.norm_squared() != 0.0 {
            let horizontal_acceleration = horizontal_acceleration.normalize().scale(HORIZONTAL_ACCELERATION);
            self.acceleration += horizontal_acceleration;
        }
    }

    pub fn get_colliding_block_coords(&self, chunk_manager: &ChunkManager) -> Option<Vec3> {
        let player_mins = &self.aabb.mins;
        let player_maxs = &self.aabb.maxs;

        let block_mins = vec3(
            player_mins.x.floor() as i32, player_mins.y.floor() as i32, player_mins.z.floor() as i32,
        );
        let block_maxs = vec3(
            player_maxs.x.floor() as i32, player_maxs.y.floor() as i32, player_maxs.z.floor() as i32,
        );

        // We query all the blocks around the player to check whether it's colliding with one of them
        let mut colliding_block = None;
        for y in block_mins.y..=block_maxs.y {
            for z in block_mins.z..=block_maxs.z {
                for x in block_mins.x..=block_maxs.x {
                    if let Some(block) = chunk_manager.get_block(x, y, z) {
                        if !block.is_air() {
                            let block_aabb = get_block_aabb(&vec3(x as f32, y as f32, z as f32));
                            if self.aabb.intersects(&block_aabb) {
                                colliding_block = Some(vec3(x as f32, y as f32, z as f32));
                                break;
                            }
                        }
                    }
                }
            }
        }
        colliding_block
    }

    pub fn separate_from_block(&mut self, v: &Vec3, block_coords: &Vec3) -> bool {
        let mut is_player_on_ground = false;
        let block_aabb = get_block_aabb(&block_coords);

        if !v.x.is_zero() {
            if v.x < 0.0 {
                // I've opted to create a new AABB instead of translating the old one
                // because of the imprecision of floats.
                self.aabb = AABB::new(
                    vec3(block_aabb.maxs.x, self.aabb.mins.y, self.aabb.mins.z),
                    vec3(block_aabb.maxs.x + PLAYER_WIDTH, self.aabb.maxs.y, self.aabb.maxs.z));
            } else {
                self.aabb = AABB::new(
                    vec3(block_aabb.mins.x - PLAYER_WIDTH, self.aabb.mins.y, self.aabb.mins.z),
                    vec3(block_aabb.mins.x, self.aabb.maxs.y, self.aabb.maxs.z));
            }
            self.velocity.x = 0.0
        }

        if !v.y.is_zero() {
            if v.y < 0.0 {
                self.aabb = AABB::new(
                    vec3(self.aabb.mins.x, block_aabb.maxs.y, self.aabb.mins.z),
                    vec3(self.aabb.maxs.x, block_aabb.maxs.y + PLAYER_HEIGHT, self.aabb.maxs.z));
                is_player_on_ground = true;
            } else {
                self.aabb = AABB::new(
                    vec3(self.aabb.mins.x, block_aabb.mins.y - PLAYER_HEIGHT, self.aabb.mins.z),
                    vec3(self.aabb.maxs.x, block_aabb.mins.y, self.aabb.maxs.z));
            }
            self.velocity.y = 0.0;
        }

        if !v.z.is_zero() {
            if v.z < 0.0 {
                self.aabb = AABB::new(
                    vec3(self.aabb.mins.x, self.aabb.mins.y, block_aabb.maxs.z),
                    vec3(self.aabb.maxs.x, self.aabb.maxs.y, block_aabb.maxs.z + PLAYER_WIDTH));
            } else {
                self.aabb = AABB::new(
                    vec3(self.aabb.mins.x, self.aabb.mins.y, block_aabb.mins.z - PLAYER_WIDTH),
                    vec3(self.aabb.maxs.x, self.aabb.maxs.y, block_aabb.mins.z));
            }
            self.velocity.z = 0.0
        }
        is_player_on_ground
    }

    pub fn apply_friction(&mut self, dt: f32, vertically: bool) {
        let friction = if self.is_on_ground {
            ON_GROUND_FRICTION
        } else {
            IN_AIR_FRICTION
        };

        // We apply friction if the player is either slowing down (a = 0) or
        // walks in the opposite direction
        if self.acceleration.x.is_zero() || self.acceleration.x.signum() != self.velocity.x.signum() {
            self.velocity.x -= friction * self.velocity.x * dt;
        }
        if self.acceleration.z.is_zero() || self.acceleration.z.signum() != self.velocity.z.signum() {
            self.velocity.z -= friction * self.velocity.z * dt;
        }
        if vertically {
            if self.acceleration.y.is_zero() || self.acceleration.y.signum() != self.velocity.y.signum() {
                self.velocity.y -= ON_GROUND_FRICTION * self.velocity.y * dt;
            }
        }
    }

    pub fn limit_velocity(&mut self, player_properties: &PlayerState) {
        // Limit the horizontal speed
        let mut horizontal_vel = vec2(self.velocity.x, self.velocity.z);
        let speed = horizontal_vel.magnitude();

        let max_speed = if player_properties.is_flying {
            self.velocity.y = clamp(self.velocity.y, -8.0, 8.0);
            if player_properties.is_sprinting {
                FLYING_SPRINTING_SPEED
            } else {
                FLYING_SPEED
            }
        } else {
            if player_properties.is_sprinting {
                SPRINTING_SPEED
            } else if player_properties.is_sneaking {
                SNEAKING_SPEED
            } else {
                WALKING_SPEED
            }
        };

        if speed > max_speed {
            horizontal_vel = horizontal_vel.scale(max_speed / speed);
        }

        self.velocity.x = horizontal_vel.x;
        self.velocity.z = horizontal_vel.y;

        // Limit the free falling speed (vertical)
        // https://www.planetminecraft.com/blog/the-acceleration-of-gravity-in-minecraft-and-terminal-velocity/
        if self.velocity.y < -MAX_VERTICAL_VELOCITY {
            self.velocity.y = -MAX_VERTICAL_VELOCITY;
        }
    }
}


