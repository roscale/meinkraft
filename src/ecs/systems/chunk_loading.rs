use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};

use bit_vec::BitVec;
use noise::{NoiseFn, Point2, SuperSimplex};
use num_traits::abs;
use parking_lot::RwLock;
use specs::{Join, Read, ReadStorage, System};

use crate::chunk::{BlockID, ChunkColumn};
use crate::chunk_manager::ChunkManager;
use crate::constants::{RENDER_DISTANCE, WORLD_GENERATION_THREAD_POOL_SIZE, CHUNK_UPLOADS_PER_FRAME};
use crate::physics::Interpolator;
use crate::player::PlayerPhysicsState;
use crate::types::TexturePack;

pub struct ChunkLoading {
    noise_fn: SuperSimplex,
    chunk_column_pool: Arc<RwLock<Vec<Arc<ChunkColumn>>>>,
    chunk_at_player: (i32, i32, i32),

    send_chunks: Sender<(i32, i32, i32)>,
    receive_chunks: Receiver<(i32, i32, i32)>,

    expand_ring: Arc<RwLock<bool>>,
    world_generation_thread_pool: rayon::ThreadPool,
}

impl ChunkLoading {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        Self {
            noise_fn: SuperSimplex::new(),
            chunk_column_pool: Arc::new(RwLock::new({
                let mut vec = Vec::new();
                let matrix_width = (2 * (RENDER_DISTANCE + 2) + 1) as usize;

                let reserved_columns = matrix_width * matrix_width;
                vec.reserve(reserved_columns);
                for _ in 0..reserved_columns {
                    vec.push(Arc::new(ChunkColumn::new()));
                }
                vec
            })),
            chunk_at_player: (-100, -100, -100),
            send_chunks: tx,
            receive_chunks: rx,
            expand_ring: Arc::new(RwLock::new(true)),
            world_generation_thread_pool: rayon::ThreadPoolBuilder::new()
                .stack_size(4 * 1024 * 1024)
                .num_threads(*WORLD_GENERATION_THREAD_POOL_SIZE)
                .build().unwrap(),
        }
    }

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
                abs(z - c_z) <= distance
        };

        let mut queue = VecDeque::new();
        let mut ring = Vec::new();

        queue.push_back((x, y, z));
        ring.push((x, y, z));
        is_visited.set(coords_to_index(x, y, z), true);

        // Load the first tile
        if let Some(chunk) = chunk_manager.get_chunk(x, y, z) {
            if !*chunk.is_rendered.read() {
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
                        ring.push((x, y, z));
                        is_visited.set(coords_to_index(x, y, z), true);
                    }
                }
            }

            let mut unloaded_chunks = Vec::new();
            for &(x, y, z) in &ring {
                if  y >= 0 && y < 16 && !*chunk_manager.get_chunk(x, y, z).unwrap().is_rendered.read() {
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
        Read<'a, Arc<ChunkManager>>,
        Read<'a, TexturePack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_physics_state,
            chunk_manager,
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
            if c_xyz != self.chunk_at_player {
                self.chunk_at_player = c_xyz;

                let mut columns_to_remove = Vec::new();
                for (&(x, z), column) in chunk_manager.loaded_chunk_columns.read().iter() {
                    for (y, chunk) in column.chunks.iter().enumerate() {
                        let y = y as i32;
                        if abs(x - c_x) > RENDER_DISTANCE ||
                            abs(y - c_y) > RENDER_DISTANCE ||
                            abs(z - c_z) > RENDER_DISTANCE {

                            chunk.unload_from_gpu();
                            *chunk.is_rendered.write() = false;
                        }
                    }

                    if  abs(x - c_x) > RENDER_DISTANCE + 2 ||
                        abs(z - c_z) > RENDER_DISTANCE + 2 {

                        columns_to_remove.push((x, z));
                    }
                }

                for xz in columns_to_remove {
                    if let Some(column) = chunk_manager.remove_chunk_column(&xz) {
                        self.chunk_column_pool.write().push(column);
                    }
                }
            }

            for (c_x, c_y, c_z) in self.receive_chunks.try_iter().take(CHUNK_UPLOADS_PER_FRAME) {
                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                    chunk.upload_to_gpu(&texture_pack);
                    *chunk.is_rendered.write() = true;
                }
            }

            if *self.expand_ring.read() {
                *self.expand_ring.write() = false;

                let visited_columns = Self::flood_fill_columns(&chunk_manager, c_x, c_z, RENDER_DISTANCE + 2);
                let column_pool = Arc::clone(&self.chunk_column_pool);
                let new_columns = &visited_columns;

                let mut vec = Vec::new();
                for &(x, z) in new_columns {
                    vec.push((x, z, {
                        let mut column_pool = column_pool.write();
                        match column_pool.pop() {
                            Some(column) => {
                                for chunk in column.chunks.iter() {
                                    chunk.reset();
                                }
                                column
                            },
                            None => {
                                Arc::new(ChunkColumn::new())
                            }
                        }
                    }));
                }

                let noise_fn = self.noise_fn.clone();
                let send_chunk = self.send_chunks.clone();
                let cm = Arc::clone(&chunk_manager);
                let expand_ring = Arc::clone(&self.expand_ring);

                self.world_generation_thread_pool.spawn(move || {

                    // Terrain generation
                    let chunk_manager = Arc::clone(&cm);
                    rayon::scope(move |s| {
                        for (x, z, column) in vec {
                            let chunk_manager = Arc::clone(&chunk_manager);
                            s.spawn(move |_s| {
                                for b_x in 0..16 {
                                    for b_z in 0..16 {
                                        let x = 16 * x;
                                        let z = 16 * z;

                                        // Scale the input for the noise function
                                        let (xf, zf) = ((x + b_x as i32) as f64 / 64.0, (z + b_z as i32) as f64 / 64.0);
                                        let _y = 200.0;
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
                            });
                        }
                    });
                    let chunk_manager = Arc::clone(&cm);

                    // Chunk face culling & AO
                    rayon::scope(move |s| {
                        let new_chunks = Self::flood_fill_chunks(&chunk_manager, c_x, c_y, c_z, RENDER_DISTANCE);
                        for (c_x, c_y, c_z) in new_chunks {
                            let chunk_manager = Arc::clone(&chunk_manager);
                            let send_chunk = send_chunk.clone();

                            s.spawn(move |_s| {
                                if let Some(chunk) = chunk_manager.get_chunk(c_x, c_y, c_z) {
                                    *chunk.is_rendered.write() = true;
                                    if chunk.is_empty() {
                                        *chunk.is_rendered.write() = true;
                                        return;
                                    }
                                    chunk_manager.update_all_blocks(c_x, c_y, c_z);
                                    if let Err(err) = send_chunk.send((c_x, c_y, c_z)) {
                                        error!("{}", err);
                                    }
                                }
                            });
                        }
                    });
                    *expand_ring.write() = true;
                });
            }
        }
    }
}