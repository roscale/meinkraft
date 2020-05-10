use specs::prelude::*;
use super::resources::*;
use super::components::*;
use crate::draw_commands::{Renderer2D, QuadProps};
use crate::shader_compilation::ShaderProgram;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use glfw::ffi::glfwGetTime;
use std::process::exit;

pub struct Physics2D;

impl<'a> System<'a> for Physics2D {
    type SystemData = (
        Read<'a, DeltaTime>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Velocity>);

    fn run(&mut self, (dt, mut pos, vel): Self::SystemData) {
        let dt: f32 = dt.delta.as_micros() as f32 / 1_000_000.0;
//        exit(0);
        for (pos, vel) in (&mut pos, &vel).join() {
            pos.0 += vel.0 * dt;
            pos.1 += vel.1 * dt;
        }
    }
}

pub struct Render;

impl<'a> System<'a> for Render {
    type SystemData = (
        Write<'a, Renderer2D>,
        WriteExpect<'a, ShaderProgram>,
        ReadStorage<'a, Position>);

    fn run(&mut self, (mut renderer, mut shader, pos): Self::SystemData) {
        renderer.begin_batch();
        for pos in (&pos, ).join() {
            let pos = &*pos.0;
            let tuple = (pos.0, pos.1, pos.2);

            renderer.submit_quad(QuadProps {
                position: tuple,
                size: (0.5, 0.5),
                texture_id: 1,
                texture_coords: (0.0, 0.0, 1.0, 1.0),
            });
        }
        renderer.end_batch(&mut shader);
    }
}

pub struct ComputeDeltaTime;

impl<'a> System<'a> for ComputeDeltaTime {
    type SystemData = (Write<'a, DeltaTime>);

    fn run(&mut self, mut dt: Self::SystemData) {
        let now = now();
        dt.delta = now - dt.prev;
        dt.prev = now;
    }
}

pub struct Bounce;

impl<'a> System<'a> for Bounce {
    type SystemData = (
        ReadStorage<'a, Position>,
        WriteStorage<'a, Velocity>);

    fn run(&mut self, (pos, mut vel): Self::SystemData) {
        for (pos, vel) in (&pos, &mut vel).join() {
            if pos.0 < -1.0 || pos.0 > 1.0 {
                vel.0 *= -1.0;
            }
            if pos.1 < -1.0 || pos.1 > 1.0 {
                vel.1 *= -1.0;
            }
        }
    }
}