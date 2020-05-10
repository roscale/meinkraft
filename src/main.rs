#[macro_use]
pub mod debugging;
pub mod draw_commands;
pub mod shader_compilation;
pub mod texture;
pub mod ecs;
pub mod shapes;
pub mod util;
pub mod chunk;

use glfw::{WindowHint, OpenGlProfileHint, Context, Key, Action, CursorMode, Cursor};
use glfw::ffi::glfwSwapInterval;
use std::os::raw::c_void;
use crate::debugging::*;
use std::ffi::CString;
use crate::shader_compilation::{ShaderPart, ShaderProgram};
use crate::ecs::components::Position;
use glfw::WindowEvent::Pos;
use nalgebra_glm::{Vec3, vec3, Mat4, Vec2, vec2, pi};
use nalgebra::{Vector3, Matrix4, clamp};
use crate::texture::create_texture;
use crate::shapes::unit_cube_array;
use std::collections::HashMap;
use crate::util::forward;

pub struct InputCache {
    pub last_cursor_pos: Vec2,
    pub cursor_rel_pos: Vec2,

    pub key_states: HashMap<Key, Action>,
}

impl Default for InputCache {
    fn default() -> Self {
        InputCache {
            last_cursor_pos: vec2(0.0, 0.0),
            cursor_rel_pos: vec2(0.0, 0.0),
            key_states: HashMap::default(),
        }
    }
}

impl InputCache {
    pub fn is_key_pressed(&self, key: Key) -> bool {
        match self.key_states.get(&key) {
            None => false,
            Some(action) => *action == Action::Press || *action == Action::Repeat
        }
    }
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(WindowHint::ContextVersionMajor(4));
    glfw.window_hint(WindowHint::ContextVersionMinor(6));
    glfw.window_hint(WindowHint::OpenGlProfile(OpenGlProfileHint::Core));
    glfw.window_hint(WindowHint::OpenGlDebugContext(true));

    let window_size = (800, 800);
    let window_title = "Meinkraft";

    let (mut window, events) = glfw.create_window(window_size.0, window_size.1, window_title, glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    // Make the window's context current
    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_raw_mouse_motion(true);
    window.set_cursor_mode(CursorMode::Disabled);
    window.set_cursor_pos(400.0, 400.0);

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

    gl_call!(gl::Viewport(0, 0, 800, 800));


    let mut camera_position = vec3(0.0f32, 0.0, 0.0);
    let mut camera_rotation = vec3(0.0f32, 0.0, 0.0);


    let vert = ShaderPart::from_vert_source(
        &CString::new(include_str!("shaders/diffuse.vert")).unwrap()).unwrap();
    let frag = ShaderPart::from_frag_source(
        &CString::new(include_str!("shaders/diffuse.frag")).unwrap()).unwrap();
    let mut program = ShaderProgram::from_shaders(vert, frag).unwrap();

    let cobblestone = create_texture("blocks/cobblestone.png");
    gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 0));
    gl_call!(gl::BindTexture(gl::TEXTURE_2D, cobblestone));

    let cube = unit_cube_array();

    let mut cube_vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut cube_vbo));
    gl_call!(gl::NamedBufferData(cube_vbo,
            (cube.len() * std::mem::size_of::<f32>()) as isize,
            cube.as_ptr() as *mut c_void,
            gl::STATIC_DRAW));

    let mut cube_vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut cube_vao));

    gl_call!(gl::EnableVertexArrayAttrib(cube_vao, 0));
    gl_call!(gl::EnableVertexArrayAttrib(cube_vao, 1));

    gl_call!(gl::VertexArrayAttribFormat(cube_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribFormat(cube_vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));

    gl_call!(gl::VertexArrayAttribBinding(cube_vao, 0, 0));
    gl_call!(gl::VertexArrayAttribBinding(cube_vao, 1, 0));



    gl_call!(gl::VertexArrayVertexBuffer(cube_vao, 0, cube_vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));

    gl_call!(gl::BindVertexArray(cube_vao));






    let mut input_cache = InputCache::default();
    let mut past_cursor_pos = (0.0, 0.0);
    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::CursorPos(x, y) => {
                    let rel_x = x - past_cursor_pos.0;
                    let rel_y = y - past_cursor_pos.1;

                    // dbg!(rel_x, rel_y);
                    camera_rotation.y += rel_x as f32 / 100.0;
                    camera_rotation.x -= rel_y as f32 / 100.0;

                    camera_rotation.x = clamp(camera_rotation.x, -pi::<f32>() / 2.0 + 0.0001, pi::<f32>() / 2.0 - 0.0001);

                    past_cursor_pos = (x, y);
                }

                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                glfw::WindowEvent::Key(key, _, action, _) => {
                    input_cache.key_states.insert(key, action);
                }
                _ => {}
            }
        }

        if input_cache.is_key_pressed(Key::W) {
            camera_position += forward(&camera_rotation).scale(0.03f32);
        }

        if input_cache.is_key_pressed(Key::S) {
            camera_position -= forward(&camera_rotation).scale(0.03f32);
        }

        if input_cache.is_key_pressed(Key::A) {
            camera_position -= forward(&camera_rotation).cross(&Vector3::y()).normalize().scale(0.03f32);
        }

        if input_cache.is_key_pressed(Key::D) {
            camera_position += forward(&camera_rotation).cross(&Vector3::y()).normalize().scale(0.03f32);
        }

        if input_cache.is_key_pressed(Key::Q) {
            camera_position.y += 0.03;
            println!("up");
        }

        if input_cache.is_key_pressed(Key::Z) {
            camera_position.y -= 0.03;
        }

        dbg!(camera_position);
        dbg!(camera_rotation);
        let direction = forward(&camera_rotation);

        let view_matrix = nalgebra_glm::look_at(&camera_position, &(camera_position + direction), &Vector3::y());
        let projection_matrix = nalgebra_glm::perspective(1.0, pi::<f32>() / 2.0, 0.1, 1000.0);

        let model_matrix = {
            let translate_matrix = Matrix4::new_translation(&vec3(5.0f32, 0.0, 0.0));

            let rotate_matrix = Matrix4::from_euler_angles(
                0.0f32,
                0.0,
                0.0,
            );

            let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(1.0f32, 1.0f32, 1.0f32));
            translate_matrix * rotate_matrix * scale_matrix
        };




        program.use_program();
        program.set_uniform_matrix4fv("model", model_matrix.as_ptr());
        program.set_uniform_matrix4fv("view", view_matrix.as_ptr());
        program.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        program.set_uniform1i("tex", 0);




        gl_call!(gl::ClearColor(0.74, 0.84, 1.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

        gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 36));

        window.swap_buffers();
    }
}