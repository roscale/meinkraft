use std::ffi::c_void;

use image::GenericImageView;
use nalgebra::Matrix4;
use nalgebra_glm::{Mat4, vec3};

use crate::constants::{CROSSHAIR_SIZE, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::shader_compilation::ShaderProgram;
use crate::shapes::block_outline;
use crate::shapes::quad;

pub fn create_gui_icons_texture() -> u32 {
    let gui_icons_image = match image::open("textures/gui/icons.png") {
        Ok(img) => img,
        Err(err) => panic!("Filename: {}, error: {}", "textures/gui/icons.png", err.to_string())
    };
    match gui_icons_image.color() {
        image::RGBA(8) => {}
        _ => panic!("Texture format not supported")
    };

    // Upload the image to the GPU
    let mut gui_icons_texture = 0;
    gl_call!(gl::CreateTextures(gl::TEXTURE_2D, 1, &mut gui_icons_texture));
    gl_call!(gl::TextureParameteri(gui_icons_texture, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_NEAREST as i32));
    gl_call!(gl::TextureParameteri(gui_icons_texture, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
    gl_call!(gl::TextureStorage2D(gui_icons_texture, 1, gl::RGBA8, gui_icons_image.width() as i32, gui_icons_image.height() as i32));
    gl_call!(gl::TextureSubImage2D(
            gui_icons_texture, 0,
            0, 0, gui_icons_image.width() as i32, gui_icons_image.height() as i32,
            gl::RGBA, gl::UNSIGNED_BYTE,
            gui_icons_image.raw_pixels().as_ptr() as *mut c_void));
    gui_icons_texture
}

pub fn create_crosshair_vao() -> u32 {
    let mut gui_vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut gui_vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(gui_vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(gui_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(gui_vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(gui_vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(gui_vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(gui_vao, 1, 0));

    let mut gui_vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut gui_vbo));

    gl_call!(gl::VertexArrayVertexBuffer(gui_vao, 0, gui_vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));
    gl_call!(gl::NamedBufferData(gui_vbo,
                    (30 * std::mem::size_of::<f32>() as usize) as isize,
                    quad((0.0, 0.0, 15.0 / 256.0, 15.0 / 256.0)).as_ptr() as *const c_void,
                    gl::STATIC_DRAW));
    gui_vao
}

pub fn draw_crosshair(vao: u32, shader: &mut ShaderProgram) {
    let model_matrix = {
        let translate_matrix = Matrix4::new_translation(&vec3(
            WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0, 0.0));
        let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(CROSSHAIR_SIZE, CROSSHAIR_SIZE, 1.0));
        translate_matrix * scale_matrix
    };
    let projection_matrix = nalgebra_glm::ortho(
        0.0, WINDOW_WIDTH as f32, 0.0, WINDOW_HEIGHT as f32, -5.0, 5.0);

    shader.use_program();
    shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
    shader.set_uniform_matrix4fv("projection", projection_matrix.as_ptr());
    shader.set_uniform1i("tex", 1);

    gl_call!(gl::BlendFunc(gl::ONE_MINUS_DST_COLOR, gl::ZERO));
    gl_call!(gl::BindVertexArray(vao));
    gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
}

pub fn create_block_outline_vao() -> u32 {
    let mut outline_vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut outline_vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(outline_vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(outline_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(outline_vao, 0, 0));

    let mut outline_vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut outline_vbo));

    gl_call!(gl::VertexArrayVertexBuffer(outline_vao, 0, outline_vbo, 0, (3 * std::mem::size_of::<f32>()) as i32));
    gl_call!(gl::NamedBufferData(outline_vbo,
                    (72 * std::mem::size_of::<f32>() as usize) as isize,
                    block_outline().as_ptr() as *const c_void,
                    gl::STATIC_DRAW));
    outline_vao
}

pub fn create_widgets_texture() -> u32 {
    let widgets_image = match image::open("textures/gui/widgets.png") {
        Ok(img) => img,
        Err(err) => panic!("Filename: {}, error: {}", "textures/gui/widgets.png", err.to_string())
    };
    match widgets_image.color() {
        image::RGBA(8) => {}
        _ => panic!("Texture format not supported")
    };

    // quad((0.0, 0.0, 0.0, 0.0)).chunks(3).take(2).collect::<Vec2>();

    // Upload the image to the GPU
    let mut widgets_texture = 0;
    gl_call!(gl::CreateTextures(gl::TEXTURE_2D, 1, &mut widgets_texture));
    gl_call!(gl::TextureParameteri(widgets_texture, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_NEAREST as i32));
    gl_call!(gl::TextureParameteri(widgets_texture, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
    gl_call!(gl::TextureStorage2D(widgets_texture, 1, gl::RGBA8, widgets_image.width() as i32, widgets_image.height() as i32));
    gl_call!(gl::TextureSubImage2D(
            widgets_texture, 0,
            0, 0, widgets_image.width() as i32, widgets_image.height() as i32,
            gl::RGBA, gl::UNSIGNED_BYTE,
            widgets_image.raw_pixels().as_ptr() as *mut c_void));
    widgets_texture
}

pub fn create_hotbar_vao() -> u32 {
    let mut hotbar_vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut hotbar_vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(hotbar_vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(hotbar_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(hotbar_vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(hotbar_vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(hotbar_vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(hotbar_vao, 1, 0));

    let mut hotbar_vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut hotbar_vbo));

    gl_call!(gl::VertexArrayVertexBuffer(hotbar_vao, 0, hotbar_vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));
    gl_call!(gl::NamedBufferData(hotbar_vbo,
                    (30 * std::mem::size_of::<f32>() as usize) as isize,
                    quad((0.0, 0.0, 182.0 / 256.0, 22.0 / 256.0)).as_ptr() as *const c_void,
                    // quad((0.0, 0.0, 1.0, 1.0)).as_ptr() as *const c_void,
                    gl::STATIC_DRAW));
    hotbar_vao
}

pub fn create_hotbar_selection_vao() -> u32 {
    let mut hotbar_selection_vao = 0;
    gl_call!(gl::CreateVertexArrays(1, &mut hotbar_selection_vao));

    // Position
    gl_call!(gl::EnableVertexArrayAttrib(hotbar_selection_vao, 0));
    gl_call!(gl::VertexArrayAttribFormat(hotbar_selection_vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
    gl_call!(gl::VertexArrayAttribBinding(hotbar_selection_vao, 0, 0));

    // Texture coords
    gl_call!(gl::EnableVertexArrayAttrib(hotbar_selection_vao, 1));
    gl_call!(gl::VertexArrayAttribFormat(hotbar_selection_vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
    gl_call!(gl::VertexArrayAttribBinding(hotbar_selection_vao, 1, 0));

    let mut hotbar_selection_vbo = 0;
    gl_call!(gl::CreateBuffers(1, &mut hotbar_selection_vbo));

    gl_call!(gl::VertexArrayVertexBuffer(hotbar_selection_vao, 0, hotbar_selection_vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));
    gl_call!(gl::NamedBufferData(hotbar_selection_vbo,
                    (30 * std::mem::size_of::<f32>() as usize) as isize,
                    quad((0.0, 22.0 / 256.0, 24.0 / 256.0, 46.0 / 256.0)).as_ptr() as *const c_void,
                    // quad((0.0, 0.0, 1.0, 1.0)).as_ptr() as *const c_void,
                    gl::STATIC_DRAW));
    hotbar_selection_vao
}