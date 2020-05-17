use nalgebra_glm::{Vec3, vec3};

pub trait Forward {
    fn forward(&self) -> Self;
}

impl Forward for Vec3 {
    fn forward(&self) -> Self {
        vec3(
            self.x.cos() * self.y.cos(),
            self.x.sin(),
            self.x.cos() * self.y.sin(),
        )
    }
}
