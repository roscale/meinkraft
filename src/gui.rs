use crate::shapes::quad;
use std::ffi::c_void;
use crate::shader_compilation::ShaderProgram;
use nalgebra_glm::{vec3, Mat4};
use crate::constants::{WINDOW_WIDTH, WINDOW_HEIGHT, CROSSHAIR_SIZE};
use image::GenericImageView;
use nalgebra::Matrix4;

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
            WINDOW_WIDTH as f32 / 2.0, WINDOW_WIDTH as f32 / 2.0, 0.0));
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