#![feature(entry_insert)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate specs;

use core::ffi::c_void;
use std::collections::HashMap;
use std::time::Instant;

use glfw::{Action, Context, Key, MouseButton};
use nalgebra::{Matrix4, Vector3};
use nalgebra_glm::{IVec3, Mat4, Vec3};
use nalgebra_glm::vec3;
use specs::{Builder, DispatcherBuilder, World, WorldExt, RunNow};

use ecs::components::*;
use ecs::systems::*;
use timer::Timer;

use crate::aabb::{AABB, get_block_aabb};
use crate::chunk::BlockID;
use crate::chunk_manager::ChunkManager;
use crate::constants::*;
use crate::debugging::*;
use crate::fps_counter::FpsCounter;
use crate::gui::{create_block_outline_vao, create_crosshair_vao, create_gui_icons_texture, create_hotbar_selection_vao, create_hotbar_vao, create_widgets_texture, draw_crosshair};
use crate::input::InputCache;
use crate::inventory::Inventory;
use crate::main_hand::MainHand;
use crate::particle_system::ParticleSystem;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};
use crate::shader_compilation::ShaderProgram;
use crate::shapes::centered_unit_cube;
use crate::texture_pack::generate_array_texture;
use crate::types::{ParticleSystems, TexturePack, Shaders};
use crate::util::Forward;
use crate::window::create_window;
use specs::shred::RunWithPool;

#[macro_use]
pub mod debugging;
pub mod draw_commands;
pub mod shader_compilation;
pub mod shapes;
pub mod util;
pub mod chunk_manager;
pub mod chunk;
pub mod raycast;
pub mod block_texture_faces;
pub mod physics;
pub mod aabb;
pub mod constants;
pub mod input;
pub mod window;
pub mod texture_pack;
pub mod player;
pub mod types;
pub mod gui;
pub mod inventory;
pub mod ambient_occlusion;
pub mod timer;
pub mod particle_system;
pub mod ecs;
pub mod main_hand;

fn main() {
    pretty_env_logger::init();

    let mut world = World::new();
    world.register::<PlayerState>();
    world.register::<Interpolator<PlayerPhysicsState>>();
    world.register::<Inventory>();
    world.register::<MainHand>();
    world.register::<MainHandItemChanged>();

    let mut dispatcher = DispatcherBuilder::new()
        .with_thread_local({
            let (mut glfw, mut window, events) = create_window(WINDOW_WIDTH, WINDOW_HEIGHT, WINDOW_NAME);

            gl_call!(gl::Enable(gl::DEBUG_OUTPUT));
            gl_call!(gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS));
            gl_call!(gl::DebugMessageCallback(Some(debug_message_callback), 0 as *const c_void));
            gl_call!(gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, 0 as *const u32, gl::TRUE));
            gl_call!(gl::Enable(gl::CULL_FACE));
            gl_call!(gl::CullFace(gl::BACK));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::Viewport(0, 0, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32));

            ReadWindowEvents {
                glfw,
                window,
                events,
            }
        })
        .with_thread_local(InventoryHandleInput)
        .with_thread_local(HandlePlayerInput)
        .with_thread_local(UpdatePlayerPhysics)
        .with_thread_local(UpdatePlayerState)
        .with_thread_local(PlaceAndBreakBlocks)
        .with_thread_local(UpdateMainHand)
        .with_thread_local(AdvanceGlobalTime)
        .with_thread_local(FpsCounter::new())
        .build();


    world.insert(InputCache::default());
    world.insert(Timer::default());
    world.insert({
        let (item_array_texture, texture_pack) = generate_array_texture();
        gl_call!(gl::BindTextureUnit(0, item_array_texture));
        texture_pack
    });
    world.insert({
        let mut chunk_manager = ChunkManager::new();
        chunk_manager.generate_terrain();
        chunk_manager
    });
    world.insert({
        let mut particle_systems: HashMap<&str, ParticleSystem> = HashMap::new();
        particle_systems.insert("block_particles", ParticleSystem::new(500));
        particle_systems
    });
    world.insert({
        let mut shaders_resource = Shaders::new();
        shaders_resource.insert("voxel_shader", ShaderProgram::compile("src/shaders/voxel.vert", "src/shaders/voxel.frag"));
        shaders_resource.insert("gui_shader", ShaderProgram::compile("src/shaders/gui.vert", "src/shaders/gui.frag"));
        shaders_resource.insert("outline_shader", ShaderProgram::compile("src/shaders/outline.vert", "src/shaders/outline.frag"));
        shaders_resource.insert("item_shader", ShaderProgram::compile("src/shaders/item.vert", "src/shaders/item.frag"));
        shaders_resource.insert("particle_shader", ShaderProgram::compile("src/shaders/particle.vert", "src/shaders/particle.frag"));
        shaders_resource.insert("hand_shader", ShaderProgram::compile("src/shaders/hand.vert", "src/shaders/hand.frag"));
        shaders_resource
    });

    {
        let gui_icons_texture = create_gui_icons_texture();
        gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 1));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, gui_icons_texture));

        let gui_widgets_texture = create_widgets_texture();
        gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 2));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, gui_widgets_texture));
    }

    let crosshair_vao = create_crosshair_vao();
    let block_outline_vao = create_block_outline_vao();
    let hotbar_vao = create_hotbar_vao();
    let hotbar_selection_vao = create_hotbar_selection_vao();

    let player = world.create_entity()
        .with(PlayerState::new())
        .with(Interpolator::new(
            1.0 / PHYSICS_TICKRATE,
            PlayerPhysicsState::new_at_position(vec3(0.0f32, 30.0, 0.0)),
        ))
        .with(Inventory::new())
        .with(MainHand::new())
        .with(MainHandItemChanged)
        .build();

    let mut draw_main_hand = DrawMainHand;

    loop {
        dispatcher.dispatch(&world);

        let mut player_states = world.write_component::<PlayerState>();
        let mut player_physics_states = world.write_component::<Interpolator<PlayerPhysicsState>>();

        let mut player_state = player_states.get_mut(player).unwrap();
        let mut player_physics_state = player_physics_states.get_mut(player).unwrap();

        let mut chunk_manager = world.fetch_mut::<ChunkManager>();
        let mut particle_systems = world.fetch_mut::<ParticleSystems>();
        let mut shaders = world.fetch_mut::<Shaders>();

        let global_timer = world.fetch::<Timer>();
        let texture_pack = world.fetch::<TexturePack>();

        let view_matrix = {
            let player_physics_state = player_physics_state.get_interpolated_state();
            let camera_position = player_physics_state.position + vec3(0., *player_state.camera_height.get_interpolated_state(), 0.);
            let looking_dir = player_state.rotation.forward();
            nalgebra_glm::look_at(&camera_position, &(camera_position + looking_dir), &Vector3::y())
        };

        let projection_matrix = {
            let fov = *player_state.fov.get_interpolated_state();
            nalgebra_glm::perspective(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, fov, NEAR_PLANE, FAR_PLANE)
        };

        // Draw chunks
        {
            let texture_pack = world.fetch::<TexturePack>();
            chunk_manager.rebuild_dirty_chunks(&texture_pack);

            let mut voxel_shader = shaders.get_mut("voxel_shader").unwrap();
            voxel_shader.use_program();
            voxel_shader.set_uniform_matrix4fv("view", view_matrix.as_ptr());
            voxel_shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
            voxel_shader.set_uniform1i("array_texture", 0);

            let (r, g, b, a) = BACKGROUND_COLOR;
            gl_call!(gl::ClearColor(r, g, b, a));
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));

            chunk_manager.render_loaded_chunks(&mut voxel_shader);
        }

        // Draw particles
        {
            gl_call!(gl::Disable(gl::CULL_FACE));
            let mut particle_shader = shaders.get_mut("particle_shader").unwrap();
            particle_shader.use_program();
            // particle_shader.set_uniform_matrix4fv("view", view_matrix.as_ptr());
            // particle_shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
            particle_shader.set_uniform1i("array_texture", 0);

            for particle_system in particle_systems.values_mut() {
                particle_system.update_all_particles(global_timer.time(), &chunk_manager);
                particle_system.render_all_particles(&mut particle_shader, &view_matrix, &projection_matrix);
            }
            gl_call!(gl::Enable(gl::CULL_FACE));
        }

        // Block outline
        if let Some(((x, y, z), _)) = player_state.targeted_block {
            let (x, y, z) = (x as f32, y as f32, z as f32);
            let model_matrix = Matrix4::new_translation(&vec3(x, y, z));

            let mut outline_shader = shaders.get_mut("outline_shader").unwrap();
            outline_shader.use_program();
            outline_shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
            outline_shader.set_uniform_matrix4fv("view", view_matrix.as_ptr());
            outline_shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());

            gl_call!(gl::LineWidth(BLOCK_OUTLINE_WIDTH));
            gl_call!(gl::BindVertexArray(block_outline_vao));
            gl_call!(gl::DrawArrays(gl::LINES, 0, 24));
        }

        // Draw hand
        // {
        //     let mut main_hand = world.write_component::<MainHand>();
        //     let mut main_hand = main_hand.get_mut(player).unwrap();
        //
        //     // main_hand.set_showing_item(player_state.inventory.get_selected_item());
        //     main_hand.update_if_dirty(&texture_pack);
        //
        //     let player_pos = player_physics_state.get_interpolated_state().position;
        //     let camera_height = *player_state.camera_height.get_interpolated_state();
        //     let camera_pos = player_pos + vec3(0., camera_height, 0.);
        //
        //     let forward = &player_state.rotation.forward().normalize();
        //     let right = forward.cross(&Vector3::y()).normalize();
        //     let up = right.cross(&forward).normalize();
        //
        //     let model_matrix = {
        //         let translate_matrix = Matrix4::new_translation(&(vec3(
        //             camera_pos.x, camera_pos.y, camera_pos.z) + up * -1.2));
        //
        //         let translate_matrix2 = Matrix4::new_translation(&(vec3(2.0, 0.0, 0.0)));
        //
        //         let rotate_matrix = nalgebra_glm::rotation(-player_state.rotation.y, &vec3(0.0, 1.0, 0.0));
        //         let rotate_matrix = nalgebra_glm::rotation(player_state.rotation.x, &right) * rotate_matrix;
        //
        //         let rotate_matrix = nalgebra_glm::rotation(-35.0f32.to_radians(), &up) * rotate_matrix;
        //
        //         translate_matrix * rotate_matrix * translate_matrix2
        //     };
        //
        //     let projection_matrix = {
        //         let fov = 1.22173;
        //         nalgebra_glm::perspective(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, fov, NEAR_PLANE, FAR_PLANE)
        //     };
        //
        //     hand_shader.use_program();
        //     hand_shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
        //     hand_shader.set_uniform_matrix4fv("view", view_matrix.as_ptr());
        //     // hand_shader.set_uniform_matrix4fv("model_view", model_view.as_ptr());
        //     hand_shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        //     hand_shader.set_uniform1i("tex", 0);
        //
        //     gl_call!(gl::BindVertexArray(main_hand.render.vbo));
        //
        //     gl_call!(gl::Disable(gl::DEPTH_TEST));
        //     gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 36 as i32));
        //     gl_call!(gl::Enable(gl::DEPTH_TEST));
        // }

        // Draw GUI
        {
            let mut inventory = world.write_component::<Inventory>();
            let mut inventory = inventory.get_mut(player).unwrap();

            let mut gui_shader = shaders.get_mut("gui_shader").unwrap();
            draw_crosshair(crosshair_vao, &mut gui_shader);
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
            inventory.update_dirty_items(&texture_pack);
            inventory.draw_hotbar(hotbar_vao, &mut gui_shader);
            inventory.draw_hotbar_selection_box(hotbar_selection_vao, &mut gui_shader);

            let mut item_shader = shaders.get_mut("item_shader").unwrap();
            inventory.draw_hotbar_items(&mut item_shader);
            gl_call!(gl::Enable(gl::DEPTH_TEST));
        }

        drop(player_states);
        drop(player_physics_states);
        drop(chunk_manager);
        drop(particle_systems);
        drop(shaders);
        drop(global_timer);
        drop(texture_pack);

        RunNow::run_now(&mut draw_main_hand, &world);
    }
}
