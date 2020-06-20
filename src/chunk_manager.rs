use std::collections::{HashMap, HashSet};

use nalgebra::Matrix4;
use nalgebra_glm::{Mat4, vec3};

use crate::ambient_occlusion::compute_ao_of_block;
use crate::chunk::{BlockID, Chunk, ChunkColumn, BlockIterator};
use crate::shader_compilation::ShaderProgram;
use crate::types::TexturePack;
use std::sync::{Arc, RwLockWriteGuard};
use parking_lot::{RwLock, RawRwLock, MappedRwLockWriteGuard, RwLockReadGuard, MappedRwLockReadGuard};
use std::borrow::BorrowMut;

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
// pub const CUBE_SIZE: u32 = 180;

#[derive(Default)]
pub struct ChunkManager {
    loaded_chunk_columns: HashMap<(i32, i32), Arc<RwLock<Box<ChunkColumn>>>>,
    pub block_changelist: HashSet<(i32, i32, i32)>,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunk_columns: HashMap::new(),
            block_changelist: HashSet::new(),
        }
    }

    pub fn get_chunk(&self, x: i32, y: i32, z: i32) -> Option<MappedRwLockReadGuard<Chunk>> {
        if y < 0 || y > 15 {
            return None;
        }
        self.loaded_chunk_columns.get(&(x, z)).map(|column| {
            parking_lot::RwLockReadGuard::map(column.read(), |column| &column.chunks[y as usize])
        })
    }

    pub fn get_chunk_mut(&self, x: i32, y: i32, z: i32) -> Option<MappedRwLockWriteGuard<Chunk>> {
        if y < 0 || y > 15 {
            return None;
        }
        // self.loaded_chunk_columns.get_mut(&(x, z)).map(|column| &mut column.chunks[y as usize])
        self.loaded_chunk_columns.get(&(x, z)).map(
            |column| {
                parking_lot::RwLockWriteGuard::map(column.write(), |column| &mut column.chunks[y as usize])
            }
        )
    }

    pub fn add_chunk_column(&mut self, xz: (i32, i32), chunk_column: Arc<RwLock<Box<ChunkColumn>>>) {
        if !self.loaded_chunk_columns.contains_key(&xz) {
            self.loaded_chunk_columns.insert(xz, chunk_column);
            // self.fresh_chunk.insert(xz);
        }
    }

    pub fn remove_chunk_column(&mut self, xz: &(i32, i32)) -> Option<Arc<RwLock<Box<ChunkColumn>>>> {
        self.loaded_chunk_columns.remove(&xz)
    }

    // pub fn generate_terrain(&mut self) {
    //     let render_distance = 5;
    //
    //     let ss = SuperSimplex::new();
    //     for y in -render_distance..=render_distance {
    //         for z in -render_distance..=render_distance {
    //             for x in -render_distance..=render_distance {
    //                 self.add_chunk_column((x, y, z), Chunk::new());
    //             }
    //         }
    //     }
    //
    //     for x in -16 * render_distance..=16 * render_distance {
    //         for z in -16 * render_distance..=16 * render_distance {
    //             // Scale the input for the noise function
    //             let (xf, zf) = (x as f64 / 64.0, z as f64 / 64.0);
    //             let y = ss.get(Point2::from([xf, zf]));
    //             let y = (16.0 * (y + 1.0)) as i32;
    //
    //             // Ground layers
    //             self.set_block(BlockID::GrassBlock, x, y, z);
    //             self.set_block(BlockID::Dirt, x, y - 1, z);
    //             self.set_block(BlockID::Dirt, x, y - 2, z);
    //             self.set_block(BlockID::Cobblestone, x, y - 3, z);
    //
    //             // Trees
    //             if random::<u32>() % 100 < 1 {
    //                 let h = 5;
    //                 for i in y + 1..y + 1 + h {
    //                     self.set_block(BlockID::OakLog, x, i, z);
    //                 }
    //
    //                 for yy in y + h - 2..=y + h - 1 {
    //                     for xx in x - 2..=x + 2 {
    //                         for zz in z - 2..=z + 2 {
    //                             if xx != x || zz != z {
    //                                 self.set_block(BlockID::OakLeaves, xx, yy, zz);
    //                             }
    //                         }
    //                     }
    //                 }
    //
    //                 for xx in x - 1..=x + 1 {
    //                     for zz in z - 1..=z + 1 {
    //                         if xx != x || zz != z {
    //                             self.set_block(BlockID::OakLeaves, xx, y + h, zz);
    //                         }
    //                     }
    //                 }
    //
    //                 self.set_block(BlockID::OakLeaves, x, y + h + 1, z);
    //                 self.set_block(BlockID::OakLeaves, x + 1, y + h + 1, z);
    //                 self.set_block(BlockID::OakLeaves, x - 1, y + h + 1, z);
    //                 self.set_block(BlockID::OakLeaves, x, y + h + 1, z + 1);
    //                 self.set_block(BlockID::OakLeaves, x, y + h + 1, z - 1);
    //             }
    //         }
    //     }
    // }

    pub fn preload_some_chunks(&mut self) {
        for z in 0..2 {
            for x in 0..2 {
                self.add_chunk_column((x, z), Arc::new(RwLock::new(Box::new(ChunkColumn::new()))));
            }
        }
    }

    pub fn single(&mut self) {
        self.add_chunk_column((0, 0), Arc::new(RwLock::new(Box::new(ChunkColumn::new()))));
        self.set_block(BlockID::Cobblestone, 0, 0, 0);
    }

    pub fn single_chunk(&mut self) {
        self.add_chunk_column((0, 0), Arc::new(RwLock::new(Box::new(ChunkColumn::full_of_block(BlockID::Cobblestone)))));
    }

    // Transform global block coordinates into chunk local coordinates
    pub fn get_chunk_coords(x: i32, y: i32, z: i32) -> (i32, i32, i32, u32, u32, u32) {
        let chunk_x = if x < 0 { (x + 1) / 16 - 1 } else { x / 16 };
        let chunk_y = if y < 0 { (y + 1) / 16 - 1 } else { y / 16 };
        let chunk_z = if z < 0 { (z + 1) / 16 - 1 } else { z / 16 };

        let block_x = x.rem_euclid(16) as u32;
        let block_y = y.rem_euclid(16) as u32;
        let block_z = z.rem_euclid(16) as u32;

        (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
    }

    // Transform chunk local coordinates into global coordinates
    pub fn get_global_coords((chunk_x, chunk_y, chunk_z, block_x, block_y, block_z): (i32, i32, i32, u32, u32, u32)) -> (i32, i32, i32) {
        let x = 16 * chunk_x + block_x as i32;
        let y = 16 * chunk_y + block_y as i32;
        let z = 16 * chunk_z + block_z as i32;
        (x, y, z)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockID> {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        match self.get_chunk(chunk_x, chunk_y, chunk_z) {
            None => None,
            Some(chunk) => Some(chunk.get_block(block_x, block_y, block_z)),
        }
    }

    /// Replaces the block at (x, y, z) with `block`.
    ///
    /// This function should be used for terrain generation because it does not
    /// modify the changelist.
    pub fn set_block(&mut self, block: BlockID, x: i32, y: i32, z: i32) -> bool {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        match self.get_chunk_mut(chunk_x, chunk_y, chunk_z) {
            Some(mut chunk) => {
                chunk.set_block(block, block_x, block_y, block_z);
                true
            },
            None => false,
        }
    }

    /// Like `set_block` but it modifies the changelist.
    ///
    /// Should be used when an entity (player, mob etc.) interacts with the world.
    pub fn put_block(&mut self, block: BlockID, x: i32, y: i32, z: i32) {
        if self.set_block(block, x, y, z) {
            self.block_changelist.insert((x, y, z));
        }
    }

    pub fn is_solid_block_at(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z)
            .filter(|&block| block != BlockID::Air)
            .is_some()
    }

    pub fn update_all_blocks(&self, c_x: i32, c_y: i32, c_z: i32) {
        let mut neighbourhood = [None, None, None, None, None, None, None, None, None];
        for x in -1..=1 {
            for z in -1..=1 {
                neighbourhood[3 * (x + 1) as usize + (z + 1) as usize] = if x == 0 && z == 0 {
                    None
                } else {
                    Some(self.loaded_chunk_columns.get(&(c_x + x, c_z + z)).unwrap().read())
                };
            }
        }

        let mut this_column = self.loaded_chunk_columns.get(&(c_x, c_z)).unwrap().write();

        #[inline]
        fn block_at(column: &ChunkColumn, neighbourhood: &[Option<RwLockReadGuard<Box<ChunkColumn>>>; 9], c_x: i32, c_y: i32, c_z: i32, w_x: i32, w_y: i32, w_z: i32) -> BlockID {
            let to_index = |x: i32, z: i32| -> usize {
                3 * (x - c_x + 1) as usize + (z - c_z + 1) as usize
            };

            let (c_x_n, c_y_n, c_z_n, b_x, b_y, b_z) = ChunkManager::get_chunk_coords(w_x, w_y, w_z);

            if c_x == c_x_n && c_z == c_z_n {
                column.chunks[c_y_n as usize].get_block(b_x, b_y, b_z)
            } else {
                neighbourhood[to_index(c_x_n, c_z_n)].as_ref().unwrap().chunks[c_y_n as usize].get_block(b_x, b_y, b_z)
            }
        };

        #[inline]
        fn active_faces(column: &ChunkColumn, neighbourhood: &[Option<RwLockReadGuard<Box<ChunkColumn>>>; 9], c_x: i32, c_y: i32, c_z: i32, x: i32, y: i32, z: i32) -> [bool; 6] {
            let right = block_at(&column, &neighbourhood, c_x, c_y, c_z, x + 1, y, z).is_transparent();
            let left = block_at(&column, &neighbourhood, c_x, c_y, c_z, x - 1, y, z).is_transparent();
            let top = block_at(&column, &neighbourhood, c_x, c_y, c_z, x, y + 1, z).is_transparent();
            let bottom = block_at(&column, &neighbourhood, c_x, c_y, c_z, x, y - 1, z).is_transparent();
            let front = block_at(&column, &neighbourhood, c_x, c_y, c_z, x, y, z + 1).is_transparent();
            let back = block_at(&column, &neighbourhood, c_x, c_y, c_z, x, y, z - 1).is_transparent();
            [right, left, top, bottom, front, back]
        };

        for (b_x, b_y, b_z) in BlockIterator::new() {
            if this_column.chunks[c_y as usize].get_block(b_x, b_y, b_z) == BlockID::Air {
                continue;
            }
            let (w_x, w_y, w_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));
            // let (c_x, c_y, c_z, b_x, b_y, b_z) = ChunkManager::get_chunk_coords(w_x + 1, w_y, w_z);

            let af = active_faces(&this_column, &neighbourhood, c_x, c_y, c_z, w_x, w_y, w_z);
            let array_index = (b_y * CHUNK_SIZE * CHUNK_SIZE + b_z * CHUNK_SIZE + b_x) as usize;
            // let mut chunk = this_column.chunks[c_y as usize];

            // drop(this_column);
            // let mut this_column = self.loaded_chunk_columns.get(&(c_x, c_z)).unwrap().write();
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index, af[0]);
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index + 1, af[1]);
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index + 2, af[2]);
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index + 3, af[3]);
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index + 4, af[4]);
            this_column.chunks[c_y as usize].active_faces.set(6 * array_index + 5, af[5]);


            // Ambient Occlusion

            let block_ao = compute_ao_of_block(&|rx: i32, ry: i32, rz: i32| {
                !block_at(&this_column, &neighbourhood, c_x, c_y, c_z, w_x + rx, w_y + ry, w_z + rz).is_transparent_no_leaves()
            });

            this_column.chunks[c_y as usize].ao_vertices[array_index] = block_ao;
            // dbg!(af);
            // this_column.chunks[c_y as usize].active_faces[]

            // let nearby_block = chunk.get_block(b_x, b_y, b_z);
            // println!("{:?}", nearby_block);
        }
    }

    pub fn update_block(&mut self, c_x: i32, c_y: i32, c_z: i32, b_x: u32, b_y: u32, b_z: u32) {
        let array_index = (b_y * CHUNK_SIZE * CHUNK_SIZE + b_z * CHUNK_SIZE + b_x) as usize;
        let (w_x, w_y, w_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));

        let active_faces_of_block = {
            let chunk = self.get_chunk(c_x, c_y, c_z).unwrap();
            if chunk.get_block(b_x, b_y, b_z) == BlockID::Air {
                return;
            }
            self.get_active_faces_of_block(w_x, w_y, w_z)
        };

        {
            let mut chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();
            chunk.active_faces.set(6 * array_index, active_faces_of_block[0]);
            chunk.active_faces.set(6 * array_index + 1, active_faces_of_block[1]);
            chunk.active_faces.set(6 * array_index + 2, active_faces_of_block[2]);
            chunk.active_faces.set(6 * array_index + 3, active_faces_of_block[3]);
            chunk.active_faces.set(6 * array_index + 4, active_faces_of_block[4]);
            chunk.active_faces.set(6 * array_index + 5, active_faces_of_block[5]);
        }


        // chunk.active_faces.set(6 * array_index, true);
        // chunk.active_faces.set(6 * array_index + 1, true);
        // chunk.active_faces.set(6 * array_index + 2, true);
        // chunk.active_faces.set(6 * array_index + 3, true);
        // chunk.active_faces.set(6 * array_index + 4, true);
        // chunk.active_faces.set(6 * array_index + 5, true);

        // Ambient Occlusion

        let block_ao = compute_ao_of_block(&|rx: i32, ry: i32, rz: i32| {
            self.get_block(w_x + rx, w_y + ry, w_z + rz)
                .filter(|b| !b.is_transparent_no_leaves())
                .is_some()
        });

        let mut chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();
        chunk.ao_vertices[array_index] = block_ao;
        // chunk.ao_vertices[array_index] = [[0; 4]; 6];
    }

    pub fn rebuild_dirty_chunks(&mut self, uv_map: &TexturePack) {
        let mut changelist_per_chunk: HashMap<(i32, i32, i32), Vec<(u32, u32, u32)>> = HashMap::new();
        for &change in &self.block_changelist {
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let (
                            c_x, c_y, c_z,
                            b_x, b_y, b_z,
                        ) = ChunkManager::get_chunk_coords(change.0 + x, change.1 + y, change.2 + z);
                        changelist_per_chunk.entry((c_x, c_y, c_z)).or_default().push((b_x, b_y, b_z));
                    }
                }
            }
        }
        self.block_changelist.clear();

        // for &(c_x, c_y, c_z) in &self.fresh_chunk.clone() {
        //     for (b_x, b_y, b_z) in BlockIterator::new() {
        //         self.update_block(c_x, c_y, c_z, b_x, b_y, b_z);
        //     }
        // }
        // self.fresh_chunk.clear();

        for (&(c_x, c_y, c_z), dirty_blocks) in &changelist_per_chunk {
            if let None = self.get_chunk(c_x, c_y, c_z) {
                continue;
            }
            for &(b_x, b_y, b_z) in dirty_blocks {
                self.update_block(c_x, c_y, c_z, b_x, b_y, b_z);
            }
            self.get_chunk_mut(c_x, c_y, c_z).unwrap().upload_to_gpu(&uv_map);
        }
    }

    // An active face is a block face next to a transparent block that needs to be rendered
    pub fn get_active_faces_of_block(&self, x: i32, y: i32, z: i32) -> [bool; 6] {
        let right = self.get_block(x + 1, y, z).filter(|&b| !b.is_transparent()).is_none();
        let left = self.get_block(x - 1, y, z).filter(|&b| !b.is_transparent()).is_none();
        let top = self.get_block(x, y + 1, z).filter(|&b| !b.is_transparent()).is_none();
        let bottom = self.get_block(x, y - 1, z).filter(|&b| !b.is_transparent()).is_none();
        let front = self.get_block(x, y, z + 1).filter(|&b| !b.is_transparent()).is_none();
        let back = self.get_block(x, y, z - 1).filter(|&b| !b.is_transparent()).is_none();
        [right, left, top, bottom, front, back]
    }

    pub fn render_loaded_chunks(&self, program: &mut ShaderProgram) {
        for ((x, z), chunk_column) in &self.loaded_chunk_columns {
            for (ref y, chunk) in chunk_column.read().chunks.iter().enumerate() {
                // Skip rendering the chunk if there is nothing to draw
                if !chunk.is_rendered || chunk.vertices_drawn == 0 {
                    continue;
                }
                let model_matrix = {
                    let translate_matrix = Matrix4::new_translation(&vec3(
                        *x as f32, *y as f32, *z as f32).scale(16.0));
                    let rotate_matrix = Matrix4::from_euler_angles(
                        0.0f32,
                        0.0,
                        0.0,
                    );
                    let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(1.0f32, 1.0f32, 1.0f32));
                    translate_matrix * rotate_matrix * scale_matrix
                };

                gl_call!(gl::BindVertexArray(chunk.vao));
                program.set_uniform_matrix4fv("model", model_matrix.as_ptr());
                gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, chunk.vertices_drawn as i32));
            }
        }
    }
}