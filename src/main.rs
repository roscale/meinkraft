use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;

use glfw::{Action, Context, Cursor, CursorMode, Key, MouseButton, OpenGlProfileHint, WindowHint};
use glfw::ffi::glfwSwapInterval;
use glfw::MouseButton::Button1;
use glfw::WindowEvent::Pos;
use image::{DynamicImage, GenericImageView};
use nalgebra::{clamp, Matrix4, Vector3, Point3};
use nalgebra_glm::{IVec3, Mat4, pi, proj, Vec2, vec2, Vec3, vec3};
use rand::random;

use crate::block_texture_faces::BlockFaces;
use crate::chunk::{BlockID, Chunk};
use crate::chunk_manager::ChunkManager;
use crate::debugging::*;
use crate::shader_compilation::{ShaderPart, ShaderProgram};
use crate::util::forward;
use crate::collisions::player_collision_detection;
use crate::aabb::AABB;

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
pub mod collisions;
pub mod aabb;

type UVCoords = (f32, f32, f32, f32);
type UVFaces = (UVCoords, UVCoords, UVCoords, UVCoords, UVCoords, UVCoords);

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

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_EYES_HEIGHT: f32 = 1.6;
const PLAYER_HALF_WIDTH: f32 = PLAYER_WIDTH / 2.0;
const PLAYER_HALF_HEIGHT: f32 = PLAYER_HEIGHT / 2.0;

pub struct Player {
    pub position: Vec3,
    pub aabb: AABB,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub rotation: Vec3,
}

impl Player {
    pub fn new_at_position(position: Vec3) -> Player {
        Player {
            position,
            aabb: {
                let mins = vec3(position.x - PLAYER_HALF_WIDTH, position.y, position.z - PLAYER_HALF_WIDTH);
                let maxs = vec3(position.x + PLAYER_HALF_WIDTH, position.y + PLAYER_HEIGHT, position.z + PLAYER_HALF_WIDTH);
                AABB::new(mins, maxs)
            },
            velocity: vec3(0.0, 0.0, 0.0),
            acceleration: vec3(0.0, 0.0, 0.0),
            rotation: vec3(0.0, 0.0, 0.0) // In radians
        }
    }

    // pub fn get_aabb(&self) -> AABB<f32> {
    //     AABB::from_half_extents(
    //         (self.position + vec3(0.0, PLAYER_HALF_HEIGHT, 0.0)).into(),
    //         vec3(PLAYER_HALF_WIDTH, PLAYER_HALF_HEIGHT, PLAYER_HALF_WIDTH))
    // }

    pub fn get_camera_rotation(&mut self) -> &mut Vec3 {
        &mut self.rotation
    }

    pub fn get_camera_position(&self) -> Vec3 {
        self.position + vec3(0.0, PLAYER_EYES_HEIGHT, 0.0)
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


    let mut player = Player::new_at_position(vec3(0.0f32, 30.0, 0.0));


    let vert = ShaderPart::from_vert_source(
        &CString::new(include_str!("shaders/diffuse.vert")).unwrap()).unwrap();
    let frag = ShaderPart::from_frag_source(
        &CString::new(include_str!("shaders/diffuse.frag")).unwrap()).unwrap();
    let mut program = ShaderProgram::from_shaders(vert, frag).unwrap();


    let mut texture_map: HashMap<BlockID, BlockFaces<&str>> = HashMap::new();
    texture_map.insert(BlockID::Dirt, BlockFaces::All("blocks/dirt.png"));
    texture_map.insert(BlockID::GrassBlock, BlockFaces::Sides {
        sides: "blocks/grass_block_side.png",
        top: "blocks/grass_block_top.png",
        bottom: "blocks/dirt.png",
    });
    texture_map.insert(BlockID::Cobblestone, BlockFaces::All("blocks/cobblestone.png"));
    texture_map.insert(BlockID::Obsidian, BlockFaces::All("blocks/obsidian.png"));
    texture_map.insert(BlockID::OakLog, BlockFaces::Sides {
        sides: "blocks/oak_log.png",
        top: "blocks/oak_log_top.png",
        bottom: "blocks/oak_log_top.png",
    });
    texture_map.insert(BlockID::OakLeaves, BlockFaces::All("blocks/oak_leaves_mod.png"));
    texture_map.insert(BlockID::Urss, BlockFaces::All("blocks/urss.png"));
    texture_map.insert(BlockID::Hitler, BlockFaces::All("blocks/hitler.png"));
    texture_map.insert(BlockID::Debug, BlockFaces::All("blocks/debug.png"));
    texture_map.insert(BlockID::Debug2, BlockFaces::All("blocks/debug2.png"));

    let mut uv_map = HashMap::<BlockID, BlockFaces<UVCoords>>::new();

    // Generate texture atlas
    let mut atlas: u32 = 0;
    gl_call!(gl::CreateTextures(gl::TEXTURE_2D, 1, &mut atlas));
    gl_call!(gl::TextureParameteri(atlas, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_NEAREST as i32));
    gl_call!(gl::TextureParameteri(atlas, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
    gl_call!(gl::TextureStorage2D(atlas, 1, gl::RGBA8, 1024, 1024)); // Big enough to contain all the textures

    let load_image = |texture_path: &str| {
        let img = image::open(texture_path);
        let img = match img {
            Ok(img) => img.flipv(),
            Err(err) => panic!("Filename: {}, error: {}", texture_path, err.to_string())
        };
        match img.color() {
            image::RGBA(8) => {}
            _ => panic!("Texture format not supported")
        };
        img
    };

    let mut x = 0;
    let mut y = 0;
    let mut blit_image_to_texture = |img: &mut DynamicImage| {
        let (dest_x, dest_y) = (x, y);
        gl_call!(gl::TextureSubImage2D(
            atlas, 0,
            dest_x, dest_y, img.width() as i32, img.height() as i32,
            gl::RGBA, gl::UNSIGNED_BYTE,
            img.raw_pixels().as_ptr() as *mut c_void));

        // Left to right, bottom to top
        x += 16;
        if x >= 1024 {
            x = 0;
            y += 16;
        }

        let dest_x = dest_x as f32;
        let dest_y = dest_y as f32;
        // Coordinates must be between 0.0 and 1.0 (percentage)
        (dest_x / 1024.0, dest_y / 1024.0, (dest_x + 16.0) / 1024.0, (dest_y + 16.0) / 1024.0)
    };

    // Load all the images and fill the UV map for all the blocks
    // TODO don't load the same texture multiple times if reused for another block
    for (block, faces) in texture_map {
        match faces {
            BlockFaces::All(all) => {
                uv_map.insert(block, BlockFaces::All(blit_image_to_texture(&mut load_image(all))));
            }
            BlockFaces::Sides { sides, top, bottom } => {
                uv_map.insert(block, BlockFaces::Sides {
                    sides: blit_image_to_texture(&mut load_image(sides)),
                    top: blit_image_to_texture(&mut load_image(top)),
                    bottom: blit_image_to_texture(&mut load_image(bottom)),
                });
            }
            BlockFaces::Each { top, bottom, front, back, left, right } => {
                uv_map.insert(block, BlockFaces::Each {
                    top: blit_image_to_texture(&mut load_image(top)),
                    bottom: blit_image_to_texture(&mut load_image(bottom)),
                    front: blit_image_to_texture(&mut load_image(front)),
                    back: blit_image_to_texture(&mut load_image(back)),
                    left: blit_image_to_texture(&mut load_image(left)),
                    right: blit_image_to_texture(&mut load_image(right)),
                });
            }
        }
    }

    gl_call!(gl::ActiveTexture(gl::TEXTURE0 + 0));
    gl_call!(gl::BindTexture(gl::TEXTURE_2D, atlas));

    let mut chunk_manager = ChunkManager::new();
    // chunk_manager.preload_some_chunks();
    chunk_manager.simplex_noise();
    // chunk_manager.empty_99();
    // chunk_manager.set(BlockID::COBBLESTONE, 1, 1, 1);
    // chunk_manager.set(BlockID::COBBLESTONE, -16, -16, -16);

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
                    player.rotation.y += rel_x as f32 / 100.0;
                    player.rotation.x -= rel_y as f32 / 100.0;

                    player.rotation.x = clamp(player.rotation.x, -pi::<f32>() / 2.0 + 0.0001, pi::<f32>() / 2.0 - 0.0001);

                    past_cursor_pos = (x, y);
                }

                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }

                glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                    player.velocity.y = 0.05;
                }

                glfw::WindowEvent::Key(key, _, action, _) => {
                    input_cache.key_states.insert(key, action);
                }

                glfw::WindowEvent::MouseButton(button, Action::Press, _) => {
                    let reach_distance = 400.0;

                    let fw = forward(&player.rotation);
                    let get_voxel = |x: i32, y: i32, z: i32| {
                        chunk_manager.get_block(x, y, z)
                            .filter(|&block| block != BlockID::Air)
                            .and_then(|_| Some((x, y, z)))
                    };

                    let hit = raycast::raycast(&get_voxel, &player.get_camera_position(), &fw.normalize(), reach_distance);
                    if let Some(((x, y, z), normal)) = hit {
                        if button == MouseButton::Button1 {
                            chunk_manager.set_block(BlockID::Air, x, y, z);
                        } else if button == MouseButton::Button2 {
                            let near = IVec3::new(x, y, z) + normal;

                            // TODO implement Hotbar
                            chunk_manager.set_block(BlockID::Debug2, near.x, near.y, near.z);
                            println!("Put block at {} {} {}", near.x, near.y, near.z);
                        }

                        println!("HIT {} {} {}", x, y, z);
                    } else {
                        println!("NO HIT");
                    }
                }

                _ => {}
            }
        }

        // TODO use deltatime
        let multiplier = 0.001f32;

        let mut rotation = player.rotation.clone();
        rotation.x = 0.0;

        if input_cache.is_key_pressed(Key::W) {
            player.acceleration += forward(&rotation).scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::S) {
            player.acceleration += -forward(&rotation).scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::A) {
            player.acceleration += -forward(&rotation).cross(&Vector3::y()).normalize().scale(multiplier);
        }

        if input_cache.is_key_pressed(Key::D) {
            player.acceleration += forward(&rotation).cross(&Vector3::y()).normalize().scale(multiplier);
        }

        // if input_cache.is_key_pressed(Key::Q) {
        //     player.velocity.y += multiplier;
        // }
        //
        // if input_cache.is_key_pressed(Key::Z) {
        //     player.velocity.y -= multiplier;
        // }

        let direction = forward(&player.rotation);

        let camera_position = player.get_camera_position();
        let view_matrix = nalgebra_glm::look_at(&camera_position, &(camera_position + direction), &Vector3::y());
        let projection_matrix = nalgebra_glm::perspective(1.0, pi::<f32>() / 2.0, 0.1, 1000.0);

        chunk_manager.rebuild_dirty_chunks(&uv_map);


        program.use_program();
        program.set_uniform_matrix4fv("view", view_matrix.as_ptr());
        program.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
        program.set_uniform1i("tex", 0); // The texture atlas

        gl_call!(gl::ClearColor(0.74, 0.84, 1.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

        chunk_manager.render_loaded_chunks(&mut program);
        player.acceleration.y = -0.0007;
        // player.velocity.x *= 0.01;
        // player.velocity.z *= 0.01;
        player_collision_detection(&mut player, &chunk_manager);

        player.velocity.x *= 0.96;
        player.velocity.z *= 0.96;
        player.acceleration.x = 0.0;
        player.acceleration.y = 0.0;
        player.acceleration.z = 0.0;
        window.swap_buffers();
    }
}