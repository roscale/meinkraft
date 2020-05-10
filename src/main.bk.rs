pub mod debugging;
pub mod draw_commands;
pub mod shader_compilation;
pub mod texture;
pub mod ecs;
pub mod lua;

use serde::{Serialize, Deserialize};


use crate::draw_commands::{Renderer2D, QuadProps};

extern crate glfw;

use glfw::{Action, Context, Key, WindowHint, OpenGlProfileHint};
use glfw::ffi::{glfwSwapInterval, glfwGetTime};
use crate::shader_compilation::{ShaderPart, ShaderProgram};
use std::ffi::CString;
use std::os::raw::c_void;
use crate::debugging::debug_message_callback;
use rand::Rng;
use crate::texture::create_texture;
use specs::{World, WorldExt, Builder, DispatcherBuilder};
use crate::ecs::components::*;
use crate::ecs::systems::*;
use rlua::{Table, ToLua};
use rlua::prelude::LuaTable;

#[derive(Default)]
pub struct PrintFramerate {
    prev: f64,
    frames: u32,
}

impl PrintFramerate {
    fn run(&mut self) {
        self.frames += 1;
        let now = unsafe { glfwGetTime() };
        let delta = now - self.prev;
        if delta >= 1.0 {
            self.prev = now;
            println!("Framerate: {}", f64::from(self.frames) / delta);
            self.frames = 0;
        }
    }
}


fn main() {
    // let lua = rlua::Lua::new();
    // lua.context(|lua| {
    //     let go = lua::structures::GameObject {
    //         id: 42,
    //         name: "Poate".to_string(),
    //     };
    //
    //     lua.globals().set("this", go).unwrap();
    //     lua.load(
    //         "
    //             print(this.id)
    //             print(this.name)
    //             print(this:haha())
    //         ").exec().unwrap();
    // });
    // return;

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(WindowHint::ContextVersionMajor(4));
    glfw.window_hint(WindowHint::ContextVersionMinor(6));
    glfw.window_hint(WindowHint::OpenGlProfile(OpenGlProfileHint::Core));
    glfw.window_hint(WindowHint::OpenGlDebugContext(true));
    let window_size = (500, 500);
    let window_title = "Batch renderer";

    // Create a windowed mode window and its OpenGL context
    let (mut window, events) = glfw.create_window(window_size.0, window_size.1, window_title, glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    // Make the window's context current
    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_raw_mouse_motion(true);

    gl::load_with(|s| window.get_proc_address(s) as *const _);
    unsafe { glfwSwapInterval(0) };

    gl_call!(gl::Enable(gl::DEBUG_OUTPUT));
    gl_call!(gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS));
    gl_call!(gl::DebugMessageCallback(Some(debug_message_callback), 0 as *const c_void));
    gl_call!(gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, 0 as *const u32, gl::TRUE));

    gl_call!(gl::Enable(gl::CULL_FACE));
    gl_call!(gl::CullFace(gl::BACK));
    gl_call!(gl::Enable(gl::DEPTH_TEST));
    gl_call!(gl::Enable(gl::BLEND));
    gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));

    ////////
    create_texture("blocks/cobblestone.png");
    create_texture("blocks/tnt.png");
    create_texture("blocks/diamond_block.png");
    create_texture("blocks/diamond_ore.png");
    create_texture("blocks/dirt.png");
    create_texture("blocks/emerald_ore.png");
    create_texture("blocks/glass.png");
    create_texture("blocks/glowstone.png");
    create_texture("blocks/gold_block.png");
    create_texture("blocks/gold_ore.png");

    let mut renderer = Renderer2D::default();

    let vert = ShaderPart::from_vert_source(
        &CString::new(include_str!("shaders/vert.vert")).unwrap()).unwrap();

    let frag = ShaderPart::from_frag_source(
        &CString::new(include_str!("shaders/frag.frag")).unwrap()).unwrap();

    let mut program = ShaderProgram::from_shaders(vert, frag).unwrap();

    gl_call!(gl::Viewport(0, 0, 500, 500));
    let mut framerate = PrintFramerate {
        prev: 0.0,
        frames: 0,
    };

    let mut quads: Vec<QuadProps> = Vec::new();

    let mut rng = rand::thread_rng();
    let mut i = 0.9;


    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.insert(program);

    // An entity may or may not contain some component.

    for i in 0..10_000 {
        world.create_entity()
            .with(Position(0.0, 0.0, 0.0))
            .with(Velocity(
                rng.gen_range(-1.0, 1.0),
                rng.gen_range(-1.0, 1.0)))
            .build();
    }

    // This builds a dispatcher.
    // The third parameter of `with` specifies
    // logical dependencies on other systems.
    // Since we only have one, we don't depend on anything.
    // See the `full` example for dependencies.
    let mut dispatcher = DispatcherBuilder::new()
        .with(Physics2D, "physics", &[])
        .with(Bounce, "bounce", &[])
        .with_thread_local(Render)
        .with_thread_local(ComputeDeltaTime)
        .build();
    // This will call the `setup` function of every system.
    // In this example this has no effect since we already registered our components.
    dispatcher.setup(&mut world);

    // Loop until the user closes the window
    while !window.should_close() {
        // Poll for and process events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                glfw::WindowEvent::Key(Key::Space, _, _, _) => {
                    quads.push(QuadProps {
                        position: (
                            (window.get_cursor_pos().0 as f32).to_range(0.0, 500.0, -1.0, 1.0),
                            (window.get_cursor_pos().1 as f32).to_range(0.0, 500.0, 1.0, -1.0),
                            i
                        ),
                        size: (0.5, 0.5),

                        texture_id: rng.gen_range(1u32, 11),
                        texture_coords: (0.0, 0.0, 1.0, 1.0),
                    });
                    // dbg!(i);
                    i -= 0.001;
                }
                _ => {}
            }
        }

        gl_call!(gl::ClearColor(1.0, 1.0, 1.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

        dispatcher.dispatch(&mut world);

//        renderer.begin_batch();
//
//        for quad in &quads {
//            renderer.submit_quad(quad.clone());
//        }
//
//        program.use_program();
//        renderer.end_batch(&mut program);

        // Swap front and back buffers
        window.swap_buffers();

        framerate.run();
    }
}

trait ToRange {
    fn to_range(&self, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32;
}

impl ToRange for f32 {
    fn to_range(&self, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
        let old_range = old_max - old_min;
        let new_range = new_max - new_min;
        (((self - old_min) * new_range) / old_range) + new_min
    }
}