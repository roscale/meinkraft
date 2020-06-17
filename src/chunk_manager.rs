use std::collections::{HashMap, HashSet};
use std::ptr::null;

use nalgebra::Matrix4;
use nalgebra_glm::{Mat4, vec3};
use noise::{NoiseFn, Point2, SuperSimplex};
use rand::random;

use crate::ambient_occlusion::compute_ao_of_block;
use crate::chunk::{BlockID, BlockIterator, Chunk, ChunkColumn};
use crate::shader_compilation::ShaderProgram;
use crate::shapes::write_unit_cube_to_ptr;
use crate::types::TexturePack;

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
// pub const CUBE_SIZE: u32 = 180;

#[derive(Default)]
pub struct ChunkManager {
    loaded_chunk_columns: HashMap<(i32, i32), Box<ChunkColumn>>,
    fresh_chunk: HashSet<(i32, i32, i32)>,
    pub block_changelist: HashSet<(i32, i32, i32)>,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunk_columns: HashMap::new(),
            fresh_chunk: HashSet::new(),
            block_changelist: HashSet::new(),
        }
    }

    pub fn get_chunk(&self, x: i32, y: i32, z: i32) -> Option<&Chunk> {
        self.loaded_chunk_columns.get(&(x, z)).map(|column| &column.chunks[y as usize])
    }

    pub fn get_chunk_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut Chunk> {
        self.loaded_chunk_columns.get_mut(&(x, z)).map(|column| &mut column.chunks[y as usize])
    }

    pub fn add_chunk_column(&mut self, xz: (i32, i32), chunk_column: Box<ChunkColumn>) {
        if !self.loaded_chunk_columns.contains_key(&xz) {
            self.loaded_chunk_columns.insert(xz, chunk_column);
            // self.fresh_chunk.insert(xz);
        }
    }

    pub fn remove_chunk(&mut self, xz: &(i32, i32)) {
        self.loaded_chunk_columns.remove(&xz);
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
                self.add_chunk_column((x, z), Box::new(ChunkColumn::new()));
            }
        }
    }

    pub fn single(&mut self) {
        self.add_chunk_column((0, 0), Box::new(ChunkColumn::new()));
        self.set_block(BlockID::Cobblestone, 0, 0, 0);
    }

    pub fn single_chunk(&mut self) {
        self.add_chunk_column((0, 0), Box::new(ChunkColumn::full_of_block(BlockID::Cobblestone)));
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
            Some(chunk) => {
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

    pub fn update_block(&mut self, c_x: i32, c_y: i32, c_z: i32, b_x: u32, b_y: u32, b_z: u32) {
        let chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();
        if chunk.get_block(b_x, b_y, b_z) == BlockID::Air {
            return;
        }
        let array_index = (b_y * CHUNK_SIZE * CHUNK_SIZE + b_z * CHUNK_SIZE + b_x) as usize;
        let (w_x, w_y, w_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));
        let active_faces_of_block = self.get_active_faces_of_block(w_x, w_y, w_z);

        let chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();
        chunk.active_faces.set(6 * array_index, active_faces_of_block[0]);
        chunk.active_faces.set(6 * array_index + 1, active_faces_of_block[1]);
        chunk.active_faces.set(6 * array_index + 2, active_faces_of_block[2]);
        chunk.active_faces.set(6 * array_index + 3, active_faces_of_block[3]);
        chunk.active_faces.set(6 * array_index + 4, active_faces_of_block[4]);
        chunk.active_faces.set(6 * array_index + 5, active_faces_of_block[5]);

        // Ambient Occlusion

        let block_ao = compute_ao_of_block(&|rx: i32, ry: i32, rz: i32| {
            self.get_block(w_x + rx, w_y + ry, w_z + rz)
                .filter(|b| !b.is_transparent_no_leaves())
                .is_some()
        });

        let mut chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();
        chunk.ao_vertices[array_index] = block_ao;
    }

    pub fn upload_chunk_to_gpu(&mut self, c_x: i32, c_y: i32, c_z: i32, texture_pack: &TexturePack) {
        let mut chunk = self.get_chunk_mut(c_x, c_y, c_z).unwrap();

        let n_visible_faces = chunk.active_faces.iter().fold(0, |acc, b| acc + b as i32);
        if n_visible_faces == 0 {
            return;
        }

        // Initialize the VBO
        gl_call!(gl::NamedBufferData(chunk.vbo,
                (6 * 10 * std::mem::size_of::<f32>() * n_visible_faces as usize) as isize,
                null(),
                gl::DYNAMIC_DRAW));

        // Map VBO to virtual memory
        let vbo_ptr: *mut f32 = gl_call!(gl::MapNamedBuffer(chunk.vbo, gl::WRITE_ONLY)) as *mut f32;
        let mut vbo_offset = 0;

        chunk.vertices_drawn = 0;
        let sides_vec = &chunk.active_faces;
        let ao_vec = &chunk.ao_vertices;
        let mut j = 0;

        for (x, y, z) in BlockIterator::new() {
            let block = chunk.get_block(x, y, z);
            if block != BlockID::Air {
                let active_sides = [
                    sides_vec[6 * j],
                    sides_vec[6 * j + 1],
                    sides_vec[6 * j + 2],
                    sides_vec[6 * j + 3],
                    sides_vec[6 * j + 4],
                    sides_vec[6 * j + 5],
                ];

                let ao_block = ao_vec[j];

                let uvs = texture_pack.get(&block).unwrap().clone();
                let uvs = uvs.get_uv_of_every_face();

                let copied_vertices = unsafe { write_unit_cube_to_ptr(vbo_ptr.offset(vbo_offset), x as f32, y as f32, z as f32, uvs, active_sides, ao_block) };
                // let cube_array = unit_cube_array(x as f32, y as f32, z as f32, uv_bl, uv_tr, active_sides);
                // gl_call!(gl::NamedBufferSubData(chunk.vbo, (i * std::mem::size_of::<f32>()) as isize, (cube_array.len() * std::mem::size_of::<f32>()) as isize, cube_array.as_ptr() as *mut c_void));
                chunk.vertices_drawn += copied_vertices;
                vbo_offset += copied_vertices as isize * 10; // 5 floats per vertex
            }
            j += 1;
        }
        gl_call!(gl::UnmapNamedBuffer(chunk.vbo));
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
            self.upload_chunk_to_gpu(c_x, c_y, c_z, &uv_map);
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
            for (ref y, chunk) in chunk_column.chunks.iter().enumerate() {
                // Skip rendering the chunk if there is nothing to draw
                if chunk.vertices_drawn == 0 {
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