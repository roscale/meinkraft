use specs::Component;
use specs::DenseVecStorage;
use specs::NullStorage;

use crate::inventory::Inventory;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};

impl Component for Interpolator<PlayerPhysicsState> {
    type Storage = DenseVecStorage<Self>;
}

impl Component for PlayerState {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct MainHandItemChanged;

impl Component for MainHandItemChanged {
    type Storage = NullStorage<Self>;
}

impl Component for Inventory {
    type Storage = DenseVecStorage<Self>;
}