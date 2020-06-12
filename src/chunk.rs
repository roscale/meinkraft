use std::collections::HashSet;
use rand::{random, Rng};
use rand::prelude::Distribution;
use rand::distributions::Standard;
use crate::chunk_manager::{CHUNK_VOLUME, CHUNK_SIZE};
use bit_vec::BitVec;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum BlockID {
    Air,
    Dirt,
    GrassBlock,
    Cobblestone,
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
    pub fn is_air(&self) -> bool {
        self == &BlockID::Air
    }
    pub fn is_transparent(&self) -> bool {
        match self {
            &BlockID::Air |
            &BlockID::OakLeaves |
            &BlockID::Glass => true,
            _ => false
        }
    }
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
    pub chunks: [Chunk; 16],
}

impl ChunkColumn {
    pub fn new() -> Self {
        Self {
            chunks: [
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
            ],
        }
    }

    pub fn random() -> Self {
        Self {
            chunks: [
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
            ],
        }
    }

    pub fn full_of_block() -> Self {
        Self {
            chunks: [
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
                Chunk::full_of_block(BlockID::Dirt),
            ],
        }
    }
}

pub struct Chunk {
    blocks: [BlockID; CHUNK_VOLUME as usize],
    visible_block_faces: BitVec,

    pub vao: u32,
    pub vbo: u32,
    pub vertices_drawn: u32,
    // When a chunk is dirty, its VBO needs to be recreated to match the blocks array
    pub dirty: bool,
    // Changes to the outer blocks of the chunk lead to dirty nearby chunks
    pub dirty_neighbours: HashSet<(i32, i32, i32)>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::empty()
    }
}

impl Chunk {
    /// Returns the relative coordinates of nearby chunks whether they exist or not
    fn all_neighbours() -> HashSet<(i32, i32, i32)> {
        let mut hs = HashSet::new();
        hs.insert((1, 0, 0));
        hs.insert((0, 1, 0));
        hs.insert((0, 0, 1));
        hs.insert((-1, 0, 0));
        hs.insert((0, -1, 0));
        hs.insert((0, 0, -1));
        hs
    }

    /// Creates an empty chunk with no blocks
    pub fn empty() -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        Chunk {
            blocks: [BlockID::Air; CHUNK_VOLUME as usize],
            visible_block_faces: BitVec::from_elem(6 * 16 * 16 * 16, false),
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        }
    }

    /// Creates a chunk where every block is the same
    pub fn full_of_block(block: BlockID) -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        Chunk {
            blocks: [block; CHUNK_VOLUME as usize],
            visible_block_faces: BitVec::from_elem(6 * 16 * 16 * 16, false),
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        }
    }

    /// Creates a chunk where every block is random
    pub fn random() -> Chunk {
        let (vao, vbo) = create_vao_vbo();

        Chunk {
            blocks: {
                let mut blocks = [BlockID::Air; CHUNK_VOLUME as usize];
                for i in 0..blocks.len() {
                    blocks[i] = random::<BlockID>();
                }
                blocks
            },
            visible_block_faces: BitVec::from_elem(6 * 16 * 16 * 16, false),
            vao,
            vbo,
            vertices_drawn: 0,
            dirty: true,
            dirty_neighbours: Chunk::all_neighbours(),
        }
    }

    #[inline]
    fn coords_to_index(x: u32, y: u32, z: u32) -> usize {
        (y * (CHUNK_SIZE * CHUNK_SIZE) + z * CHUNK_SIZE + x) as usize
    }

    #[inline]
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockID {
        self.blocks[Chunk::coords_to_index(x, y, z)]
    }

    /// Sets a block at some given coordinates
    /// The coordinates must be within the chunk size
    pub fn set_block(&mut self, block: BlockID, x: u32, y: u32, z: u32) {
        self.blocks[Chunk::coords_to_index(x, y, z)] = block;
        self.dirty = true;
        // The edges of the chunk
        if x == 0 {
            self.dirty_neighbours.insert((-1, 0, 0));
        } else if x == 15 {
            self.dirty_neighbours.insert((1, 0, 0));
        }
        if y == 0 {
            self.dirty_neighbours.insert((0, -1, 0));
        } else if y == 15 {
            self.dirty_neighbours.insert((0, 1, 0));
        }
        if z == 0 {
            self.dirty_neighbours.insert((0, 0, -1));
        } else if z == 15 {
            self.dirty_neighbours.insert((0, 0, 1));
        }
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
