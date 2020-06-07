use glfw::WindowEvent;
use nalgebra::Matrix4;
use nalgebra_glm::{Mat4, vec3};

use crate::chunk::BlockID;
use crate::constants::{GUI_SCALING, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::inventory::item::ItemStack;
use crate::shader_compilation::ShaderProgram;
use crate::types::TexturePack;
use specs::WriteStorage;
use crate::ecs::components::MainHandItemChanged;

pub mod item;

const INVENTORY_SIZE: usize = 36;
const HOTBAR_SIZE: usize = 9;

pub struct Inventory {
    pub slots: [Option<ItemStack>; INVENTORY_SIZE],
    pub selected_hotbar_slot: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory {
            slots: {
                let mut slots = [None; INVENTORY_SIZE];
                slots[0] = Some(ItemStack::new(1, BlockID::Dirt));
                slots[1] = Some(ItemStack::new(1, BlockID::GrassBlock));
                slots[2] = Some(ItemStack::new(1, BlockID::Cobblestone));
                slots[3] = Some(ItemStack::new(1, BlockID::OakLog));
                slots[4] = Some(ItemStack::new(1, BlockID::OakPlanks));
                slots[5] = Some(ItemStack::new(1, BlockID::OakLeaves));
                slots[6] = Some(ItemStack::new(1, BlockID::Glass));
                slots[7] = Some(ItemStack::new(1, BlockID::Urss));
                slots[8] = Some(ItemStack::new(1, BlockID::Hitler));
                slots
            },
            selected_hotbar_slot: 0,
        }
    }

    pub fn get_selected_item(&self) -> Option<BlockID> {
        self.slots[self.selected_hotbar_slot].map(|item_stack| item_stack.item)
    }

    pub fn select_item(&mut self, index: usize) {
        self.selected_hotbar_slot = index;
    }

    pub fn select_next_item(&mut self) {
        self.selected_hotbar_slot += 1;
        if self.selected_hotbar_slot >= HOTBAR_SIZE {
            self.selected_hotbar_slot = 0;
        }
    }

    pub fn select_previous_item(&mut self) {
        if self.selected_hotbar_slot == 0 {
            self.selected_hotbar_slot = HOTBAR_SIZE - 1;
        } else {
            self.selected_hotbar_slot -= 1;
        }
    }

    pub fn update_dirty_items(&mut self, texture_pack: &TexturePack) {
        for item_stack in self.slots.iter_mut() {
            if let Some(item_stack) = item_stack {
                item_stack.update_if_dirty(&texture_pack);
            }
        }
    }

    pub fn draw_hotbar(&self, vao: u32, shader: &mut ShaderProgram) {
        let model_matrix = {
            let translate_matrix = Matrix4::new_translation(&vec3(
                WINDOW_WIDTH as f32 / 2.0, 11.0 * GUI_SCALING, 0.0));
            let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(182.0 * GUI_SCALING, 22.0 * GUI_SCALING, 1.0));
            translate_matrix * scale_matrix
        };
        let projection_matrix = nalgebra_glm::ortho(
            0.0, WINDOW_WIDTH as f32, 0.0, WINDOW_HEIGHT as f32, -5.0, 5.0);

        shader.use_program();
        shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
        shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        shader.set_uniform1i("tex", 2);

        gl_call!(gl::BindVertexArray(vao));
        gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
    }

    pub fn draw_hotbar_selection_box(&self, vao: u32, shader: &mut ShaderProgram) {
        let interslot_spacing = 20.0;
        let hotbar_left_margin = WINDOW_WIDTH as f32 / 2.0 - 4.0 * interslot_spacing * GUI_SCALING;
        let selection_box_x_pos = hotbar_left_margin + interslot_spacing * self.selected_hotbar_slot as f32 * GUI_SCALING;

        let model_matrix = {
            let translate_matrix = Matrix4::new_translation(&vec3(selection_box_x_pos, 11.0 * GUI_SCALING, 0.0));
            let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(24.0 * GUI_SCALING, 24.0 * GUI_SCALING, 1.0));
            translate_matrix * scale_matrix
        };
        let projection_matrix = nalgebra_glm::ortho(
            0.0, WINDOW_WIDTH as f32, 0.0, WINDOW_HEIGHT as f32, -5.0, 5.0);

        shader.use_program();
        shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
        shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        shader.set_uniform1i("tex", 2);

        gl_call!(gl::BindVertexArray(vao));
        gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
    }

    pub fn draw_hotbar_items(&self, shader: &mut ShaderProgram) {
        let interslot_spacing = 20.0;
        let hotbar_left_margin = WINDOW_WIDTH as f32 / 2.0 - 4.0 * interslot_spacing * GUI_SCALING;

        let mut x = 0;
        let y = 11;
        for slot in self.slots.iter() {
            if let Some(slot) = slot {
                let item_x_pos = hotbar_left_margin + (x as f32) * interslot_spacing * GUI_SCALING;
                slot.item_render.draw(item_x_pos, (y as f32) * GUI_SCALING, shader);
            }
            x += 1;
        }
    }
}