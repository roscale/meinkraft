use std::collections::{HashMap, HashSet};
use crate::chunk::{Chunk, BlockID, BlockIterator};
use nalgebra_glm::{Mat4, vec3};
use crate::shader_compilation::ShaderProgram;
use nalgebra::Matrix4;
use std::borrow::Borrow;
use crate::shapes::{write_unit_cube_to_ptr};
use noise::{SuperSimplex, NoiseFn, Point2};
use crate::block_texture_faces::BlockFaces;
use rand::random;
use crate::types::{UVCoords};
use std::ptr::null;
use crate::ambient_occlusion::compute_ao_of_block;

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
// pub const CUBE_SIZE: u32 = 180;

pub struct ChunkManager {
    loaded_chunks: HashMap<(i32, i32, i32), Chunk>,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunks: HashMap::new()
        }
    }

    pub fn generate_terrain(&mut self) {
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
                // Scale the input for the noise function
                let (xf, zf) = (x as f64 / 64.0, z as f64 / 64.0);
                let y = ss.get(Point2::from([xf, zf]));
                let y = (16.0 * (y + 1.0)) as i32;

                // Ground layers
                self.set_block(BlockID::GrassBlock, x, y, z);
                self.set_block(BlockID::Dirt, x, y - 1, z);
                self.set_block(BlockID::Dirt, x, y - 2, z);
                self.set_block(BlockID::Cobblestone, x, y - 3, z);

                // Trees
                if random::<u32>() % 100 < 1 {
                    let h = 5;
                    for i in y + 1..y + 1 + h {
                        self.set_block(BlockID::OakLog, x, i, z);
                    }

                    for yy in y + h - 2..=y + h - 1 {
                        for xx in x - 2..=x + 2 {
                            for zz in z - 2..=z + 2 {
                                if xx != x || zz != z {
                                    self.set_block(BlockID::OakLeaves, xx, yy, zz);
                                }
                            }
                        }
                    }

                    for xx in x - 1..=x + 1 {
                        for zz in z - 1..=z + 1 {
                            if xx != x || zz != z {
                                self.set_block(BlockID::OakLeaves, xx, y + h, zz);
                            }
                        }
                    }

                    self.set_block(BlockID::OakLeaves, x, y + h + 1, z);
                    self.set_block(BlockID::OakLeaves, x + 1, y + h + 1, z);
                    self.set_block(BlockID::OakLeaves, x - 1, y + h + 1, z);
                    self.set_block(BlockID::OakLeaves, x, y + h + 1, z + 1);
                    self.set_block(BlockID::OakLeaves, x, y + h + 1, z - 1);
                }
            }
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

    pub fn single(&mut self) {
        self.loaded_chunks.insert((0, 0, 0), Chunk::full_of_block(BlockID::Cobblestone));
    }

    // Transform global block coordinates into chunk local coordinates
    fn get_chunk_coords(x: i32, y: i32, z: i32) -> (i32, i32, i32, u32, u32, u32) {
        let chunk_x = if x < 0 { (x + 1) / 16 - 1 } else { x / 16 };
        let chunk_y = if y < 0 { (y + 1) / 16 - 1 } else { y / 16 };
        let chunk_z = if z < 0 { (z + 1) / 16 - 1 } else { z / 16 };

        let block_x = x.rem_euclid(16) as u32;
        let block_y = y.rem_euclid(16) as u32;
        let block_z = z.rem_euclid(16) as u32;

        (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
    }

    // Transform chunk local coordinates into global coordinates
    fn get_global_coords((chunk_x, chunk_y, chunk_z, block_x, block_y, block_z): (i32, i32, i32, u32, u32, u32)) -> (i32, i32, i32) {
        let x = 16 * chunk_x + block_x as i32;
        let y = 16 * chunk_y + block_y as i32;
        let z = 16 * chunk_z + block_z as i32;
        (x, y, z)
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockID> {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        self.loaded_chunks.get((chunk_x, chunk_y, chunk_z).borrow()).and_then(|chunk| {
            Some(chunk.get_block(block_x, block_y, block_z))
        })
    }

    pub fn set_block(&mut self, block: BlockID, x: i32, y: i32, z: i32) {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        self.loaded_chunks.get_mut((chunk_x, chunk_y, chunk_z).borrow()).map(|chunk| {
            chunk.set_block(block, block_x, block_y, block_z)
        });
    }

    pub fn is_solid_block_at(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z)
            .filter(|&block| block != BlockID::Air)
            .is_some()
    }

    // uv_map: the UV coordinates of all the block's faces
    // UV coordinates are composed of 4 floats, the first 2 are the bottom left corner and the last 2 are the top right corner (all between 0.0 and 1.0)
    // These specify the subtexture to use when rendering
    pub fn rebuild_dirty_chunks(&mut self, uv_map: &HashMap<BlockID, BlockFaces<UVCoords>>) {
        // Collect all the dirty chunks
        // Nearby chunks can be also dirty if the change happens at the edge
        let mut dirty_chunks: HashSet<(i32, i32, i32)> = HashSet::new();
        for (&(x, y, z), chunk) in &self.loaded_chunks {
            if chunk.dirty {
                dirty_chunks.insert((x, y, z));
            }
            for &(rx, ry, rz) in &chunk.dirty_neighbours {
                dirty_chunks.insert((x + rx, y + ry, z + rz));
            }
        }

        /*
            Optimization:
                If 2 solid blocks are touching, don't render the faces where they touch.
                Render only the faces that are next to a transparent block (AIR for example)
         */
        type ChunkCoords = (i32, i32, i32);

        type Sides = [bool; 6];
        let mut active_faces: HashMap<ChunkCoords, Vec<Sides>> = HashMap::new();

        type CubeAO = [[u8; 4]; 6];
        let mut ao_chunks: HashMap<ChunkCoords, Vec<CubeAO>> = HashMap::new();

        for &coords in &dirty_chunks {
            let (c_x, c_y, c_z) = coords;
            let chunk = self.loaded_chunks.get(&coords);
            if let Some(chunk) = chunk {
                let active_faces_vec = active_faces.entry(coords).or_default();
                let ao_chunk = ao_chunks.entry(coords).or_default();

                for (b_x, b_y, b_z) in BlockIterator::new() {
                    let block = chunk.get_block(b_x, b_y, b_z);
                    if !block.is_air() {
                        let (g_x, g_y, g_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));
                        let active_faces_of_block = self.get_active_faces_of_block(g_x, g_y, g_z);
                        active_faces_vec.push(active_faces_of_block);

                        // Ambient Occlusion
                        let does_occlude = |x: i32, y: i32, z: i32| {
                            self.get_block(g_x + x, g_y + y, g_z + z)
                                .filter(|&b| !b.is_transparent_no_leaves())
                                .is_some()
                        };
                        ao_chunk.push(compute_ao_of_block(&does_occlude));
                    }
                }
            }
        }

        // Update the VBOs of the dirty chunks
        for chunk_coords in &dirty_chunks {
            let chunk = self.loaded_chunks.get_mut(&chunk_coords);
            // We check for a valid chunk because maybe the calculated neighbour chunk does not exist
            if let Some(chunk) = chunk {
                chunk.dirty = false;
                chunk.dirty_neighbours.clear();
                chunk.vertices_drawn = 0;

                let sides = active_faces.get(&chunk_coords).unwrap();
                let n_visible_faces = sides.iter().map(|faces| faces.iter()
                    .fold(0, |acc, &b| acc + b as u32))
                    .fold(0, |acc, n| acc + n);

                if n_visible_faces == 0 {
                    continue;
                }

                // Initialize the VBO
                gl_call!(gl::NamedBufferData(chunk.vbo,
                    (6 * 9 * std::mem::size_of::<f32>() * n_visible_faces as usize) as isize,
                    null(),
                    gl::DYNAMIC_DRAW));

                // Map VBO to virtual memory
                let vbo_ptr: *mut f32 = gl_call!(gl::MapNamedBuffer(chunk.vbo, gl::WRITE_ONLY)) as *mut f32;
                let mut vbo_offset = 0;

                let sides_vec = active_faces.get(&chunk_coords).unwrap();
                let ao_vec = ao_chunks.get(&chunk_coords).unwrap();
                let mut j = 0;

                for (x, y, z) in BlockIterator::new() {
                    let block = chunk.get_block(x, y, z);
                    if block != BlockID::Air {
                        let active_sides = sides_vec[j];
                        let ao_block = ao_vec[j];

                        let uvs = uv_map.get(&block).unwrap().clone();
                        let uvs = uvs.get_uv_of_every_face();

                        let copied_vertices = unsafe { write_unit_cube_to_ptr(vbo_ptr.offset(vbo_offset), x as f32, y as f32, z as f32, uvs, active_sides, ao_block) };
                        // let cube_array = unit_cube_array(x as f32, y as f32, z as f32, uv_bl, uv_tr, active_sides);
                        // gl_call!(gl::NamedBufferSubData(chunk.vbo, (i * std::mem::size_of::<f32>()) as isize, (cube_array.len() * std::mem::size_of::<f32>()) as isize, cube_array.as_ptr() as *mut c_void));
                        chunk.vertices_drawn += copied_vertices;
                        vbo_offset += copied_vertices as isize * 9; // 5 floats per vertex
                        j += 1;
                    }

                }
                gl_call!(gl::UnmapNamedBuffer(chunk.vbo));
            }
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
        for ((x, y, z), chunk) in &self.loaded_chunks {
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