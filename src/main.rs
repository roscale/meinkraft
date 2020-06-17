#![feature(entry_insert)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate specs;

use core::ffi::c_void;
use std::collections::HashMap;

use nalgebra_glm::vec3;
use specs::{Builder, DispatcherBuilder, World, WorldExt};

use ecs::components::*;
use ecs::systems::*;
use timer::Timer;

use crate::chunk_manager::ChunkManager;
use crate::constants::*;
use crate::debugging::*;
use crate::fps_counter::FpsCounter;
use crate::gui::{create_gui_icons_texture, create_widgets_texture};
use crate::input::InputCache;
use crate::inventory::Inventory;
use crate::main_hand::MainHand;
use crate::particle_system::ParticleSystem;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};
use crate::shader_compilation::ShaderProgram;
use crate::texture_pack::generate_array_texture;
use crate::types::Shaders;
use crate::window::create_window;
use crate::ecs::systems::chunk_loading::ChunkLoading;

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
            let (glfw, window, events) = create_window(WINDOW_WIDTH, WINDOW_HEIGHT, WINDOW_NAME);

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
        .with_thread_local(ChunkLoading::new())

        .with_thread_local(RenderChunks)
        .with_thread_local(RenderParticles)
        .with_thread_local(RenderBlockOutline::new())
        .with_thread_local(RenderMainHand::new())
        .with_thread_local(RenderGUI::new())

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
    world.insert(ChunkManager::new());

    {
        let gui_icons_texture = create_gui_icons_texture();
        gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 1));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, gui_icons_texture));

        let gui_widgets_texture = create_widgets_texture();
        gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 2));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, gui_widgets_texture));
    }

    let _player = world.create_entity()
        .with(PlayerState::new())
        .with(Interpolator::new(
            1.0 / PHYSICS_TICKRATE,
            PlayerPhysicsState::new_at_position(vec3(0.0f32, 200.0, 0.0)),
        ))
        .with(Inventory::new())
        .with(MainHand::new())
        .with(MainHandItemChanged)
        .build();

    loop {
        dispatcher.dispatch(&world);
    }
}
