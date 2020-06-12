use std::borrow::{Borrow, BorrowMut};
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
use num_traits::Zero;
use std::time::{Instant, Duration};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_VOLUME: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
// pub const CUBE_SIZE: u32 = 180;

#[derive(Default)]
pub struct ChunkManager {
    loaded_chunks: HashMap<(i32, i32), ChunkColumn>,
    ss: SuperSimplex,
    i: i32,
}

impl ChunkManager {
    pub fn new() -> ChunkManager {
        ChunkManager {
            loaded_chunks: HashMap::new(),
            ss: SuperSimplex::new(),
            i: 0,
        }
    }

    pub fn get_chunk(&self, x: i32, y: i32, z: i32) -> Option<&Chunk> {
        if y < 0 || y >= 16 {
            return None;
        }
        self.loaded_chunks.get(&(x, z)).and_then(|col| Some(&col.chunks[y as usize]))
    }

    pub fn get_chunk_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut Chunk> {
        if y < 0 || y >= 16 {
            return None;
        }
        self.loaded_chunks.get_mut(&(x, z)).and_then(|col| Some(&mut col.chunks[y as usize]))
    }

    pub fn generate_progressive_terrain(&mut self) {
        if self.i != 0 {
            return;
        }
        for i in 0..=5 {
            for j in 0..=i {
                let column = ChunkColumn::random();
                // column.chunks
                //
                println!("da");
                self.loaded_chunks.insert((j, i - j), column);
            }
        }

        self.i += 1;
    }

    pub fn generate_terrain(&mut self) {
        let render_distance = 5;

        let ss = SuperSimplex::new();
        for z in -render_distance..=render_distance {
            for x in -render_distance..=render_distance {
                self.loaded_chunks.insert((x, z), ChunkColumn::new());
            }
        }

        for x in -16 * render_distance..16 * render_distance {
            for z in -16 * render_distance..16 * render_distance {
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
        for z in 0..2 {
            for x in 0..2 {
                self.loaded_chunks.insert((x, z), ChunkColumn::random());
            }
        }
    }

    pub fn single(&mut self) {
        self.loaded_chunks.insert((0, 0), ChunkColumn::new());
        self.set_block(BlockID::Cobblestone, 0, 0, 0);
    }

    pub fn single_column(&mut self) {
        self.loaded_chunks.insert((0, 0), ChunkColumn::full_of_block());
    }

    // Transform global block coordinates into chunk local coordinates
    #[inline]
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

        match self.get_chunk(chunk_x, chunk_y, chunk_z) {
            None => None,
            Some(chunk) => Some(chunk.get_block(block_x, block_y, block_z)),
        }
    }

    pub fn set_block(&mut self, block: BlockID, x: i32, y: i32, z: i32) {
        let (chunk_x, chunk_y, chunk_z, block_x, block_y, block_z)
            = ChunkManager::get_chunk_coords(x, y, z);

        match self.get_chunk_mut(chunk_x, chunk_y, chunk_z) {
            None => None,
            Some(chunk) => Some(chunk.set_block(block, block_x, block_y, block_z)),
        };
    }

    pub fn is_solid_block_at(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z)
            .filter(|&block| block != BlockID::Air)
            .is_some()
    }

    // uv_map: the UV coordinates of all the block's faces
    // UV coordinates are composed of 4 floats, the first 2 are the bottom left corner and the last 2 are the top right corner (all between 0.0 and 1.0)
    // These specify the subtexture to use when rendering
    pub fn rebuild_dirty_chunks(&mut self, uv_map: &TexturePack) {
        // Collect all the dirty chunks
        // Nearby chunks can be also dirty if the change happens at the edge
        let mut dirty_chunks: HashSet<(i32, i32, i32)> = HashSet::new();
        for (&(x, z), chunk_column) in &self.loaded_chunks {
            for (y, chunk) in chunk_column.chunks.iter().enumerate() {
                if chunk.dirty {
                    dirty_chunks.insert((x, y as i32, z));
                }
                for &(rx, ry, rz) in &chunk.dirty_neighbours {
                    dirty_chunks.insert((x + rx, y as i32 + ry, z + rz));
                }
            }
        }
        if dirty_chunks.is_empty() {
            return;
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

            let chunk = self.get_chunk(coords.0, coords.1, coords.2);
            if let Some(chunk) = chunk {
                let active_faces_vec = active_faces.entry(coords).or_default();
                let ao_chunk = ao_chunks.entry(coords).or_default();

                let mut neighbours: [Option<Option<&Chunk>>; 27] = [None; 27];

                let to_array_coords = |x: i32, y: i32, z: i32| {
                    let x = x + 1;
                    let y = y + 1;
                    let z = z + 1;
                    assert!(x >= 0);
                    assert!(y >= 0);
                    assert!(z >= 0);
                    (x * 9 + y * 3 + z) as usize
                };

                let now = Instant::now();

                let mut active_faces_duration = Duration::default();
                let mut edge_ao = Duration::default();
                let mut internal_ao = Duration::default();

                for (b_x, b_y, b_z) in BlockIterator::new() {
                    let block = chunk.get_block(b_x, b_y, b_z);
                    if !block.is_air() {
                        let (g_x, g_y, g_z) = ChunkManager::get_global_coords((c_x, c_y, c_z, b_x, b_y, b_z));

                        let now = Instant::now();
                        let active_faces_of_block = self.get_active_faces_of_block(g_x, g_y, g_z);
                        active_faces_duration += Instant::now().duration_since(now);

                        // let active_faces_of_block = [true, false, true, false, false, false];
                        active_faces_vec.push(active_faces_of_block);

                        // Ambient Occlusion

                        // Optimisation
                        // If the block is not at the edge of the chunk then we
                        // can skip the chunk manager and iterate through the blocks
                        // of the same chunk
                        if b_x > 0 && b_x < 15 && b_y > 0 && b_y < 15 && b_z > 0 && b_z < 15 {
                            let now = Instant::now();

                            let chunk = &self.loaded_chunks.get(&(c_x, c_z)).unwrap().chunks[c_y as usize];
                            let mut does_occlude = |x: i32, y: i32, z: i32| {
                                !chunk.get_block((b_x as i32 + x) as u32, (b_y as i32 + y) as u32, (b_z as i32 + z) as u32).is_transparent_no_leaves()
                            };
                            ao_chunk.push(compute_ao_of_block(&mut does_occlude));

                            internal_ao += Instant::now().duration_since(now);
                        } else {
                            let now = Instant::now();
                            // let mut does_occlude = |x: i32, y: i32, z: i32| {
                            //     self.get_block(g_x + x, g_y + y, g_z + z)
                            //         .filter(|b| !b.is_transparent_no_leaves())
                            //         .is_some()
                            // };

                            // let mut does_occlude = |x: i32, y: i32, z: i32| false;

                            let mut does_occlude = |x: i32, y: i32, z: i32| {
                                let (
                                    c_xx,
                                    c_yy,
                                    c_zz,
                                    b_xx,
                                    b_yy,
                                    b_zz,
                                ) = ChunkManager::get_chunk_coords(g_x + x, g_y + y, g_z + z);

                                let r_xx = c_xx - c_x;
                                let r_yy = c_yy - c_y;
                                let r_zz = c_zz - c_z;

                                if r_xx.is_zero() && r_yy.is_zero() && r_zz.is_zero() {
                                    !chunk.get_block(b_xx as u32, b_yy as u32, b_zz as u32).is_transparent_no_leaves()
                                } else {
                                    let mut neighbour = neighbours[to_array_coords(r_xx, r_yy, r_zz)];
                                    if let None = &neighbour {
                                        // println!("huh");
                                        neighbours[to_array_coords(r_xx, r_yy, r_zz)] = Some(self.get_chunk(c_xx, c_yy, c_zz));
                                    }

                                    if let Some(Some(neighbour)) = neighbour {
                                        !neighbour.get_block(b_xx, b_yy, b_zz).is_transparent_no_leaves()
                                    } else {
                                        false
                                    }
                                }
                            };

                                // let mut xx = b_x as i32 + x;
                                // let mut yy = b_y as i32 + y;
                                // let mut zz = b_z as i32 + z;
                                //
                                // if xx < 0 || xx > 15 || yy < 0 || yy > 15 || zz < 0 || zz > 15 {
                                //     if xx < 0 {
                                //         xx = -1;
                                //     } else if xx > 15 {
                                //         xx = 1;
                                //     } else {
                                //         xx = 0;
                                //     }
                                //
                                //     if yy < 0 {
                                //         yy = -1;
                                //     } else if yy > 15 {
                                //         yy = 1;
                                //     } else {
                                //         yy = 0;
                                //     }
                                //
                                //     if zz < 0 {
                                //         zz = -1;
                                //     } else if zz > 15 {
                                //         zz = 1;
                                //     } else {
                                //         zz = 0;
                                //     }

                                // self.get_block(g_x + x, g_y + y, g_z + z)
                                //     .filter(|&b| !b.is_transparent_no_leaves())
                                //     .is_some()
                            // };
                            ao_chunk.push(compute_ao_of_block(&mut does_occlude));

                            edge_ao += Instant::now().duration_since(now);
                        }
                    }
                }

                println!("TIME {:#?}", Instant::now().duration_since(now));
                println!("TIME ACTIVE FACES {:#?}", active_faces_duration);
                println!("TIME INTERN AO {:#?}", internal_ao);
                println!("TIME EDGE AO {:#?}", edge_ao);
            }
        }

        // Update the VBOs of the dirty chunks
        for chunk_coords in &dirty_chunks {
            let chunk = self.get_chunk_mut(chunk_coords.0, chunk_coords.1, chunk_coords.2);
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
                    (6 * 10 * std::mem::size_of::<f32>() * n_visible_faces as usize) as isize,
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
                        vbo_offset += copied_vertices as isize * 10; // 5 floats per vertex
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
        for ((x, z), chunk_column) in &self.loaded_chunks {
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