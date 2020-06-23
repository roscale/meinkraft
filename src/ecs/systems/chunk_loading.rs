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

pub struct ChunkLoading {
    noise_fn: SuperSimplex,
    chunk_column_pool: Arc<RwLock<Vec<Arc<ChunkColumn>>>>,
    loaded_columns: Vec<(i32, i32)>,
    removed_columns: Vec<(i32, i32)>,
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
            chunk_column_pool: Arc::new(RwLock::new(Vec::new())),
            loaded_columns: Vec::new(),
            removed_columns: Vec::new(),
            loaded_chunks: Vec::new(),
            chunks_to_load: VecDeque::new(),
            chunk_at_player: (-100, -100, -100),
            send_chunk_column: tx,
            receive_chunk_column: rx,
            send_chunks: tx2,
            receive_chunks: rx2,
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

    fn flood_fill_3d(chunk_manager: &ChunkManager, x: i32, y: i32, z: i32, distance: i32) -> Vec<(i32, i32, i32)> {
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

        let mut visited_chunks = Vec::new();
        let mut queue = VecDeque::new();
        queue.reserve(100);
        queue.push_back((x, y, z, distance));
        is_visited.set(coords_to_index(x, y, z), true);

        while !queue.is_empty() {
            let (x, y, z, dist) = queue.pop_front().unwrap();

            if y >= 0 && y < 16 {
                visited_chunks.push((x, y, z));
                if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
                    if chunk.is_fully_opaque() {
                        continue;
                    }
                }
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

                println!("old {:?}", previous_chunk_at_player);
                println!("new {:?}", self.chunk_at_player);

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
                }).cloned().collect_vec();

                let now = Instant::now();
                for xz in old_columns {
                    self.removed_columns.push(xz);
                }
                println!("Removing old columns\t{:#?}", Instant::now().duration_since(now));

                let noise_fn = self.noise_fn.clone();
                let send_column = self.send_chunk_column.clone();
                let send_chunk = self.send_chunks.clone();
                let column_pool = Arc::clone(&self.chunk_column_pool);
                let chunk_manager = Arc::clone(&chunk_manager);

                let now = Instant::now();
                let visited_columns = Self::flood_fill_2d(c_x, c_z, RENDER_DISTANCE + 2);
                println!("floodfill columns\t{:#?} {} {:?}", Instant::now().duration_since(now), visited_columns.len(), (c_x, c_z));

                let new_columns = visited_columns
                    .iter()
                    .filter(|(x, z)| {
                        abs(x - previous_chunk_at_player.0)
                            + abs(z - previous_chunk_at_player.2) > RENDER_DISTANCE + 2
                    });

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
                                println!("ALLOC");
                                Arc::new(ChunkColumn::new())
                            }
                        }
                    }));
                }
                println!("terrain gen\t{:#?}", Instant::now().duration_since(now));

                let now = Instant::now();
                let visited_chunks = Self::flood_fill_3d(&chunk_manager, c_x, c_y, c_z, RENDER_DISTANCE);
                println!("floodfill chunks\t{:#?} {}", Instant::now().duration_since(now), visited_chunks.len());

                self.loaded_columns = visited_columns;
                self.loaded_chunks = visited_chunks.clone();

                self.pool.spawn(move || {
                    // Terarin gen
                    rayon::scope(move |s| {
                        for (x, z, mut column) in vec {
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
                                send_column.send((x, z, column));
                            });
                        }
                    });

                    // Chunk face culling & AO
                    rayon::scope(move |s| {

                        let new_chunks = visited_chunks.iter().filter(|c| {
                            abs(c.0 - previous_chunk_at_player.0)
                                + abs(c.1 - previous_chunk_at_player.1)
                                + abs(c.2 - previous_chunk_at_player.2) > RENDER_DISTANCE
                        });
                        // self.chunks_to_load.extend(new_chunks);

                        for &(c_x, c_y, c_z) in new_chunks {
                            let chunk_manager = Arc::clone(&chunk_manager);
                            let send_chunk = send_chunk.clone();
                            s.spawn(move |s| {
                                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                                    if chunk.is_empty() {
                                        return;
                                    }
                                    chunk_manager.update_all_blocks(c_x, c_y, c_z);
                                    send_chunk.send((c_x, c_y, c_z));
                                }
                            });
                        }
                        // self.chunks_to_load.clear();
                    });
                });
            }
        }

        // Fix Dashmap synchronisation behaviour
        let mut to_remove = Vec::new();
        self.removed_columns.retain(|xz| {
            if let Some(column) = chunk_manager.remove_chunk_column(&xz) {
                to_remove.push(column);
                // println!("removing?");
                false
            } else {
                true
            }
        });
        self.chunk_column_pool.write().extend(to_remove.into_iter());

        for (x, z, column) in self.receive_chunk_column.try_iter() {
            chunk_manager.add_chunk_column((x, z), column);
        }

        if let Ok((c_x, c_y, c_z)) = self.receive_chunks.try_recv() {
            if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                chunk.upload_to_gpu(&texture_pack);
                *chunk.is_rendered.write() = true;
            }
        }
    }
}