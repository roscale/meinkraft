use std::time::Instant;

use nalgebra_glm::{Mat4, Vec3, vec3};

use crate::chunk_manager::ChunkManager;
use crate::physics::{Interpolatable, Interpolator};
use crate::shader_compilation::ShaderProgram;
use crate::shapes::quad;
use crate::timer::Timer;
use nalgebra::Matrix4;
use std::ffi::c_void;
use std::ops::Add;
use rand::random;
use crate::aabb::get_block_aabb;
use num_traits::Zero;

pub struct ParticleSystem {
    position: Vec3,
    particles: Vec<Interpolator<ParticlePhysicsProperties>>,
    vao: u32,
    vbo: u32,
}

impl ParticleSystem {
    pub fn new(position: Vec3) -> ParticleSystem {
        let mut vao = 0;
        gl_call!(gl::CreateVertexArrays(1, &mut vao));

        // Position
        gl_call!(gl::EnableVertexArrayAttrib(vao, 0));
        gl_call!(gl::VertexArrayAttribFormat(vao, 0, 3 as i32, gl::FLOAT, gl::FALSE, 0));
        gl_call!(gl::VertexArrayAttribBinding(vao, 0, 0));

        // Texture coords
        gl_call!(gl::EnableVertexArrayAttrib(vao, 1));
        gl_call!(gl::VertexArrayAttribFormat(vao, 1, 2 as i32, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as u32));
        gl_call!(gl::VertexArrayAttribBinding(vao, 1, 0));

        let quad = quad((0.0, 0.0, 0.0, 0.0));

        let mut vbo = 0;
        gl_call!(gl::CreateBuffers(1, &mut vbo));
        gl_call!(gl::NamedBufferData(vbo,
                    (quad.len() * std::mem::size_of::<f32>() as usize) as isize,
                    quad.as_ptr() as *const c_void,
                    gl::STATIC_DRAW));
        gl_call!(gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (5 * std::mem::size_of::<f32>()) as i32));

        let mut particles = Vec::new();

        for i in 0..20 {
            let x = (random::<f32>() - 0.5) * 0.8;
            let y = (random::<f32>() - 0.5) * 0.8;
            let z = (random::<f32>() - 0.5) * 0.8;

            let vx = (x) * 5.0;
            let vy = (y) * 20.0;
            let vz = (z) * 5.0;

            particles.push(Interpolator::new(1.0 / 30.0, ParticlePhysicsProperties {
                position: vec3(x, y, z) + position,
                velocity: vec3(vx, vy, vz),
                acceleration: vec3(0.0, -30.0, 0.0),
            }));
        }

        ParticleSystem {
            position,
            particles,
            vao,
            vbo,
        }
    }

    pub fn render_all_particles(&mut self, shader: &mut ShaderProgram, time: Instant, chunk_manager: &ChunkManager) {
        let mut states = Vec::new();
        for p in &mut self.particles {
            states.push(p.update_particle(time, chunk_manager));
        }

        for state in states {
            let model_matrix = {
                let translate_matrix = Matrix4::new_translation(&state.position);
                let rotate_matrix = Matrix4::from_euler_angles(
                    0.0f32,
                    0.0,
                    0.0,
                );
                let scale_matrix: Mat4 = Matrix4::new_nonuniform_scaling(&vec3(0.5f32, 0.5f32, 0.5f32));
                translate_matrix * rotate_matrix * scale_matrix
            };


            gl_call!(gl::BindVertexArray(self.vao));
            shader.set_uniform_matrix4fv("model", model_matrix.as_ptr());
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
        }
    }
}

#[derive(Clone)]
pub struct ParticlePhysicsProperties {
    pub position: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
}

impl Interpolatable for ParticlePhysicsProperties {
    fn interpolate(&self, alpha: f32, other: &Self) -> Self {
        let interpolate_vec3 = |from: &Vec3, to: &Vec3| {
            alpha * from + (1.0 - alpha) * to
        };

        ParticlePhysicsProperties {
            position: interpolate_vec3(&self.position, &other.position),
            velocity: interpolate_vec3(&self.velocity, &other.velocity),
            acceleration: interpolate_vec3(&self.acceleration, &other.acceleration),
        }
    }
}

impl Interpolator<ParticlePhysicsProperties> {
    fn update_particle(&mut self, time: Instant, chunk_manager: &ChunkManager) -> ParticlePhysicsProperties {
        self.step(time, &mut |state, _t, dt| {
            let mut state = state.clone();
            state.velocity += state.acceleration * dt;

            let vectors: &[Vec3] = &[
                vec3(state.velocity.x, 0., 0.),
                vec3(0., state.velocity.y, 0.),
                vec3(0., 0., state.velocity.z),
            ];

            for v in vectors {
                state.position += v * dt;

                let containing_block = vec3(
                    (state.position.x).floor() as i32,
                    (state.position.y).floor() as i32,
                    (state.position.z).floor() as i32,
                );

                let mut colliding_block_aabb = None;
                if let Some(block) = chunk_manager.get_block(containing_block.x, containing_block.y, containing_block.z) {
                    if !block.is_air() {
                        let block_aabb = get_block_aabb(&vec3(
                            containing_block.x as f32,
                            containing_block.y as f32,
                            containing_block.z as f32
                        ));
                        colliding_block_aabb = Some(block_aabb);
                    }
                }

                if colliding_block_aabb.is_none() {
                    continue;
                }
                let colliding_block_aabb = colliding_block_aabb.unwrap();

                let padding = 0.001;

                if !v.x.is_zero() {
                    if v.x < 0.0 {
                        state.position.x = colliding_block_aabb.maxs.x + padding;
                    } else {
                        state.position.x = colliding_block_aabb.mins.x - padding;
                    }
                    state.velocity.x *= -0.1;
                }
                if !v.y.is_zero() {
                    if v.y < 0.0 {
                        state.position.y = colliding_block_aabb.maxs.y + padding;
                    } else {
                        state.position.y = colliding_block_aabb.mins.y - padding;
                    }
                    state.velocity.y *= -0.1;
                }
                if !v.z.is_zero() {
                    if v.z < 0.0 {
                        state.position.z = colliding_block_aabb.maxs.z + padding;
                    } else {
                        state.position.z = colliding_block_aabb.mins.z - padding
                    }
                    state.velocity.z *= -0.1;
                }
            }

            state.velocity.x *= 0.8;
            state.velocity.z *= 0.8;

            state
        })
    }
}