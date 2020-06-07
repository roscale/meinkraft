use specs::{System, WriteStorage, ReadStorage, Join, Entities, Storage, Read, Entity};
use crate::main_hand::MainHand;
use crate::ecs::components::MainHandItemChanged;
use crate::player::PlayerState;
use crate::input::InputCache;
use crate::inventory::Inventory;
use glfw::WindowEvent;

pub struct InventoryHandleInput;

impl InventoryHandleInput {
    fn select_item(inventory: &mut Inventory, index: usize, f: &mut dyn FnMut()) {
        if inventory.selected_hotbar_slot != index {
            inventory.select_item(index);
            f();
        }
    }
}

impl<'a> System<'a> for InventoryHandleInput {
    type SystemData = (
        Entities<'a>,
        Read<'a, InputCache>,
        WriteStorage<'a, Inventory>,
        WriteStorage<'a, MainHandItemChanged>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            input_cache,
            mut inventory,
            mut main_hand_item_changed,
        ) = data;

        for (e, inventory) in (&entities, &mut inventory).join() {
            let mut f = || {
                main_hand_item_changed.insert(e, MainHandItemChanged);
            };

            for event in &input_cache.events {
                use glfw::{Key, Action};
                match event {

                    WindowEvent::Scroll(_, y) => {
                        if y.is_sign_positive() {
                            inventory.select_previous_item();
                        } else {
                            inventory.select_next_item();
                        }
                        f();
                    }
                    WindowEvent::Key(Key::Num1, _, Action::Press, _) => Self::select_item(inventory, 0, &mut f),
                    WindowEvent::Key(Key::Num2, _, Action::Press, _) => Self::select_item(inventory, 1, &mut f),
                    WindowEvent::Key(Key::Num3, _, Action::Press, _) => Self::select_item(inventory, 2, &mut f),
                    WindowEvent::Key(Key::Num4, _, Action::Press, _) => Self::select_item(inventory, 3, &mut f),
                    WindowEvent::Key(Key::Num5, _, Action::Press, _) => Self::select_item(inventory, 4, &mut f),
                    WindowEvent::Key(Key::Num6, _, Action::Press, _) => Self::select_item(inventory, 5, &mut f),
                    WindowEvent::Key(Key::Num7, _, Action::Press, _) => Self::select_item(inventory, 6, &mut f),
                    WindowEvent::Key(Key::Num8, _, Action::Press, _) => Self::select_item(inventory, 7, &mut f),
                    WindowEvent::Key(Key::Num9, _, Action::Press, _) => Self::select_item(inventory, 8, &mut f),
                    _ => {}
                }
            }
        }
    }
}
