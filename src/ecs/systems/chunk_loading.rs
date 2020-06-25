use std::collections::{VecDeque, HashSet};
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
use std::iter::FromIterator;
use std::process::exit;
use std::sync::atomic::AtomicBool;

pub struct ChunkLoading {
    noise_fn: SuperSimplex,
    chunk_column_pool: Arc<RwLock<Vec<Arc<ChunkColumn>>>>,
    loaded_columns: Vec<(i32, i32)>,
    removed_columns: Vec<(i32, i32)>,
    loaded_chunks: Arc<RwLock<Vec<(i32, i32, i32)>>>,
    chunks_to_load: VecDeque<(i32, i32, i32)>,
    chunk_at_player: (i32, i32, i32),

    send_chunk_column: Sender<(i32, i32, Arc<ChunkColumn>)>,
    receive_chunk_column: Receiver<(i32, i32, Arc<ChunkColumn>)>,

    send_chunks: Sender<(i32, i32, i32)>,
    receive_chunks: Receiver<(i32, i32, i32)>,
    expand_ring: Arc<RwLock<bool>>,
    pool: rayon::ThreadPool,
}

impl ChunkLoading {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (tx2, rx2) = channel();

        Self {
            noise_fn: SuperSimplex::new(),
            chunk_column_pool: Arc::new(RwLock::new(Vec::new())),
            loaded_columns: Vec::new(),
            removed_columns: Vec::new(),
            loaded_chunks: Arc::new(RwLock::new(Vec::new())),
            chunks_to_load: VecDeque::new(),
            chunk_at_player: (-100, -100, -100),
            send_chunk_column: tx,
            receive_chunk_column: rx,
            send_chunks: tx2,
            receive_chunks: rx2,
            expand_ring: Arc::new(RwLock::new(true)),
            pool: rayon::ThreadPoolBuilder::new()
                .stack_size(4 * 1024 * 1024)
                .num_threads(*WORLD_GENERATION_THREAD_POOL_SIZE)
                .build().unwrap(),
        }
    }

    // #[inline]
    // fn allocate_chunk_column(&mut self) -> Arc<ChunkColumn> {
    //     match self.chunk_column_pool.pop() {
    //         Some(mut column) => {
    //             for chunk in column.chunks.iter() {
    //                 chunk.reset();
    //             }
    //             column
    //         },
    //         None => {
    //             println!("ALLOC");
    //             Arc::new(ChunkColumn::new())
    //         }
    //     }
    // }

    fn flood_fill_columns(chunk_manager: &ChunkManager, x: i32, z: i32, distance: i32) -> Vec<(i32, i32)> {
        assert!(distance >= 2);

        let matrix_width = 2 * distance + 1;
        let mut is_visited = BitVec::from_elem(
            (matrix_width * matrix_width) as usize, false);

        let center = (x, z);
        let matrix_index = move |x: i32, z: i32| {
            (matrix_width * (x - center.0 + distance)
                + (z - center.1 + distance)) as usize
        };

        let is_position_valid = |c_x: i32, c_z: i32| {
            abs(x - c_x) <= distance && abs(z - c_z) <= distance
        };

        let mut queue = VecDeque::new();
        let mut ring = Vec::new();
        let mut ring_number = 0;

        queue.push_back((x, z));
        ring.push((x, z));
        is_visited.set(matrix_index(x, z), true);

        while !queue.is_empty() {
            // Expand the ring
            for (c_x, c_z) in queue.drain(..) {
                for &(c_x, c_z) in &[
                    (c_x + 1, c_z),
                    (c_x - 1, c_z),
                    (c_x, c_z + 1),
                    (c_x, c_z - 1),
                ] {
                    if is_position_valid(c_x, c_z) && !is_visited[matrix_index(c_x, c_z)] {
                        ring.push((c_x, c_z));
                        is_visited.set(matrix_index(c_x, c_z), true);
                        // visited_chunks.push((c_x, c_z));
                    }
                }
            }

            // We must expand at least 2 rings before returning something
            ring_number += 1;
            if ring_number < 2 {
                queue.extend(ring.iter());
                continue;
            }

            let mut unloaded_columns = Vec::new();
            for column in &ring {
                if !chunk_manager.loaded_chunk_columns.read().contains_key(column) {
                    unloaded_columns.push(*column);
                }
            }
            if !unloaded_columns.is_empty() {
                return unloaded_columns;
            } else {
                queue.extend(ring.iter());
                ring.clear();
            }
        }
        Vec::new()
    }

    fn flood_fill_chunks(chunk_manager: &ChunkManager, x: i32, y: i32, z: i32, distance: i32) -> Vec<(i32, i32, i32)> {
        assert!(distance >= 0);

        // chunk_manager.get_chunk(x, y, z).unwrap();

        let matrix_width = 2 * distance + 1;
        let mut is_visited = BitVec::from_elem(
            (matrix_width * matrix_width * matrix_width) as usize, false);

        let center = (x, y, z);
        let coords_to_index = move |x: i32, y: i32, z: i32| {
            (matrix_width * matrix_width * (x - center.0 + distance)
                + matrix_width * (y - center.1 + distance)
                + (z - center.2 + distance)) as usize
        };

        let is_position_valid = |c_x: i32, c_y: i32, c_z: i32| {
            abs(x - c_x) <= distance &&
                abs(y - c_y) <= distance &&
                abs(z - c_z) <= distance &&
                c_y >= 0 && c_y < 16
        };

        // let mut visited_chunks = Vec::new();
        let mut queue = VecDeque::new();
        let mut ring = Vec::new();

        // queue.reserve(1000);
        queue.push_back((x, y, z));
        ring.push((x, y, z));
        is_visited.set(coords_to_index(x, y, z), true);

        // Load the first tile
        if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
            if !*chunk.is_rendered.read() {
                println!("FIRST TILE");
                return ring;
            }
        }

        while !queue.is_empty() {
            for (x, y, z) in queue.drain(..) {
                if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
                    if chunk.is_fully_opaque() {
                        continue;
                    }
                }

                for &(x, y, z) in &[
                    (x + 1, y, z),
                    (x - 1, y, z),
                    (x, y, z + 1),
                    (x, y, z - 1),
                    (x, y + 1, z),
                    (x, y - 1, z),
                ] {
                    if is_position_valid(x, y, z) && !is_visited[coords_to_index(x, y, z)] {
                        // queue.push_back((x, y, z));
                        ring.push((x, y, z));
                        is_visited.set(coords_to_index(x, y, z), true);
                    }
                }
            }

            let mut unloaded_chunks = Vec::new();
            for &(x, y, z) in &ring {
                // dbg!((x, y, z));
                // println!("BEFORE CRASH {:?}", (x, y, z));
                if !*chunk_manager.get_chunk(x, y, z).unwrap().is_rendered.read() {
                    // println!("NOT LOADED {:?}", (x, y, z));
                    unloaded_chunks.push((x, y, z));
                }
            }
            if !unloaded_chunks.is_empty() {
                return unloaded_chunks;
            } else {
                queue.extend(ring.iter());
                ring.clear();
            }
        }
        Vec::new()
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
            // if c_xyz != self.chunk_at_player {

            // Make this a constant
            let chunk_uploads_per_frame = 2;
            for (c_x, c_y, c_z) in self.receive_chunks.try_iter().take(chunk_uploads_per_frame) {
                // println!("UPLOADING {:?}", (c_x, c_y, c_z));

                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                    // dbg!((c_x, c_y, c_z));
                    // println!("UPLOADING {:?}", (c_x, c_y, c_z));

                    // println!("uploading {:?}", (c_x, c_y, c_z));
                    let now = Instant::now();
                    chunk.upload_to_gpu(&texture_pack);
                    // println!("uploading took {:?}", Instant::now().duration_since(now));
                    *chunk.is_rendered.write() = true;
                }
            }

            if *self.expand_ring.read() {
                *self.expand_ring.write() = false;



                let previous_chunk_at_player = self.chunk_at_player;
                self.chunk_at_player = c_xyz;

                // println!("old {:?}", previous_chunk_at_player);
                // println!("new {:?}", self.chunk_at_player);

                // Unload old chunks
                // let loaded_chunks = self.loaded_chunks.read().clone();
                // let old_chunks = loaded_chunks.iter().filter(|(x, y, z)| {
                //     abs(x - self.chunk_at_player.0) > RENDER_DISTANCE ||
                //         abs(y - self.chunk_at_player.1) > RENDER_DISTANCE ||
                //         abs(z - self.chunk_at_player.2) > RENDER_DISTANCE
                // });
                //
                // for &(x, y, z) in old_chunks {
                //     if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
                //         chunk.unload_from_gpu();
                //         *chunk.is_rendered.write() = false;
                //     }
                // }
                //
                // // Remove old chunk columns
                // let old_columns = self.loaded_columns.iter().filter(|(x, z)| {
                //     abs(x - self.chunk_at_player.0) > RENDER_DISTANCE + 2 ||
                //         abs(z - self.chunk_at_player.2) > RENDER_DISTANCE + 2
                // }).cloned().collect_vec();
                //
                // let now = Instant::now();
                // for xz in old_columns {
                //     self.removed_columns.push(xz);
                // }
                // println!("Removing old columns\t{:#?}", Instant::now().duration_since(now));

                let now = Instant::now();
                let visited_columns = Self::flood_fill_columns(&chunk_manager, c_x, c_z, RENDER_DISTANCE + 2);
                // dbg!(&visited_columns);
                // println!("floodfill columns\t{:#?} {} {:?}", Instant::now().duration_since(now), visited_columns.len(), (c_x, c_z));

                let column_pool = Arc::clone(&self.chunk_column_pool);

                let new_columns = &visited_columns;
                // let new_columns = visited_columns
                //     .iter()
                //     .filter(|(x, z)| {
                //         abs(x - previous_chunk_at_player.0) > RENDER_DISTANCE + 2 ||
                //             abs(z - previous_chunk_at_player.2) > RENDER_DISTANCE + 2
                //     });

                let mut vec = Vec::new();
                let now = Instant::now();
                for &(x, z) in new_columns {
                    // vec.push((x, z, self.allocate_chunk_column()));
                    vec.push((x, z, {
                        let mut guard = column_pool.write();
                        match guard.pop() {
                            Some(mut column) => {
                                for chunk in column.chunks.iter() {
                                    chunk.reset();
                                }
                                column
                            },
                            None => {
                                // println!("ALLOC");
                                Arc::new(ChunkColumn::new())
                            }
                        }
                    }));
                }
                // println!("terrain gen\t{:#?}", Instant::now().duration_since(now));
                self.loaded_columns = visited_columns;


                let noise_fn = self.noise_fn.clone();
                let send_column = self.send_chunk_column.clone();
                let send_chunk = self.send_chunks.clone();
                let cm = Arc::clone(&chunk_manager);
                let loaded_chunks = Arc::clone(&self.loaded_chunks);
                let expand_ring = Arc::clone(&self.expand_ring);

                self.pool.spawn(move || {

                    // Terrain gen
                    let chunk_manager = Arc::clone(&cm);
                    rayon::scope(move |s| {
                        // let chunk_manager = Arc::clone(&chunk_manager);

                        // let chunk_manager = Arc::clone(&chunk_manager);
                        for (x, z, mut column) in vec {
                            let chunk_manager = Arc::clone(&chunk_manager);
                            let send_column = send_column.clone();
                            s.spawn(move |s| {
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
                                    };
                                }
                                chunk_manager.add_chunk_column((x, z), column);
                                // send_column.send((x, z, column));
                            });
                        }
                    });

                    let now = Instant::now();

                    let chunk_manager = Arc::clone(&cm);
                    // Chunk face culling & AO
                    rayon::scope(move |s| {

                        let visited_chunks = Self::flood_fill_chunks(&chunk_manager, c_x, c_y, c_z, RENDER_DISTANCE);
                        // dbg!(&visited_chunks)
                        // for chunk in &visited_chunks {
                        //     dbg!(chunk);
                        // }
                        // dbg!((c_x, c_y, c_z));
                        // exit(0);
                        // println!("floodfill chunks\t{:#?} {}", Instant::now().duration_since(now), visited_chunks.len());
                        loaded_chunks.write().extend(visited_chunks.iter());

                        // let new_chunks = visited_chunks.iter().filter(|c| {
                        //     abs(c.0 - previous_chunk_at_player.0) > RENDER_DISTANCE ||
                        //         abs(c.1 - previous_chunk_at_player.1) > RENDER_DISTANCE ||
                        //         abs(c.2 - previous_chunk_at_player.2) > RENDER_DISTANCE
                        // });
                        let new_chunks = &visited_chunks;
                        // println!("LEN NEW CHUNKS {:?}", new_chunks.len());

                        // self.chunks_to_load.extend(new_chunks);

                        for &(c_x, c_y, c_z) in new_chunks {
                            let chunk_manager = Arc::clone(&chunk_manager);
                            let send_chunk = send_chunk.clone();
                            // dbg!("here {:?}", (c_x, c_y, c_z));
                            s.spawn(move |s| {
                                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                                    // println!("11 {:?}", (c_x, c_y, c_z));
                                    *chunk.is_rendered.write() = true;

                                    if chunk.is_empty() {
                                        *chunk.is_rendered.write() = true;
                                        // println!("RETURN");
                                        return;
                                    }
                                    // println!("22");
                                    let now = Instant::now();

                                    chunk_manager.update_all_blocks(c_x, c_y, c_z);
                                    // println!("ALL the generation {:?}", Instant::now().duration_since(now));

                                    send_chunk.send((c_x, c_y, c_z));
                                    // println!("33");
                                }
                            });
                        }
                        // self.chunks_to_load.clear();
                    });

                    *expand_ring.write() = true;
                });
            }
        }

        // Fix Dashmap synchronisation behaviour
        // let mut to_remove = Vec::new();
        // self.removed_columns.retain(|xz| {
        //     if let Some(column) = chunk_manager.remove_chunk_column(&xz) {
        //         to_remove.push(column);
        //         // println!("removing?");
        //         false
        //     } else {
        //         true
        //     }
        // });
        // self.chunk_column_pool.write().extend(to_remove.into_iter());

        // for (x, z, column) in self.receive_chunk_column.try_iter() {
        //     chunk_manager.add_chunk_column((x, z), column);
        // }


    }
}