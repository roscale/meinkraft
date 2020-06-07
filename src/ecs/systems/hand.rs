use nalgebra::{Matrix4, Vector3};
use nalgebra_glm::vec3;
use specs::{Entities, Join, Read, ReadStorage, Storage, System, Write, WriteStorage};

use crate::constants::{FAR_PLANE, NEAR_PLANE, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::ecs::components::MainHandItemChanged;
use crate::inventory::Inventory;
use crate::main_hand::MainHand;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};
use crate::types::{Shaders, TexturePack};
use crate::util::Forward;

pub struct UpdateMainHand;

impl<'a> System<'a> for UpdateMainHand {
    type SystemData = (
        WriteStorage<'a, MainHandItemChanged>,
        ReadStorage<'a, Inventory>,
        WriteStorage<'a, MainHand>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut main_hand_item_changed,
            inventory,
            mut main_hand,
        ) = data;

        for (_, inventory, main_hand) in (&main_hand_item_changed, &inventory, &mut main_hand).join() {
            main_hand.set_showing_item(inventory.get_selected_item());
        }

        main_hand_item_changed.clear();
    }
}

pub struct DrawMainHand;

impl<'a> System<'a> for DrawMainHand {
    type SystemData = (
        WriteStorage<'a, MainHand>,
        ReadStorage<'a, PlayerState>,
        ReadStorage<'a, Interpolator<PlayerPhysicsState>>,
        Read<'a, TexturePack>,
        Write<'a, Shaders>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut main_hand,
            player_state,
            player_physics_state,
            texture_pack,
            mut shaders,
        ) = data;

        for (player_state, player_physics_state, main_hand) in (&player_state, &player_physics_state, &mut main_hand).join() {
            let view_matrix = {
                let player_physics_state = player_physics_state.get_interpolated_state();
                let camera_position = player_physics_state.position + vec3(0., *player_state.camera_height.get_interpolated_state(), 0.);
                let looking_dir = player_state.rotation.forward();
                nalgebra_glm::look_at(&camera_position, &(camera_position + looking_dir), &Vector3::y())
            };

            main_hand.update_if_dirty(&texture_pack);

            let player_pos = player_physics_state.get_interpolated_state().position;
            let camera_height = *player_state.camera_height.get_interpolated_state();
            let camera_pos = player_pos + vec3(0., camera_height, 0.);

            let forward = &player_state.rotation.forward().normalize();
            let right = forward.cross(&Vector3::y()).normalize();
            let up = right.cross(&forward).normalize();

            let model_matrix = {
                let translate_matrix = Matrix4::new_translation(&(vec3(
                    camera_pos.x, camera_pos.y, camera_pos.z) + up * -1.2));

                let translate_matrix2 = Matrix4::new_translation(&(vec3(2.0, 0.0, 0.0)));

                let rotate_matrix = nalgebra_glm::rotation(-player_state.rotation.y, &vec3(0.0, 1.0, 0.0));
                let rotate_matrix = nalgebra_glm::rotation(player_state.rotation.x, &right) * rotate_matrix;

                let rotate_matrix = nalgebra_glm::rotation(-35.0f32.to_radians(), &up) * rotate_matrix;

                translate_matrix * rotate_matrix * translate_matrix2
            };

            let projection_matrix = {
                let fov = 70.0f32.to_radians();
                nalgebra_glm::perspective(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, fov, NEAR_PLANE, FAR_PLANE)
            };

            let hand_shader = shaders.get_mut("hand_shader").unwrap();
            hand_shader.use_program();
            hand_shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
            hand_shader.set_uniform_matrix4fv("view", view_matrix.as_ptr());
            // hand_shader.set_uniform_matrix4fv("model_view", model_view.as_ptr());
            hand_shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
            hand_shader.set_uniform1i("tex", 0);

            gl_call!(gl::BindVertexArray(main_hand.render.vbo));

            gl_call!(gl::Disable(gl::DEPTH_TEST));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 36 as i32));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
        }
    }
}