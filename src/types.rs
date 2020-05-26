use std::collections::HashMap;
use crate::chunk::BlockID;
use crate::block_texture_faces::BlockFaces;

pub type TextureLayer = u32;
pub type UVFaces = (TextureLayer, TextureLayer, TextureLayer, TextureLayer, TextureLayer, TextureLayer);
pub type UVMap = HashMap<BlockID, BlockFaces<TextureLayer>>;