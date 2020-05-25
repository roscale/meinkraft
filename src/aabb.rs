use nalgebra_glm::{Vec3, vec3};

/// Axis Aligned Bounding Box
/// A 3-dimensional box where all the faces are parallel to the axis
/// mins: the minimal corner of the box
/// maxs: the opposite corner
#[derive(Debug, Copy, Clone)]
pub struct AABB {
    pub mins: Vec3,
    pub maxs: Vec3,
}

impl AABB {
    pub fn new(mins: Vec3, maxs: Vec3) -> AABB {
        AABB { mins, maxs }
    }

    pub fn ip_translate(&mut self, translation: &Vec3) {
        self.mins += translation;
        self.maxs += translation;
    }

    /// Checks whether this AABB is intersecting another one
    pub fn intersects(&self, other: &AABB) -> bool {
        (self.mins.x < other.maxs.x && self.maxs.x > other.mins.x) &&
            (self.mins.y < other.maxs.y && self.maxs.y > other.mins.y) &&
            (self.mins.z < other.maxs.z && self.maxs.z > other.mins.z)
    }

    pub fn contains_point(&self, other: &Vec3) -> bool {
        (self.mins.x < other.x && self.maxs.x > other.x) &&
            (self.mins.y < other.y && self.maxs.y > other.y) &&
            (self.mins.z < other.z && self.maxs.z > other.z)
    }
}

/// Creates an AABB box at mins with a length of 1 in every dimension
pub fn get_block_aabb(mins: &Vec3) -> AABB {
    AABB::new(
        mins.clone(),
        mins + vec3(1.0, 1.0, 1.0))
}