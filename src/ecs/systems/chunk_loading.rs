use std::collections::{HashSet, VecDeque};

use noise::{NoiseFn, Point2, SuperSimplex};
use rand::random;
use specs::{Join, Read, ReadStorage, System, Write};

use crate::chunk::{BlockID, BlockIterator, Chunk};
use crate::chunk_manager::ChunkManager;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};
use crate::types::TexturePack;
use std::time::Instant;

pub struct ChunkLoading {
    ss: SuperSimplex,
    loaded_columns: HashSet<(i32, i32)>,
    loaded_chunks: HashSet<(i32, i32, i32)>,
    chunks_to_load: VecDeque<(i32, i32, i32)>,
    chunk_at_player: (i32, i32, i32),
}

impl ChunkLoading {
    pub fn new() -> Self {
        Self {
            ss: SuperSimplex::new(),
            loaded_columns: HashSet::new(),
            loaded_chunks: HashSet::new(),
            chunks_to_load: VecDeque::new(),
            chunk_at_player: (-1, -1, -1),
        }
    }

    fn flood_fill_2d(x: i32, z: i32, distance: i32) -> HashSet<(i32, i32)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((x, z, distance));

        while !queue.is_empty() {
            let (x, z, dist) = queue.pop_front().unwrap();
            visited.insert((x, z));
            if dist <= 0 {
                continue;
            }

            queue.push_back((x + 1, z, dist - 1));
            queue.push_back((x - 1, z, dist - 1));
            queue.push_back((x, z + 1, dist - 1));
            queue.push_back((x, z - 1, dist - 1));
        }
        visited
    }

    fn flood_fill_3d(x: i32, y: i32, z: i32, distance: i32) -> HashSet<(i32, i32, i32)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((x, y, z, distance));

        while !queue.is_empty() {
            let (x, y, z, dist) = queue.pop_front().unwrap();
            // dbg!(x, y, z, dist);
            if y < 0 || y > 15 {
                continue;
            }
            visited.insert((x, y, z));
            if dist <= 0 {
                continue;
            }

            queue.push_back((x + 1, y, z, dist - 1));
            queue.push_back((x - 1, y, z, dist - 1));
            queue.push_back((x, y, z + 1, dist - 1));
            queue.push_back((x, y, z - 1, dist - 1));
            if y != 15 {
                queue.push_back((x, y + 1, z, dist - 1));
            }
            if y != 0 {
                queue.push_back((x, y - 1, z, dist - 1));
            }
        }
        visited
    }
}

const RENDER_DISTANCE: i32 = 5;

impl<'a> System<'a> for ChunkLoading {
    type SystemData = (
        ReadStorage<'a, Interpolator<PlayerPhysicsState>>,
        Write<'a, ChunkManager>,
        Read<'a, TexturePack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_physics_state,
            mut chunk_manager,
            texture_pack,
        ) = data;

        for player_physics_state in (&player_physics_state).join() {
            let state = player_physics_state.get_latest_state();
            let c_x = state.position.x as i32 / 16;
            let c_y = state.position.y as i32 / 16;
            let c_z = state.position.z as i32 / 16;
            let c_xyz = (c_x, c_y, c_z);

            if c_xyz != self.chunk_at_player {
                self.chunk_at_player = c_xyz;

                let visited = Self::flood_fill_2d(c_x, c_z, RENDER_DISTANCE + 2);

                let new_columns = visited.difference(&self.loaded_columns);
                for &(x, z) in new_columns {
                    for y in 0..16 {
                        chunk_manager.add_chunk((x, y, z), Chunk::empty());
                    }

                    let now = Instant::now();

                    for x in (16 * x)..(16 * (x + 1)) {
                        for z in (16 * z)..(16 * (z + 1)) {
                            // Scale the input for the noise function
                            let (xf, zf) = (x as f64 / 64.0, z as f64 / 64.0);
                            let y = self.ss.get(Point2::from([xf, zf]));
                            let y = (16.0 * (y + 10.0)) as i32;

                            // Ground layers
                            chunk_manager.set_block(BlockID::GrassBlock, x, y, z);
                            chunk_manager.set_block(BlockID::Dirt, x, y - 1, z);
                            chunk_manager.set_block(BlockID::Dirt, x, y - 2, z);
                            // for y in 0..=y - 3 {
                            //     chunk_manager.set_block(BlockID::Cobblestone, x, y, z);
                            // }

                            // Trees
                            if random::<u32>() % 100 < 1 {
                                let h = 5;
                                for i in y + 1..y + 1 + h {
                                    chunk_manager.set_block(BlockID::OakLog, x, i, z);
                                }

                                for yy in y + h - 2..=y + h - 1 {
                                    for xx in x - 2..=x + 2 {
                                        for zz in z - 2..=z + 2 {
                                            if xx != x || zz != z {
                                                chunk_manager.set_block(BlockID::OakLeaves, xx, yy, zz);
                                            }
                                        }
                                    }
                                }

                                for xx in x - 1..=x + 1 {
                                    for zz in z - 1..=z + 1 {
                                        if xx != x || zz != z {
                                            chunk_manager.set_block(BlockID::OakLeaves, xx, y + h, zz);
                                        }
                                    }
                                }

                                chunk_manager.set_block(BlockID::OakLeaves, x, y + h + 1, z);
                                chunk_manager.set_block(BlockID::OakLeaves, x + 1, y + h + 1, z);
                                chunk_manager.set_block(BlockID::OakLeaves, x - 1, y + h + 1, z);
                                chunk_manager.set_block(BlockID::OakLeaves, x, y + h + 1, z + 1);
                                chunk_manager.set_block(BlockID::OakLeaves, x, y + h + 1, z - 1);
                            }
                        }
                    }

                    println!("{:#?}", Instant::now().duration_since(now));
                }
                self.loaded_columns.extend(visited);

                let visited = Self::flood_fill_3d(c_x, c_y, c_z, RENDER_DISTANCE);
                let new_chunks = visited.difference(&self.loaded_chunks);
                self.chunks_to_load.extend(new_chunks);
                self.loaded_chunks.extend(visited);
            }
        }

        if let Some((c_x, c_y, c_z)) = self.chunks_to_load.pop_front() {
            let now = Instant::now();

            for (b_x, b_y, b_z) in BlockIterator::new() {
                chunk_manager.update_block(c_x, c_y, c_z, b_x, b_y, b_z);
            }

            chunk_manager.upload_chunk_to_gpu(c_x, c_y, c_z, &texture_pack);
            println!("Load {:#?}", Instant::now().duration_since(now));

        }

        //     // dbg!(xyz);
        //
        //     for x in 16 * xyz.0..16 * (xyz.0 + 1) {
        //         for z in 16 * xyz.2..16 * (xyz.2 + 1) {
        //             // Scale the input for the noise function
        //             let (xf, zf) = (x as f64 / 64.0, z as f64 / 64.0);
        //             let y = self.ss.get(Point2::from([xf, zf]));
        //             let y = (16.0 * (y + 10.0)) as i32;
        //
        //             // Ground layers
        //             chunk_manager.set_block(BlockID::GrassBlock, x, y, z);
        //             chunk_manager.set_block(BlockID::Dirt, x, y - 1, z);
        //             chunk_manager.set_block(BlockID::Dirt, x, y - 2, z);
        //             for y in 0..=y - 3 {
        //                 chunk_manager.set_block(BlockID::Cobblestone, x, y, z);
        //             }
        //
        //             // Trees
        //             // if random::<u32>() % 100 < 1 {
        //             //     let h = 5;
        //             //     for i in y + 1..y + 1 + h {
        //             //         chunk_manager.put_block(BlockID::OakLog, x, i, z);
        //             //     }
        //             //
        //             //     for yy in y + h - 2..=y + h - 1 {
        //             //         for xx in x - 2..=x + 2 {
        //             //             for zz in z - 2..=z + 2 {
        //             //                 if xx != x || zz != z {
        //             //                     chunk_manager.put_block(BlockID::OakLeaves, xx, yy, zz);
        //             //                 }
        //             //             }
        //             //         }
        //             //     }
        //             //
        //             //     for xx in x - 1..=x + 1 {
        //             //         for zz in z - 1..=z + 1 {
        //             //             if xx != x || zz != z {
        //             //                 chunk_manager.put_block(BlockID::OakLeaves, xx, y + h, zz);
        //             //             }
        //             //         }
        //             //     }
        //             //
        //             //     chunk_manager.put_block(BlockID::OakLeaves, x, y + h + 1, z);
        //             //     chunk_manager.put_block(BlockID::OakLeaves, x + 1, y + h + 1, z);
        //             //     chunk_manager.put_block(BlockID::OakLeaves, x - 1, y + h + 1, z);
        //             //     chunk_manager.put_block(BlockID::OakLeaves, x, y + h + 1, z + 1);
        //             //     chunk_manager.put_block(BlockID::OakLeaves, x, y + h + 1, z - 1);
        //             // }
        //         }
        //     }
        //
        //     // dbg!(xyz);
        //     for (b_x, b_y, b_z) in BlockIterator::new() {
        //         chunk_manager.update_block(xyz.0, xyz.1, xyz.2, b_x, b_y, b_z);
        //     }
        //     chunk_manager.upload_chunk_to_gpu(xyz.0, xyz.1, xyz.2, &texture_pack);
        // }
    }
}