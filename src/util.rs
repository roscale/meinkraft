use nalgebra_glm::{Vec3, vec3};
use std::sync::Arc;
use std::ops::Deref;

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

pub trait Zero {
    fn zero() -> Self;
}

impl Zero for Vec3 {
    fn zero() -> Self {
        vec3(0., 0., 0.)
    }
}

// pub trait MappedArc<U: ?Sized, F> {
//     fn map(arc: Arc<F>, f: &dyn FnOnce(&F) -> &U) -> ArcGuard<U, F>;
// }
//
// impl<U: ?Sized, F> MappedArc<U, F> for Arc<F> {
//     fn map(arc: Arc<F>, f: &dyn FnOnce(&F) -> &U) -> ArcGuard<U, F> {
//         ArcGuard {
//             arc,
//             data: f(&arc)
//         }
//     }
// }
//
// pub struct ArcGuard<'a, U: ?Sized, F> {
//     arc: Arc<F>,
//     data: &'a U
// }
//
// impl<'a, U: ?Sized, F> Deref for ArcGuard<'a, U, F> {
//     type Target = U;
//
//     fn deref(&self) -> &Self::Target {
//         self.data
//     }
// }