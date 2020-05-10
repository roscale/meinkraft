use nalgebra_glm::{Vec3, vec3};

pub fn forward(rotation: &Vec3) -> Vec3 {
    vec3(
        rotation.x.cos() * rotation.y.cos(),
        rotation.x.sin(),
        rotation.x.cos() * rotation.y.sin(),
    )
}