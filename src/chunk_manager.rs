use std::collections::{HashMap, HashSet};
use crate::chunk::{Chunk, BlockID};
use nalgebra_glm::{Mat4, vec3};
use crate::shader_compilation::ShaderProgram;
use nalgebra::{Vector3, Matrix4, clamp};
use std::ops::Mul;
use std::borrow::Borrow;
use std::hash::Hash;
use crate::shapes::{write_unit_cube_to_ptr};
use std::os::raw::c_void;
use std::cell::RefCell;
use noise::{SuperSimplex, NoiseFn, Point3, Point2};
use crate::block_texture_sides::{BlockFaces, get_uv_every_side};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const CUBE_SIZE: u32 = 180;

type Sides = (bool, bool, bool, bool, bool, bool);

pub struct ChunkManager {
    loaded_chunks: HashMap<(i32, i32, i32), Chunk>,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunks: HashMap::new()
        }
    }

    pub fn preload_some_chunks(&mut self) {
        for y in 0..2 {
            for z in 0..2 {
                for x in 0..2 {
                    self.loaded_chunks.insert((x, y, z), Chunk::random());
                }
            }
        }
    }

    pub fn empty_99(&mut self) {
        self.loaded_chunks.insert((0, 0, 0), Chunk::full_of_block(BlockID::Cobblestone));
    }

    pub fn simplex(&mut self) {
        let n = 5;

        let ss = SuperSimplex::new();
        for y in 0..16 {
            for z in -n..=n {
                for x in -n..=n {
                    self.loaded_chunks.insert((x, y, z), Chunk::empty());
                }
            }
        }



        for x in -16 * n..16 * n {
            for z in -16 * n..16 * n {
                let (xf, zf) = (x as f64 / 64.0, z as f64 / 64.0);
                let y = ss.get(Point2::from([xf, zf]));
                let y = (16.0 * (y + 1.0)) as i32;
                self.set_block(BlockID::GrassBlock, x, y, z);
                self.set_block(BlockID::Dirt, x, y - 1, z);
                self.set_block(BlockID::Dirt, x, y - 2, z);
                self.set_block(BlockID::Cobblestone, x, y - 3, z);
            }
        }


    }

    fn get_chunk_and_block_coords(x: i32, y: i32, z: i32) -> (i32, i32, i32, u32, u32, u32) {
        let chunk_x = if x < 0 { (x + 1) / 16 - 1 } else { x / 16 };
        let chunk_y = if y < 0 { (y + 1) / 16 - 1 } else { y / 16 };
        let chunk_z = if z < 0 { (z + 1) / 16 - 1 } else { z / 16 };

        let block_x = x.rem_euclid(16) as u32;
        let block_y = y.rem_euclid(16) as u32;
        let block_z = z.rem_euclid(16) as u32;

        (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
    }

    fn get_global_coords((chunk_x, chunk_y, chunk_z, block_x, block_y, block_z): (i32, i32, i32, u32, u32, u32)) -> (i32, i32, i32) {
        let x = 16 * chunk_x + block_x as i32;
        let y = 16 * chunk_y + block_y as i32;
        let z = 16 * chunk_z + block_z as i32;
        (x, y, z)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockID> {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_and_block_coords(x, y, z);

        self.loaded_chunks.get((chunk_x, chunk_y, chunk_z).borrow()).and_then(|chunk| {
            Some(chunk.get_block(block_x, block_y, block_z))
        })
    }

    pub fn set_block(&mut self, block: BlockID, x: i32, y: i32, z: i32) {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_and_block_coords(x, y, z);

        self.loaded_chunks.get_mut((chunk_x, chunk_y, chunk_z).borrow()).map(|chunk| {
            chunk.set_block(block, block_x, block_y, block_z)
        });
    }

    pub fn rebuild_dirty_chunks(&mut self, uv_map: &HashMap<BlockID, BlockFaces<(f32, f32, f32, f32)>>) {
        let mut dirty_chunks: HashSet<(i32, i32, i32)> = HashSet::new();
        // Nearby chunks can be also dirty if the change happens at the edge
        for (&(x, y, z), chunk) in &self.loaded_chunks {
            if chunk.dirty {
                dirty_chunks.insert((x, y, z));
            }
            for &(rx, ry, rz) in &chunk.dirty_neighbours {
                dirty_chunks.insert((x + rx, y + ry, z + rz));
            }
        }

        let mut active_sides: HashMap<(i32, i32, i32), Vec<Sides>> = HashMap::new();
        for &coords in &dirty_chunks {
            let (c_x, c_y, c_z) = coords;
            let chunk = self.loaded_chunks.get(&coords);
            if let Some(chunk) = chunk {
                let sides_vec = active_sides.entry(coords).or_default();
                for b_y in 0..CHUNK_SIZE {
                    for b_z in 0..CHUNK_SIZE {
                        for b_x in 0..CHUNK_SIZE {
                            let (g_x, g_y, g_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));
                            sides_vec.push(self.get_active_sides_of_block(g_x, g_y, g_z))
                        }
                    }
                }
            }
        }

        for coords in &dirty_chunks {
            let mut i = 0;
            let chunk = self.loaded_chunks.get_mut(&coords);
            if let Some(chunk) = chunk {
                let vbo_ptr: *mut f32 = gl_call!(gl::MapNamedBuffer(chunk.vbo, gl::WRITE_ONLY)) as *mut f32;

                let sides_vec = active_sides.get(&coords).unwrap();
                let mut j = 0;

                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let block = chunk.get_block(x, y, z);
                            if block != BlockID::Air {
                                let active_sides = sides_vec[j];

                                let uvs = uv_map.get(&block).unwrap().clone();
                                let uvs = get_uv_every_side(uvs);

                                let copied_vertices = unsafe { write_unit_cube_to_ptr(vbo_ptr.offset(i), x as f32, y as f32, z as f32, uvs, active_sides) };
                                // let cube_array = unit_cube_array(x as f32, y as f32, z as f32, uv_bl, uv_tr, active_sides);
                                // gl_call!(gl::NamedBufferSubData(chunk.vbo, (i * std::mem::size_of::<f32>()) as isize, (cube_array.len() * std::mem::size_of::<f32>()) as isize, cube_array.as_ptr() as *mut c_void));
                                chunk.vertices_drawn += copied_vertices;
                                i += copied_vertices as isize * 5;
                            }
                            j += 1;
                        }
                    }
                }
                gl_call!(gl::UnmapNamedBuffer(chunk.vbo));

                chunk.dirty = false;
                chunk.dirty_neighbours.clear();
            }
        }
    }

    pub fn get_active_sides_of_block(&self, x: i32, y: i32, z: i32) -> (bool, bool, bool, bool, bool, bool) {
        let right = self.get_block(x + 1, y, z).filter(|&b| b != BlockID::Air).is_none();
        let left = self.get_block(x - 1, y, z).filter(|&b| b != BlockID::Air).is_none();
        let top = self.get_block(x, y + 1, z).filter(|&b| b != BlockID::Air).is_none();
        let bottom = self.get_block(x, y - 1, z).filter(|&b| b != BlockID::Air).is_none();
        let front = self.get_block(x, y, z + 1).filter(|&b| b != BlockID::Air).is_none();
        let back = self.get_block(x, y, z - 1).filter(|&b| b != BlockID::Air).is_none();
        (right, left, top, bottom, front, back)
    }

    pub fn render_loaded_chunks(&self, program: &mut ShaderProgram) {
        for ((x, y, z), chunk) in &self.loaded_chunks {
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