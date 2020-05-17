use crate::types::{UVCoords, UVFaces};

#[derive(Copy, Clone)]
pub enum BlockFaces<T> {
    All(T),
    Sides { sides: T, top: T, bottom: T },
    Each { top: T, bottom: T, front: T, back: T, left: T, right: T },
}

/// Unpacks a BlockFaces<UVCoords> instance and returns a tuple of UV coordinates
/// for each face of the block
pub fn get_uv_of_every_face(faces: BlockFaces<UVCoords>) -> UVFaces {
    match faces {
        BlockFaces::All(uv) => (uv, uv, uv, uv, uv, uv),
        BlockFaces::Sides { sides, top, bottom } =>
            (sides, sides, top, bottom, sides, sides),
        BlockFaces::Each { top, bottom, front, back, left, right } =>
            (front, back, top, bottom, left, right)
    }
}