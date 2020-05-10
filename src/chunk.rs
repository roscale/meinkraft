#[macro_use]
use crate::debugging::*;
use std::ptr::null;

#[derive(Copy, Clone)]
pub enum BlockID {
    AIR,
    COBBLESTONE
}

const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
const CUBE_SIZE: usize = 180;

fn create_vao_vbo() -> (u32, u32) {
    let mut vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(vao, 1, 0));

    let mut vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut vbo));
    gl_call!(gl::NamedBufferData(vbo,
            (180 * std::mem::size_of::<f32>() * CHUNK_VOLUME) as isize,
            null(),
            gl::DYNAMIC_DRAW));

    gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));
}

pub struct Chunk {
    blocks: [BlockID; CHUNK_VOLUME],
    vao: u32,
    vbo: u32,
    vbo_data: [f32; CHUNK_VOLUME],
    vertices_drawn: u32
}

impl Chunk {
    pub fn empty() -> Chunk {
        Chunk {
            blocks: [BlockID::AIR; CHUNK_VOLUME],
            vertices_drawn: 0,
        }
    }

    pub fn full_of_block(block: BlockID) -> Chunk {
        Chunk {
            blocks: [BlockID::COBBLESTONE; CHUNK_VOLUME]
        }
    }

    #[inline]
    fn coords_to_index(x: usize, y: usize, z: usize) -> usize {
        y * (CHUNK_SIZE * CHUNK_SIZE) + z * CHUNK_SIZE + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockID {
        self.blocks[Chunk::coords_to_index(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, block: BlockID, x: usize, y: usize, z: usize) {
        self.blocks[Chunk::coords_to_index(x, y, z)] = block
    }

    pub fn regen_vbo(&mut self) {
        let mut i = 0;
        for y in ..CHUNK_SIZE {
            for z in ..CHUNK_SIZE {
                for x in ..CHUNK_SIZE {
                    let block = self.get(x, y, z);
                    if block != BlockID::AIR {

                    }
                    i += 1;
                }
            }
        }
    }
}