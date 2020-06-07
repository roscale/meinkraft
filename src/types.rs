use std::collections::HashMap;
use crate::chunk::BlockID;
use crate::block_texture_faces::BlockFaces;
use crate::particle_system::ParticleSystem;

pub type TextureLayer = u32;
pub type UVFaces = (TextureLayer, TextureLayer, TextureLayer, TextureLayer, TextureLayer, TextureLayer);
pub type TexturePack = HashMap<BlockID, BlockFaces<TextureLayer>>;
pub type ParticleSystems = HashMap<&'static str, ParticleSystem>;