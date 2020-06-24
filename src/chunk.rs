use bit_vec::BitVec;
use rand::{random, Rng};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use std::ptr::null;

use crate::chunk_manager::{CHUNK_SIZE, CHUNK_VOLUME};
use crate::types::TexturePack;
use crate::shapes::write_unit_cube_to_ptr;
use parking_lot::RwLock;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BlockID {
    Air,
    Dirt,
    GrassBlock,
    Stone,
    Cobblestone,
    Bedrock,
    Obsidian,
    OakLog,
    OakLeaves,
    OakPlanks,
    Glass,
    Urss,
    Hitler,
    Debug,
    Debug2,
}

impl BlockID {
    #[inline]
    pub fn is_air(&self) -> bool {
        self == &BlockID::Air
    }
    #[inline]
    pub fn is_transparent(&self) -> bool {
        match self {
            &BlockID::Air |
            &BlockID::OakLeaves |
            &BlockID::Glass => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_opaque(&self) -> bool {
        !self.is_transparent()
    }
    #[inline]
    pub fn is_transparent_not_air(&self) -> bool {
        match self {
            &BlockID::OakLeaves |
            &BlockID::Glass => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_transparent_no_leaves(&self) -> bool {
        match self {
            &BlockID::Air |
            &BlockID::Glass => true,
            _ => false
        }
    }
}

impl Distribution<BlockID> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockID {
        match rng.gen_range(1, 4) {
            // 0 => BlockID::AIR,
            1 => BlockID::Dirt,
            2 => BlockID::Cobblestone,
            3 => BlockID::Obsidian,
            _ => BlockID::Air,
        }
    }
}

fn create_vao_vbo() -> (u32, u32) {
    let mut vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(vao, 1, 3 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 1, 0));

    // Normals
    gl_call!(gl::EnableVertexArrayAttrib(vao, 2));
    gl_call!(gl::VertexArrayAttribFormat(vao, 2, 3 as i32, gl::FLOAT, gl::FALSE, 6 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 2, 0));

    // Ambient occlusion
    gl_call!(gl::EnableVertexArrayAttrib(vao, 3));
    gl_call!(gl::VertexArrayAttribFormat(vao, 3, 1 as i32, gl::FLOAT, gl::FALSE, 9 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 3, 0));

    let mut vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut vbo));
    // We intentionally don't initialize the buffer's data store because it's dynamically created
    // when the chunk is invalidated

    gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (10 * std::mem::size_of::<f32>()) as i32));
    (vao, vbo)
}

pub struct ChunkColumn {
    pub chunks: Box<[Chunk; 16]>,
}

impl ChunkColumn {
    pub fn new() -> Self {
        Self {
            chunks: Box::new([
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
                Chunk::empty(),
            ])
        }
    }

    pub fn random() -> Self {
        Self {
            chunks: Box::new([
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
                Chunk::random(),
            ]),
        }
    }

    pub fn full_of_block(block: BlockID) -> Self {
        Self {
            chunks: Box::new([
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
                Chunk::full_of_block(block),
            ]),
        }
    }

    pub fn alternating() -> Self {
        Self {
            chunks: Box::new([
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
                Chunk::full_of_block(BlockID::Dirt),
                Chunk::full_of_block(BlockID::Cobblestone),
            ]),
        }
    }

    #[inline]
    pub fn get_chunk(&self, y: i32) -> &Chunk {
        &self.chunks[y as usize]
    }

    #[inline]
    pub fn set_block(&self, block: BlockID, x: u32, y: u32, z: u32) {
        self.chunks[(y / 16) as usize].set_block(block, x, y % 16, z);
    }
}

pub struct Chunk {
    pub is_rendered: RwLock<bool>,
    pub blocks: RwLock<[BlockID; CHUNK_VOLUME as usize]>,
    pub number_of_opaque_blocks: RwLock<u32>,
    pub number_of_transparent_blocks: RwLock<u32>,
    pub active_faces: RwLock<BitVec>,
    pub ao_vertices: RwLock<[[[u8; 4]; 6]; CHUNK_VOLUME as usize]>,

    pub vao: RwLock<u32>,
    pub vbo: RwLock<u32>,
    pub vertices_drawn: RwLock<u32>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::empty()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn reset(&self) {
        self.unload_from_gpu();
        *self.is_rendered.write() = false;
        *self.blocks.write() = [BlockID::Air; CHUNK_VOLUME as usize];
        *self.number_of_opaque_blocks.write() = 0;
        *self.number_of_transparent_blocks.write() = 0;
        *self.vertices_drawn.write() = 0;
    }

    /// Creates a chunk where every block is the same
    pub fn full_of_block(block: BlockID) -> Self {
        let (vao, vbo) = create_vao_vbo();

        let (opaque, transparent) = match block {
            BlockID::Air => (0, 0),
            block => if block.is_transparent() {
                (0, 16 * 16 * 16)
            } else {
                (16 * 16 * 16, 0)
            }
        };

        Self {
            is_rendered: RwLock::new(false),
            blocks: RwLock::new([block; CHUNK_VOLUME as usize]),
            number_of_opaque_blocks: RwLock::new(opaque),
            number_of_transparent_blocks: RwLock::new(transparent),
            active_faces: RwLock::new(BitVec::from_elem(6 * CHUNK_VOLUME as usize, false)),
            ao_vertices: RwLock::new([[[0; 4]; 6]; CHUNK_VOLUME as usize]),

            vao: RwLock::new(vao),
            vbo: RwLock::new(vbo),
            vertices_drawn: RwLock::new(0),
        }
    }

    /// Creates an empty chunk with no blocks
    pub fn empty() -> Self {
        Self::full_of_block(BlockID::Air)
    }

    /// Creates a chunk where every block is random
    pub fn random() -> Self {
        let (vao, vbo) = create_vao_vbo();

        Self {
            is_rendered: RwLock::new(false),
            blocks: RwLock::new({
                let mut blocks = [BlockID::Air; CHUNK_VOLUME as usize];
                for i in 0..blocks.len() {
                    blocks[i] = random::<BlockID>();
                }
                blocks
            }),
            number_of_opaque_blocks: RwLock::new(16 * 16 * 16),
            number_of_transparent_blocks: RwLock::new(0),
            active_faces: RwLock::new(BitVec::from_elem(6 * CHUNK_VOLUME as usize, false)),
            ao_vertices: RwLock::new([[[0; 4]; 6]; CHUNK_VOLUME as usize]),

            vao: RwLock::new(vao),
            vbo: RwLock::new(vbo),
            vertices_drawn: RwLock::new(0),
        }
    }

    pub fn is_fully_opaque(&self) -> bool {
        *self.number_of_opaque_blocks.read() == 16 * 16 * 16
    }

    pub fn is_empty(&self) -> bool {
        *self.number_of_opaque_blocks.read() + *self.number_of_transparent_blocks.read() == 0
    }

    #[inline]
    fn chunk_coords_to_array_index(x: u32, y: u32, z: u32) -> usize {
        (y * (CHUNK_SIZE * CHUNK_SIZE) + z * CHUNK_SIZE + x) as usize
    }

    #[inline]
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockID {
        self.blocks.read()[Chunk::chunk_coords_to_array_index(x, y, z)]
    }

    /// Sets a block at some given coordinates
    /// The coordinates must be within the chunk size
    #[inline]
    pub fn set_block(&self, block: BlockID, x: u32, y: u32, z: u32) {
        let index = Chunk::chunk_coords_to_array_index(x, y, z);

        let target = self.blocks.read()[index];
        if target.is_air() {
            if block.is_transparent_not_air() {
                *self.number_of_transparent_blocks.write() += 1;
            } else if block.is_opaque() {
                *self.number_of_opaque_blocks.write() += 1;
            }
        } else if target.is_transparent_not_air() {
            if block.is_air() {
                *self.number_of_transparent_blocks.write() -= 1;
            } else if block.is_opaque() {
                *self.number_of_transparent_blocks.write() -= 1;
                *self.number_of_opaque_blocks.write() += 1;
            }
        } else if target.is_opaque() {
            if block.is_air() {
                *self.number_of_opaque_blocks.write() -= 1;
            } else if block.is_transparent_not_air() {
                *self.number_of_transparent_blocks.write() += 1;
                *self.number_of_opaque_blocks.write() -= 1;
            }
        }

        self.blocks.write()[index] = block;
    }

    pub fn unload_from_gpu(&self) {
        gl_call!(gl::NamedBufferData(*self.vbo.read(),
                0,
                null(),
                gl::DYNAMIC_DRAW));
    }

    pub fn upload_to_gpu(&self, texture_pack: &TexturePack) {
        let n_visible_faces = self.active_faces.read().iter().fold(0, |acc, b| acc + b as i32);
        if n_visible_faces == 0 {
            return;
        }

        // Initialize the VBO
        gl_call!(gl::NamedBufferData(*self.vbo.read(),
                (6 * 10 * std::mem::size_of::<f32>() * n_visible_faces as usize) as isize,
                null(),
                gl::DYNAMIC_DRAW));

        // Map VBO to virtual memory
        let vbo_ptr: *mut f32 = gl_call!(gl::MapNamedBuffer(*self.vbo.read(), gl::WRITE_ONLY)) as *mut f32;
        let mut vbo_offset = 0;

        let mut vertices_drawn = 0;
        let sides_vec = &self.active_faces.read();
        let ao_vec = &self.ao_vertices.read();
        let mut j = 0;

        for (x, y, z) in BlockIterator::new() {
            let block = self.get_block(x, y, z);
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
                // gl_call!(gl::NamedBufferSubData(self.vbo, (i * std::mem::size_of::<f32>()) as isize, (cube_array.len() * std::mem::size_of::<f32>()) as isize, cube_array.as_ptr() as *mut c_void));
                vertices_drawn += copied_vertices;
                vbo_offset += copied_vertices as isize * 10; // 5 floats per vertex
            }
            j += 1;
        }
        *self.vertices_drawn.write() = vertices_drawn;
        gl_call!(gl::UnmapNamedBuffer(*self.vbo.read()));
    }
}

/// Iterator that iterates over all possible block coordinates of a chunk on all 3 axis
/// Equivalent in functionality to a triple for loop from 0 to 15 each
pub struct BlockIterator {
    x: u32,
    y: u32,
    z: u32
}

impl BlockIterator {
    pub fn new() -> BlockIterator {
        BlockIterator {
            x: 0,
            y: 0,
            z: 0
        }
    }
}

impl Iterator for BlockIterator {
    type Item = (u32, u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y == CHUNK_SIZE {
            None
        } else {
            let to_return = (self.x, self.y, self.z);
            self.x += 1;
            if self.x >= CHUNK_SIZE {
                self.x = 0;
                self.z += 1;
                if self.z >= CHUNK_SIZE {
                    self.z = 0;
                    self.y += 1;
                }
            }
            Some(to_return)
        }
    }
}
