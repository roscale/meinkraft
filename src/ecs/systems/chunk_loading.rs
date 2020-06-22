use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Instant;

use bit_vec::BitVec;
use itertools::Itertools;
use noise::{NoiseFn, Point2, SuperSimplex};
use num_traits::abs;
use parking_lot::RwLock;
use rayon::prelude::*;
use specs::{Join, Read, ReadStorage, System, Write};

use crate::chunk::{BlockID, BlockIterator, ChunkColumn};
use crate::chunk_manager::ChunkManager;
use crate::constants::{RENDER_DISTANCE, WORLD_GENERATION_THREAD_POOL_SIZE};
use crate::physics::Interpolator;
use crate::player::PlayerPhysicsState;
use crate::types::TexturePack;

pub struct ChunkLoading {
    noise_fn: SuperSimplex,
    chunk_column_pool: Vec<Arc<ChunkColumn>>,
    loaded_columns: Vec<(i32, i32)>,
    loaded_chunks: Vec<(i32, i32, i32)>,
    chunks_to_load: VecDeque<(i32, i32, i32)>,
    chunk_at_player: (i32, i32, i32),

    send_chunk_column: Sender<(i32, i32, Arc<ChunkColumn>)>,
    receive_chunk_column: Receiver<(i32, i32, Arc<ChunkColumn>)>,

    send_chunks: Sender<(i32, i32, i32)>,
    receive_chunks: Receiver<(i32, i32, i32)>,
    pool: rayon::ThreadPool,

}

impl ChunkLoading {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (tx2, rx2) = channel();

        Self {
            noise_fn: SuperSimplex::new(),
            chunk_column_pool: Vec::new(),
            loaded_columns: Vec::new(),
            loaded_chunks: Vec::new(),
            chunks_to_load: VecDeque::new(),
            chunk_at_player: (-100, -100, -100),
            send_chunk_column: tx,
            receive_chunk_column: rx,
            send_chunks: tx2,
            receive_chunks: rx2,
            pool: rayon::ThreadPoolBuilder::new().num_threads(*WORLD_GENERATION_THREAD_POOL_SIZE).build().unwrap(),
        }
    }

    fn allocate_chunk_column(&mut self) -> Arc<ChunkColumn> {
        match self.chunk_column_pool.pop() {
            Some(mut column) => {
                for chunk in column.chunks.iter() {
                    chunk.reset();
                }
                column
            },
            None => {
                Arc::new(ChunkColumn::new())
            }
        }
    }

    fn flood_fill_2d(x: i32, z: i32, distance: i32) -> Vec<(i32, i32)> {
        assert!(distance >= 0);

        let matrix_width = 2 * distance + 1;
        let mut is_visited = BitVec::from_elem(
            (matrix_width * matrix_width) as usize, false);

        let center = (x, z);
        let coords_to_index = move |x: i32, z: i32| {
            (matrix_width * (x - center.0 + distance)
                + (z - center.1 + distance)) as usize
        };

        let mut visited_chunks = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((x, z, distance));
        is_visited.set(coords_to_index(x, z), true);

        while !queue.is_empty() {
            let (x, z, dist) = queue.pop_front().unwrap();
            visited_chunks.push((x, z));
            if dist <= 0 {
                continue;
            }

            if !is_visited[coords_to_index(x + 1, z)] {
                queue.push_back((x + 1, z, dist - 1));
                is_visited.set(coords_to_index(x + 1, z), true);
            }
            if !is_visited[coords_to_index(x - 1, z)] {
                queue.push_back((x - 1, z, dist - 1));
                is_visited.set(coords_to_index(x - 1, z), true);
            }
            if !is_visited[coords_to_index(x, z + 1)] {
                queue.push_back((x, z + 1, dist - 1));
                is_visited.set(coords_to_index(x, z + 1), true);
            }
            if !is_visited[coords_to_index(x, z - 1)] {
                queue.push_back((x, z - 1, dist - 1));
                is_visited.set(coords_to_index(x, z - 1), true);
            }
        }
        visited_chunks
    }

    fn flood_fill_3d(x: i32, y: i32, z: i32, distance: i32) -> Vec<(i32, i32, i32)> {
        assert!(distance >= 0);

        let matrix_width = 2 * distance + 1;
        let mut is_visited = BitVec::from_elem(
            (matrix_width * matrix_width * matrix_width) as usize, false);

        let center = (x, y, z);
        let coords_to_index = move |x: i32, y: i32, z: i32| {
            (matrix_width * matrix_width * (x - center.0 + distance)
                + matrix_width * (y - center.1 + distance)
                + (z - center.2 + distance)) as usize
        };

        let mut visited_chunks = Vec::new();
        let mut queue = VecDeque::new();
        queue.reserve(100);
        queue.push_back((x, y, z, distance));
        is_visited.set(coords_to_index(x, y, z), true);

        while !queue.is_empty() {
            let (x, y, z, dist) = queue.pop_front().unwrap();

            if y >= 0 && y < 16 {
                visited_chunks.push((x, y, z));
            }
            if dist <= 0 {
                continue;
            }

            if !is_visited[coords_to_index(x + 1, y, z)] {
                queue.push_back((x + 1, y, z, dist - 1));
                is_visited.set(coords_to_index(x + 1, y, z), true);
            }
            if !is_visited[coords_to_index(x - 1, y, z)] {
                queue.push_back((x - 1, y, z, dist - 1));
                is_visited.set(coords_to_index(x - 1, y, z), true);
            }
            if !is_visited[coords_to_index(x, y, z + 1)] {
                queue.push_back((x, y, z + 1, dist - 1));
                is_visited.set(coords_to_index(x, y, z + 1), true);
            }
            if !is_visited[coords_to_index(x, y, z - 1)] {
                queue.push_back((x, y, z - 1, dist - 1));
                is_visited.set(coords_to_index(x, y, z - 1), true);
            }
            if !is_visited[coords_to_index(x, y + 1, z)] {
                queue.push_back((x, y + 1, z, dist - 1));
                is_visited.set(coords_to_index(x, y + 1, z), true);
            }
            if !is_visited[coords_to_index(x, y - 1, z)] {
                queue.push_back((x, y - 1, z, dist - 1));
                is_visited.set(coords_to_index(x, y - 1, z), true);
            }
        }
        visited_chunks
    }
}

impl<'a> System<'a> for ChunkLoading {
    type SystemData = (
        ReadStorage<'a, Interpolator<PlayerPhysicsState>>,
        Write<'a, Arc<ChunkManager>>,
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
            let (c_x, c_y, c_z, _, _, _) = ChunkManager::get_chunk_coords(
                state.position.x as i32,
                state.position.y as i32,
                state.position.z as i32,
            );
            let c_xyz = (c_x, c_y, c_z);

            // Execute this system every time a player travels to another chunk
            if c_xyz != self.chunk_at_player {
                let previous_chunk_at_player = self.chunk_at_player;
                self.chunk_at_player = c_xyz;

                // Flood fill for columns and chunks
                let now = Instant::now();
                let visited_columns = Self::flood_fill_2d(c_x, c_z, RENDER_DISTANCE + 2);
                println!("floodfill columns\t{:#?}", Instant::now().duration_since(now));

                let now = Instant::now();
                let visited_chunks = Self::flood_fill_3d(c_x, c_y, c_z, RENDER_DISTANCE);
                println!("floodfill chunks\t{:#?}", Instant::now().duration_since(now));

                // Unload old chunks
                let old_chunks = self.loaded_chunks.iter().filter(|(x, y, z)| {
                    abs(x - self.chunk_at_player.0) +
                        abs(y - self.chunk_at_player.1) +
                        abs(z - self.chunk_at_player.2) > RENDER_DISTANCE
                });

                for &(x, y, z) in old_chunks {
                    if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
                        chunk.unload_from_gpu();
                        *chunk.is_rendered.write() = false;
                    }
                }

                // Remove old chunk columns
                let old_columns = self.loaded_columns.iter().filter(|(x, z)| {
                    abs(x - self.chunk_at_player.0) +
                        abs(z - self.chunk_at_player.2) > RENDER_DISTANCE + 2
                }).collect_vec();

                let now = Instant::now();
                for column in old_columns {
                    if let Some(column) = chunk_manager.remove_chunk_column(column) {
                        self.chunk_column_pool.push(column);
                    }
                }
                println!("Removing old columns\t{:#?}", Instant::now().duration_since(now));


                // Generate terrain
                let now = Instant::now();

                let new_chunks = visited_columns
                    .iter()
                    .filter(|(x, z)| {
                        abs(x - previous_chunk_at_player.0)
                            + abs(z - previous_chunk_at_player.2) > RENDER_DISTANCE + 2
                    });

                let mut vec = Vec::new();
                for &(x, z) in new_chunks {
                    vec.push((x, z, self.allocate_chunk_column()));
                }

                println!("terrain gen\t{:#?}", Instant::now().duration_since(now));

                let now = Instant::now();

                let noise_fn = self.noise_fn.clone();
                let tx = self.send_chunk_column.clone();

                self.pool.spawn_fifo(move || {
                    for (x, z, mut column) in vec {
                        // let x = *x;
                        // let z = *z;
                        // eprintln!("test");
                        // let mut column = Box::new(ChunkColumn::new());
                        //
                        for b_x in 0..16 {
                            for b_z in 0..16 {
                                let x = 16 * x;
                                let z = 16 * z;

                                // Scale the input for the noise function
                                let (xf, zf) = ((x + b_x as i32) as f64 / 64.0, (z + b_z as i32) as f64 / 64.0);
                                let y = 200.0;
                                let y = noise_fn.get(Point2::from([xf, zf]));
                                let y = (16.0 * (y + 10.0)) as u32;
                                // let y = 195;

                                // Ground layers
                                column.set_block(BlockID::GrassBlock, b_x, y, b_z);
                                column.set_block(BlockID::Dirt, b_x, y - 1, b_z);
                                column.set_block(BlockID::Dirt, b_x, y - 2, b_z);
                                column.set_block(BlockID::Dirt, b_x, y - 3, b_z);
                                column.set_block(BlockID::Dirt, b_x, y - 4, b_z);
                                column.set_block(BlockID::Dirt, b_x, y - 5, b_z);

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
                        tx.send((x, z, column));
                        // // chunk_manager.add_chunk_column((x, z), column);
                        // // (x, z, column)
                    }
                });

                // handle.join();

                // Insert new chunk columns
                // vec
                //     .par_iter_mut()
                //     .for_each(|(x, z, column)| {
                //     });

                let now = Instant::now();
                for (x, z, column) in self.receive_chunk_column.try_iter() {
                    chunk_manager.add_chunk_column((x, z), column);
                }
                println!("Adding column {:?}", Instant::now().duration_since(now));
                // for (x, z, column) in vec {
                //     chunk_manager.add_chunk_column((x, z), column);
                // }

                // println!("what \t{:#?}", Instant::now().duration_since(now));
                // .for_each(|(x, z, column)| {
                    //     // dbg!((x, z));
                    //     // chunk_manager.add_chunk_column((x, z), column);
                    // });





                // Add new chunks to the loading queue
                let new_chunks = visited_chunks.iter().filter(|c| {
                    abs(c.0 - previous_chunk_at_player.0)
                    + abs(c.1 - previous_chunk_at_player.1)
                    + abs(c.2 - previous_chunk_at_player.2) > RENDER_DISTANCE
                });
                self.chunks_to_load.extend(new_chunks);

                self.loaded_columns = visited_columns;
                self.loaded_chunks = visited_chunks;
            }
        }


        for &(c_x, c_y, c_z) in self.chunks_to_load.iter() {
            let chunk_manager = Arc::clone(&chunk_manager);
            let tx = self.send_chunks.clone();
            self.pool.spawn_fifo(move || {
                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                    if *chunk.number_of_blocks.read() == 0 {
                        return;
                    }
                    chunk_manager.update_all_blocks(c_x, c_y, c_z);
                    tx.send((c_x, c_y, c_z));
                }
            });
        }
        self.chunks_to_load.clear();

        // if let Some((c_x, c_y, c_z)) = self.chunks_to_load.pop_front() {
        //     let chunk_manager = Arc::clone(&chunk_manager);
        //     let tx = self.send_chunks.clone();
        //     thread::spawn(move || {
        //         let now = Instant::now();
        //         if let Some(chunk) = chunk_manager.read().get_chunk(c_x, c_y, c_z) {
        //             if chunk.number_of_blocks == 0 {
        //                 return;
        //             }
        //         }
        //
        //         if chunk_manager.read().get_chunk(c_x, c_y, c_z).is_some() {
        //             // for (b_x, b_y, b_z) in BlockIterator::new() {
        //             //     chunk_manager.update_block(c_x, c_y, c_z, b_x, b_y, b_z);
        //             // }
        //             chunk_manager.write().update_all_blocks(c_x, c_y, c_z);
        //             tx.send((c_x, c_y, c_z));
        //             // println!("AO & face occlusion {:?}\t{:#?}", (c_x, c_y, c_z), Instant::now().duration_since(now));
        //         }
        //     });
        // }


        if let Ok((c_x, c_y, c_z)) = self.receive_chunks.try_recv() {
            let now = Instant::now();

            if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                chunk.upload_to_gpu(&texture_pack);
                *chunk.is_rendered.write() = true;
            }
            // println!("Uploading to GPU\t{:#?}", Instant::now().duration_since(now));

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