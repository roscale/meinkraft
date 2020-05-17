use std::ops::{Add, Mul};

use glfw::Key;
use nalgebra::{Vector3, clamp};
use nalgebra_glm::{vec2, Vec3, vec3, pi};
use num_traits::Zero;

use crate::aabb::{AABB, get_block_aabb};
use crate::chunk_manager::ChunkManager;
use crate::constants::{HORIZONTAL_ACCELERATION, JUMP_IMPULSE, MAX_VERTICAL_VELOCITY, PLAYER_EYES_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH, WALKING_SPEED, ON_GROUND_FRICTION, IN_AIR_FRICTION, MOUSE_SENSITIVITY_X, MOUSE_SENSITIVITY_Y};
use crate::input::InputCache;
use crate::util::Forward;

pub struct PlayerProperties {
    pub rotation: Vec3
}

impl PlayerProperties {
    pub fn new() -> Self {
        PlayerProperties {
            rotation: vec3(0.0, 0.0, 0.0) // In radians
        }
    }
}

impl PlayerProperties {
    pub fn rotate_camera(&mut self, horizontal: f32, vertical: f32) {
        self.rotation.y += horizontal / 100.0 * MOUSE_SENSITIVITY_X;
        self.rotation.x -= vertical / 100.0 * MOUSE_SENSITIVITY_Y;
        // Limit vertical movement
        self.rotation.x = clamp(
            self.rotation.x,
            -pi::<f32>() / 2.0 + 0.0001,
            pi::<f32>() / 2.0 - 0.0001);
    }
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

    pub fn get_camera_position(&self) -> Vec3 {
        self.position + vec3(0.0, PLAYER_EYES_HEIGHT, 0.0)
    }
}

impl Mul<f32> for PlayerPhysicsState {
    type Output = PlayerPhysicsState;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.position *= rhs;
        self.acceleration *= rhs;
        self.velocity *= rhs;
        self.aabb.maxs *= rhs;
        self.aabb.mins *= rhs;
        self
    }
}

impl Add for PlayerPhysicsState {
    type Output = PlayerPhysicsState;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.position += rhs.position;
        self.acceleration += rhs.acceleration;
        self.velocity += rhs.velocity;
        self.aabb.maxs += rhs.aabb.maxs;
        self.aabb.mins += rhs.aabb.mins;
        self
    }
}

impl PlayerPhysicsState {
    pub fn apply_keyboard_mouvement(&mut self, rotation: &Vec3, input_cache: &InputCache) {
        // Jump
        if input_cache.is_key_pressed(Key::Space) {
            if self.is_on_ground {
                self.velocity.y = *JUMP_IMPULSE;
            }
        }
        // Walk
        let mut directional_acceleration = vec3(0.0, 0.0, 0.0);

        if input_cache.is_key_pressed(Key::W) {
            directional_acceleration += -rotation.forward().cross(&Vector3::y()).cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(Key::S) {
            directional_acceleration += rotation.forward().cross(&Vector3::y()).cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(Key::A) {
            directional_acceleration += -rotation.forward().cross(&Vector3::y())
        }
        if input_cache.is_key_pressed(Key::D) {
            directional_acceleration += rotation.forward().cross(&Vector3::y())
        }

        if directional_acceleration.norm_squared() != 0.0 {
            let directional_acceleration = directional_acceleration.normalize().scale(HORIZONTAL_ACCELERATION);
            self.acceleration += directional_acceleration;
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

    pub fn apply_friction(&mut self, dt: f32) {
        let friction = if self.is_on_ground {
            ON_GROUND_FRICTION
        } else {
            IN_AIR_FRICTION
        };

        if self.acceleration.x.is_zero() || self.acceleration.x.signum() != self.velocity.x.signum() {
            self.velocity.x -= friction * self.velocity.x * dt;
        }
        if self.acceleration.z.is_zero() || self.acceleration.z.signum() != self.velocity.z.signum() {
            self.velocity.z -= friction * self.velocity.z * dt;
        }
    }

    pub fn limit_velocity(&mut self) {
        // Limit the walking speed (horizontally)
        let mut horizontal_vel = vec2(self.velocity.x, self.velocity.z);
        let speed = horizontal_vel.magnitude();
        if speed > WALKING_SPEED {
            horizontal_vel = horizontal_vel.scale(WALKING_SPEED / speed);
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


