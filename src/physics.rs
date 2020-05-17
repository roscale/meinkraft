use std::time;

use nalgebra_glm::vec3;

use crate::PLAYER_HALF_WIDTH;
use crate::chunk_manager::ChunkManager;
use crate::constants::GRAVITY;
use crate::input::InputCache;
use crate::player::{PlayerPhysicsState, PlayerProperties};

/*
    Fixed timestep physics simulation using the following method
    https://gafferongames.com/post/fix_your_timestep/
 */

pub struct PhysicsManager {
    pub t: f32,
    pub dt: f32,
    pub current_time: time::Instant,
    pub accumulator: f32,
    pub previous_state: PlayerPhysicsState,
    pub current_state: PlayerPhysicsState,
}

impl PhysicsManager {
    pub fn new(dt: f32, initial_state: PlayerPhysicsState) -> PhysicsManager {
        PhysicsManager {
            t: 0.0,
            dt,
            current_time: time::Instant::now(),
            accumulator: 0.0,
            previous_state: initial_state.clone(),
            current_state: initial_state,
        }
    }

    pub fn get_current_state(&mut self) -> &mut PlayerPhysicsState {
        &mut self.current_state
    }

    pub fn step(&mut self, integrate: &dyn Fn(PlayerPhysicsState, f32, f32) -> PlayerPhysicsState) -> PlayerPhysicsState {
        let now = time::Instant::now();
        let mut frame_time = now.duration_since(self.current_time).as_secs_f32();
        if frame_time > 0.25 {
            frame_time = 0.25;
        }
        self.current_time = now;
        self.accumulator += frame_time;

        while self.accumulator >= self.dt {
            self.previous_state = self.current_state.clone();
            self.current_state = integrate(self.previous_state.clone(), self.t, self.dt);
            self.t += self.dt;
            self.accumulator -= self.dt;
        }

        let alpha = self.accumulator / self.dt;
        let state = self.current_state.clone() * alpha + self.previous_state.clone() * (1.0 - alpha);
        state
    }

    pub fn update_player_physics(&mut self, input_cache: &InputCache, chunk_manager: &ChunkManager, player_properties: &PlayerProperties) -> PlayerPhysicsState {
        self.step(&|mut player: PlayerPhysicsState, _t: f32, dt: f32| {
            player.acceleration.y += GRAVITY;
            player.apply_keyboard_mouvement(&player_properties.rotation, &input_cache);
            player.velocity += player.acceleration * dt;
            player.apply_friction(dt);
            player.limit_velocity();

            // Decompose the velocity vector into 3 vectors and do the collision detection and resolution for each of them
            let mut is_player_on_ground = false;
            let separated_axis = &[
                vec3(player.velocity.x, 0.0, 0.0),
                vec3(0.0, player.velocity.y, 0.0),
                vec3(0.0, 0.0, player.velocity.z)];

            for v in separated_axis {
                player.aabb.ip_translate(&(v * dt));
                let colliding_block = player.get_colliding_block_coords(&chunk_manager);

                // Reaction
                if let Some(colliding_block) = colliding_block {
                    is_player_on_ground |= player.separate_from_block(&v, &colliding_block);
                }
            }
            player.is_on_ground = is_player_on_ground;

            // Update the position of the player and reset the acceleration
            player.position.x = player.aabb.mins.x + PLAYER_HALF_WIDTH;
            player.position.y = player.aabb.mins.y;
            player.position.z = player.aabb.mins.z + PLAYER_HALF_WIDTH;

            player.acceleration.x = 0.0;
            player.acceleration.y = 0.0;
            player.acceleration.z = 0.0;
            player
        })
    }
}
