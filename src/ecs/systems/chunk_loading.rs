use std::collections::{HashSet, VecDeque};

use noise::{NoiseFn, Point2, SuperSimplex};
use specs::{Join, Read, ReadStorage, System, Write};

use crate::chunk::{BlockID, BlockIterator, ChunkColumn};
use crate::chunk_manager::ChunkManager;
use crate::physics::Interpolator;
use crate::player::PlayerPhysicsState;
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

            if !visited.contains(&(x + 1, z)) {
                queue.push_back((x + 1, z, dist - 1));
            }
            if !visited.contains(&(x - 1, z)) {
                queue.push_back((x - 1, z, dist - 1));
            }
            if !visited.contains(&(x, z + 1)) {
                queue.push_back((x, z + 1, dist - 1));
            }
            if !visited.contains(&(x, z - 1)) {
                queue.push_back((x, z - 1, dist - 1));
            }
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

            if !visited.contains(&(x + 1, y, z)) {
                queue.push_back((x + 1, y, z, dist - 1));
            }
            if !visited.contains(&(x - 1, y, z)) {
                queue.push_back((x - 1, y, z, dist - 1));
            }
            if !visited.contains(&(x, y, z + 1)) {
                queue.push_back((x, y, z + 1, dist - 1));
            }
            if !visited.contains(&(x, y, z - 1)) {
                queue.push_back((x, y, z - 1, dist - 1));
            }
            if y != 15 && !visited.contains(&(x, y + 1, z)) {
                queue.push_back((x, y + 1, z, dist - 1));
            }
            if y != 0 && !visited.contains(&(x, y - 1, z)) {
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
                let now = Instant::now();
                self.chunk_at_player = c_xyz;
                let visited = Self::flood_fill_2d(c_x, c_z, RENDER_DISTANCE + 2);
                println!("floodfill columns\t{:#?}", Instant::now().duration_since(now));

                let old_columns = self.loaded_columns.difference(&visited);
                for chunk in old_columns {
                    chunk_manager.remove_chunk(chunk);
                }

                let new_columns = visited.difference(&self.loaded_columns);
                for &(x, z) in new_columns {
                    let mut column = Box::new(ChunkColumn::new());

                    for b_x in 0..16 {
                        for b_z in 0..16 {

                            // let now = Instant::now();
                            let x = 16 * x;
                            let z = 16 * z;

                            // Scale the input for the noise function
                            let (xf, zf) = ((x + b_x as i32) as f64 / 64.0, (z + b_z as i32) as f64 / 64.0);
                            let y = self.ss.get(Point2::from([xf, zf]));
                            let y = (16.0 * (y + 10.0)) as u32;

                            // Ground layers
                            column.set_block(BlockID::GrassBlock, b_x, y, b_z);
                            column.set_block(BlockID::Dirt, b_x, y - 1, b_z);
                            column.set_block(BlockID::Dirt, b_x, y - 2, b_z);
                            column.set_block(BlockID::Dirt, b_x, y - 3, b_z);
                            column.set_block(BlockID::Dirt, b_x, y - 4, b_z);
                            column.set_block(BlockID::Dirt, b_x, y - 5, b_z);

                            // println!("what {:#?}", Instant::now().duration_since(now));
                            for y in 1..y - 5 {
                                column.set_block(BlockID::Stone, b_x, y, b_z);
                            }
                            column.set_block(BlockID::Bedrock, b_x, 0, b_z);

                            // Trees
                            // if random::<u32>() % 100 < 1 {
                            //     let h = 5;
                            //     for i in y + 1..y + 1 + h {
                            //         chunk_manager.set_block(BlockID::OakLog, b_x, i, b_z);
                            //     }
                            //
                            //     for yy in y + h - 2..=y + h - 1 {
                            //         for xx in b_x - 2..=b_x + 2 {
                            //             for zz in b_z - 2..=b_z + 2 {
                            //                 if xx != b_x || zz != b_z {
                            //                     chunk_manager.set_block(BlockID::OakLeaves, xx, yy, zz);
                            //                 }
                            //             }
                            //         }
                            //     }
                            //
                            //     for xx in b_x - 1..=b_x + 1 {
                            //         for zz in b_z - 1..=b_z + 1 {
                            //             if xx != b_x || zz != b_z {
                            //                 chunk_manager.set_block(BlockID::OakLeaves, xx, y + h, zz);
                            //             }
                            //         }
                            //     }
                            //
                            //     chunk_manager.set_block(BlockID::OakLeaves, b_x, y + h + 1, b_z);
                            //     chunk_manager.set_block(BlockID::OakLeaves, b_x + 1, y + h + 1, b_z);
                            //     chunk_manager.set_block(BlockID::OakLeaves, b_x - 1, y + h + 1, b_z);
                            //     chunk_manager.set_block(BlockID::OakLeaves, b_x, y + h + 1, b_z + 1);
                            //     chunk_manager.set_block(BlockID::OakLeaves, b_x, y + h + 1, b_z - 1);
                            // }
                        };
                    }

                    chunk_manager.add_chunk_column((x, z), column);
                }

                self.loaded_columns = visited;

                let now = Instant::now();
                let visited = Self::flood_fill_3d(c_x, c_y, c_z, RENDER_DISTANCE);
                println!("floodfill chunks\t{:#?}", Instant::now().duration_since(now));

                let new_chunks = visited.difference(&self.loaded_chunks);
                self.chunks_to_load.extend(new_chunks);
                self.loaded_chunks = visited;

                println!("terrain gen\t{:#?}", Instant::now().duration_since(now));
            }
        }


        if let Some((c_x, c_y, c_z)) = self.chunks_to_load.pop_front() {
            let now = Instant::now();
            for (b_x, b_y, b_z) in BlockIterator::new() {
                chunk_manager.update_block(c_x, c_y, c_z, b_x, b_y, b_z);
            }
            println!("AO & face occlusion {:?}\t{:#?}", (c_x, c_y, c_z), Instant::now().duration_since(now));

            // let now = Instant::now();
            chunk_manager.upload_chunk_to_gpu(c_x, c_y, c_z, &texture_pack);
            // println!("Upload {:#?}", Instant::now().duration_since(now));

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