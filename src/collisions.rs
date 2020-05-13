use crate::{Player, PLAYER_HALF_WIDTH, PLAYER_WIDTH, PLAYER_HEIGHT};
use crate::chunk_manager::ChunkManager;
use num_traits::real::Real;
use ncollide3d::math::Point;
use nalgebra_glm::{vec3, Vec3};
use crate::chunk::BlockID;
use num_traits::Zero;
use std::process::exit;
use crate::aabb::AABB;

pub fn get_block_aabb(mins: &Vec3) -> AABB {
    AABB::new(
        mins.clone(),
        mins + vec3(1.0, 1.0, 1.0))
    // vec3(mins.x + 1.0, mins.y + 1.0, mins.z + 1.0))
}

pub fn player_collision_detection(player: &mut Player, chunk_manager: &ChunkManager) {
    player.velocity += player.acceleration;
    let mag = player.velocity.magnitude();
    if player.velocity.magnitude() > 0.1 {
        player.velocity = player.velocity.unscale(mag).scale(0.1)
    }

    let separated_axis = &[
        vec3(player.velocity.x, 0.0, 0.0),
        vec3(0.0, player.velocity.y, 0.0),
        vec3(0.0, 0.0, player.velocity.z)];

    for v in separated_axis {
        player.aabb.ip_translate(v);

        let player_mins = &player.aabb.mins;
        let player_maxs = &player.aabb.maxs;

        let block_min = vec3(
            player_mins.x.floor() as i32, player_mins.y.floor() as i32, player_mins.z.floor() as i32
        );
        let block_max = vec3(
            player_maxs.x.floor() as i32, player_maxs.y.floor() as i32, player_maxs.z.floor() as i32
        );

        let mut colliding_block = None;
        for y in block_min.y..=block_max.y {
            for z in block_min.z..=block_max.z {
                for x in block_min.x..=block_max.x {
                    if let Some(block) = chunk_manager.get_block(x, y, z) {
                        if !block.is_air() {
                            let block_aabb = get_block_aabb(&vec3(x as f32, y as f32, z as f32));
                            if player.aabb.intersects(&block_aabb) {
                                colliding_block = Some(vec3(x as f32, y as f32, z as f32));
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Reaction
        if let Some(colliding_block) = colliding_block {
            let block_aabb = get_block_aabb(&colliding_block);

            if !v.x.is_zero() {
                if v.x < 0.0{
                    player.aabb = AABB::new(
                        vec3(block_aabb.maxs.x, player.aabb.mins.y, player.aabb.mins.z),
                        vec3(block_aabb.maxs.x + PLAYER_WIDTH, player.aabb.maxs.y, player.aabb.maxs.z));
                } else {
                    player.aabb = AABB::new(
                        vec3(block_aabb.mins.x - PLAYER_WIDTH, player.aabb.mins.y, player.aabb.mins.z),
                        vec3(block_aabb.mins.x, player.aabb.maxs.y, player.aabb.maxs.z));
                }
                player.velocity.x = 0.0
            }

            if !v.y.is_zero() {
                if v.y < 0.0 {
                    player.aabb = AABB::new(
                        vec3(player.aabb.mins.x, block_aabb.maxs.y, player.aabb.mins.z),
                        vec3(player.aabb.maxs.x, block_aabb.maxs.y + PLAYER_HEIGHT, player.aabb.maxs.z));
                } else {
                    player.aabb = AABB::new(
                        vec3(player.aabb.mins.x, block_aabb.mins.y - PLAYER_HEIGHT, player.aabb.mins.z),
                        vec3(player.aabb.maxs.x, block_aabb.mins.y, player.aabb.maxs.z));
                }
                player.velocity.y = 0.0;
            }

            if !v.z.is_zero() {
                if v.z < 0.0 {
                    player.aabb = AABB::new(
                        vec3(player.aabb.mins.x, player.aabb.mins.y, block_aabb.maxs.z),
                        vec3(player.aabb.maxs.x, player.aabb.maxs.y, block_aabb.maxs.z + PLAYER_WIDTH));
                } else {
                    player.aabb = AABB::new(
                        vec3(player.aabb.mins.x, player.aabb.mins.y, block_aabb.mins.z - PLAYER_WIDTH),
                        vec3(player.aabb.maxs.x, player.aabb.maxs.y, block_aabb.mins.z));
                }
                player.velocity.z = 0.0
            }
        }
    }
    let new_position = vec3(player.aabb.mins.x + PLAYER_HALF_WIDTH, player.aabb.mins.y, player.aabb.mins.z + PLAYER_HALF_WIDTH);
    if (player.position - new_position).magnitude() > 0.5 {
        println!("WOW");
    }

    player.position.x = player.aabb.mins.x + PLAYER_HALF_WIDTH;
    player.position.y = player.aabb.mins.y;
    player.position.z = player.aabb.mins.z + PLAYER_HALF_WIDTH;
}