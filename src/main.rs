#[macro_use]
pub mod debugging;
pub mod draw_commands;
pub mod shader_compilation;
pub mod texture;
pub mod ecs;
pub mod shapes;
pub mod util;
pub mod chunk_manager;
pub mod chunk;
pub mod raycast;

use glfw::{WindowHint, OpenGlProfileHint, Context, Key, Action, CursorMode, Cursor, MouseButton};
use glfw::ffi::glfwSwapInterval;
use std::os::raw::c_void;
use crate::debugging::*;
use std::ffi::CString;
use crate::shader_compilation::{ShaderPart, ShaderProgram};
use crate::ecs::components::Position;
use glfw::WindowEvent::Pos;
use nalgebra_glm::{Vec3, vec3, Mat4, Vec2, vec2, pi, IVec3, proj};
use nalgebra::{Vector3, Matrix4, clamp};
use crate::texture::create_texture;
use crate::shapes::unit_cube_array;
use std::collections::HashMap;
use crate::util::forward;
use crate::chunk::{Chunk, BlockID};
use rand::random;
use image::GenericImageView;
use glfw::MouseButton::Button1;
use crate::chunk_manager::ChunkManager;

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
    window.set_mouse_button_polling(true);
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



    // Generate texture atlas
    let mut texture_map: HashMap<BlockID, &str> = HashMap::new();
    texture_map.insert(BlockID::DIRT, "blocks/dirt.png");
    texture_map.insert(BlockID::COBBLESTONE, "blocks/cobblestone.png");
    texture_map.insert(BlockID::OBSIDIAN, "blocks/obsidian.png");

    let mut uv_map = HashMap::<BlockID, ((f32, f32), (f32, f32))>::new();

    let mut atlas: u32 = 0;
    gl_call!(gl::CreateTextures(gl::TEXTURE_2D, 1, &mut atlas));
    gl_call!(gl::TextureParameteri(atlas, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_NEAREST as i32));
    gl_call!(gl::TextureParameteri(atlas, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
    gl_call!(gl::TextureStorage2D(atlas, 1, gl::RGBA8, 1024, 1024));

    let mut x = 0;
    let mut y = 0;

    for (block, texture_path) in texture_map {
        let img = image::open(texture_path);
        let img = match img {
            Ok(img) => img.flipv(),
            Err(err) => panic!("Filename: {}, error: {}", texture_path, err.to_string())
        };

        match img.color() {
            image::RGBA(8) => {},
            _ => panic!("Texture format not supported")
        };

        gl_call!(gl::TextureSubImage2D(
            atlas, 0,
            x, y, img.width() as i32, img.height() as i32,
            gl::RGBA, gl::UNSIGNED_BYTE,
            img.raw_pixels().as_ptr() as *mut c_void));

        uv_map.insert(block, ((x as f32 / 1024.0, y as f32 / 1024.0),
                              ((x as f32 + 16.0) / 1024.0, (y as f32 + 16.0) / 1024.0)));

        x += 16;
        if x >= 1024 {
            x = 0;
            y += 16;
        }
    }

    gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 0));
    gl_call!(gl::BindTexture(gl::TEXTURE_2D, atlas));

    // let cube = unit_cube_array(0.0, 0.0, 0.0);
    //
    // let mut cube_vbo = 0;
    // gl_call!(gl::CreateBuffers(1, &mut cube_vbo));
    // gl_call!(gl::NamedBufferData(cube_vbo,
    //         (cube.len() * std::mem::size_of::<f32>()) as isize,
    //         cube.as_ptr() as *mut c_void,
    //         gl::STATIC_DRAW));
    //
    // let mut cube_vao = 0;
    // gl_call!(gl::CreateVertexArrays(1, &mut cube_vao));
    //
    // gl_call!(gl::EnableVertexArrayAttrib(cube_vao, 0));
    // gl_call!(gl::EnableVertexArrayAttrib(cube_vao, 1));
    //
    // gl_call!(gl::VertexArrayAttribFormat(cube_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    // gl_call!(gl::VertexArrayAttribFormat(cube_vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    //
    // gl_call!(gl::VertexArrayAttribBinding(cube_vao, 0, 0));
    // gl_call!(gl::VertexArrayAttribBinding(cube_vao, 1, 0));



    // gl_call!(gl::VertexArrayVertexBuffer(cube_vao, 0, cube_vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));

    // let mut c = Chunk::empty();
    // c.set(BlockID::COBBLESTONE, 0, 0, 0);
    // c.set(BlockID::COBBLESTONE, 1, 0, 0);
    // c.regen_vbo();
    // let mut c = Chunk::empty();
    //
    // for y in 0..4 {
    //     for x in 0..16 {
    //         for z in 0..16 {
    //             c.set(BlockID::COBBLESTONE, x, y, z);
    //         }
    //     }
    // }

    let mut chunk_manager = ChunkManager::new();
    chunk_manager.preload_some_chunks();
    // chunk_manager.empty_99();
    // chunk_manager.set(BlockID::COBBLESTONE, 1, 1, 1);
    // chunk_manager.set(BlockID::COBBLESTONE, -16, -16, -16);

    // c.regen_vbo(&uv_map);

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
                // glfw::WindowEvent::Key(Key::R, _, Action::Press, _) => {
                //     for _ in 0..16*16*8 {
                //         c.set(BlockID::AIR,
                //               (random::<u8>() % 16) as usize,
                //               (random::<u8>() % 16) as usize,
                //               (random::<u8>() % 16) as usize);
                //     }
                //
                //     c.regen_vbo(&uv_map);
                //     // exit(0);
                //
                // }
                glfw::WindowEvent::Key(key, _, action, _) => {
                    input_cache.key_states.insert(key, action);
                }

                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    let fw = forward(&camera_rotation);
                    let get_voxel = |x: i32, y: i32, z: i32| {
                        chunk_manager.get(x, y, z)
                            .filter(|&block| block != BlockID::AIR)
                            .and_then(|_| Some((x, y, z)))
                    };

                    let hit = raycast::raycast(&get_voxel, &camera_position, &fw.normalize(), 400.0);
                    if let Some(((x, y, z), normal)) = hit {
                        if button == MouseButton::Button1 {
                            chunk_manager.set(BlockID::AIR, x, y, z);
                        } else if button == MouseButton::Button2 {
                            let near = IVec3::new(x, y, z) + normal;
                            chunk_manager.set(BlockID::DIRT, near.x, near.y, near.z);
                        }

                        println!("HIT {} {} {}", x, y, z);
                        // dbg!(fw);
                    } else {
                        println!("NO HIT");
                    }
                }

                _ => {}
            }
        }

        let multiplier = 0.1f32;

        if input_cache.is_key_pressed(Key::W) {
            camera_position += forward(&camera_rotation).scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::S) {
            camera_position -= forward(&camera_rotation).scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::A) {
            camera_position -= forward(&camera_rotation).cross(&Vector3::y()).normalize().scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::D) {
            camera_position += forward(&camera_rotation).cross(&Vector3::y()).normalize().scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::Q) {
            camera_position.y += multiplier;
        }

        if input_cache.is_key_pressed(Key::Z) {
            camera_position.y -= multiplier;
        }

        // dbg!(camera_position);
        // dbg!(camera_rotation);
        let direction = forward(&camera_rotation);

        let view_matrix = nalgebra_glm::look_at(&camera_position, &(camera_position + direction), &Vector3::y());
        let projection_matrix = nalgebra_glm::perspective(1.0, pi::<f32>() / 2.0, 0.1, 1000.0);

        chunk_manager.rebuild_dirty_chunks(&uv_map);



        program.use_program();
        program.set_uniform_matrix4fv("view", view_matrix.as_ptr());
        program.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        program.set_uniform1i("tex", 0);

        gl_call!(gl::ClearColor(0.74, 0.84, 1.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

        chunk_manager.render_loaded_chunks(&mut program);

        window.swap_buffers();
    }
}