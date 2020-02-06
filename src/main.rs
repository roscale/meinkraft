pub mod debugging;
pub mod draw_commands;
pub mod shader_compilation;
pub mod texture;

use crate::draw_commands::{Renderer2D, QuadProps};

extern crate glfw;

use glfw::{Action, Context, Key, WindowHint, OpenGlProfileHint};
use glfw::ffi::{glfwSwapInterval, glfwGetTime, glfwGetCursorPos};
use crate::shader_compilation::{ShaderPart, ShaderProgram};
use std::ffi::CString;
use std::os::raw::c_void;
use crate::debugging::debug_message_callback;
use glfw::WindowEvent::MouseButton;
use rand::Rng;
use crate::texture::create_texture;


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

    //    gl_call!(gl::Enable(gl::CULL_FACE));
//    gl_call!(gl::CullFace(gl::BACK));
//    gl_call!(gl::Enable(gl::DEPTH_TEST));
//    gl_call!(gl::Enable(gl::STENCIL_TEST));
    gl_call!(gl::Enable(gl::BLEND));
    gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));


    ////////
    let id_cobble = create_texture("blocks/cobblestone.png");
    let id_tnt = create_texture("blocks/tnt.png");

    let mut renderer = Renderer2D::default();

    let vert = ShaderPart::from_vert_source(
        &CString::new(include_str!("shaders/vert.vert")).unwrap()).unwrap();

    let frag = ShaderPart::from_frag_source(
        &CString::new(include_str!("shaders/frag.frag")).unwrap()).unwrap();

    let program = ShaderProgram::from_shaders(vert, frag).unwrap();

//    let triangle_mesh = [
//        0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0,
//        1.0, 0.0, 0.0, 1.0, 0.0, 1.0,
//        0.5, 1.0, 0.0, 0.0, 1.0, 1.0,
//        0.0f32 - 1.1, 0.0, 1.0, 0.0, 0.0, 1.0,
//        1.0 - 1.1, 0.0, 0.0, 1.0, 0.0, 1.0,
//        0.5 - 1.1, 1.0, 0.0, 0.0, 1.0, 1.0,
//    ];
//
//    let mut vbo = 0;
//    gl_call!(gl::CreateBuffers(1, &mut vbo));
//
//    gl_call!(gl::NamedBufferData(vbo,
//            (triangle_mesh.len() * std::mem::size_of::<f32>()) as isize,
//            triangle_mesh.as_ptr() as *mut c_void,
//            gl::DYNAMIC_DRAW));
//
//    let mut vao = 0;
//
//    let binding_index_pos = 0;
//    let binding_index_color = 1;
//    let pos_components = 2;
//    let color_components = 4;
//
//    gl_call!(gl::CreateVertexArrays(1, &mut vao));
//
//    gl_call!(gl::EnableVertexArrayAttrib(vao, 0));
//    gl_call!(gl::VertexArrayAttribFormat(vao, 0, 2, gl::FLOAT, gl::FALSE, 0));
//
//    gl_call!(gl::VertexArrayAttribBinding(vao, 0, binding_index_pos));
//    gl_call!(gl::VertexArrayVertexBuffer(vao, binding_index_pos, vbo, 0, (6 * std::mem::size_of::<f32>()) as i32));
//
//
//    gl_call!(gl::EnableVertexArrayAttrib(vao, 1));
//    gl_call!(gl::VertexArrayAttribFormat(vao, 1, 4, gl::FLOAT, gl::FALSE, (2 * std::mem::size_of::<f32>()) as u32));
//
//    gl_call!(gl::VertexArrayAttribBinding(vao, 1, binding_index_color));
//    gl_call!(gl::VertexArrayVertexBuffer(vao, binding_index_color, vbo, 0, (6 * std::mem::size_of::<f32>() as isize) as i32));
    ////////////


    gl_call!(gl::Viewport(0, 0, 500, 500));
    let mut framerate = PrintFramerate {
        prev: 0.0,
        frames: 0,
    };

    let mut quads: Vec<QuadProps> = Vec::new();

    let mut rng = rand::thread_rng();

    // Loop until the user closes the window
    while !window.should_close() {
        // Poll for and process events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            println!("{:?}", event);
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                glfw::WindowEvent::Key(Key::Space, _, _, _) => {
                    quads.push(QuadProps {
                        position: (
                            (window.get_cursor_pos().0 as f32).to_range(0.0, 500.0, -1.0, 1.0),
                            (window.get_cursor_pos().1 as f32).to_range(0.0, 500.0, 1.0, -1.0)),
                        size: (0.5, 0.5),
                        color: (
                            rng.gen_range(0.0, 1.0),
                            rng.gen_range(0.0, 1.0),
                            rng.gen_range(0.0, 1.0),
                            1.0,
                        ),
                    });
                }
                _ => {}
            }
        }

        gl_call!(gl::ClearColor(1.0, 1.0, 1.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT));

        program.use_program();

        renderer.begin_batch();

        for quad in &quads {
            renderer.submit_quad(quad.clone());
        }

        renderer.end_batch();

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
        let old_range = (old_max - old_min);
        let new_range = (new_max - new_min);
        (((self - old_min) * new_range) / old_range) + new_min
    }
}