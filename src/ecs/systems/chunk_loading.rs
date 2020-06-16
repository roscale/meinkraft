use std::collections::{HashSet, VecDeque};

use specs::{Join, ReadStorage, System, Write};

use crate::chunk::{Chunk, BlockID};
use crate::chunk_manager::ChunkManager;
use crate::physics::Interpolator;
use crate::player::{PlayerPhysicsState, PlayerState};
use noise::{SuperSimplex, Point2};

pub struct ChunkLoading {
    ss: SuperSimplex,
    loaded_chunks: HashSet<(i32, i32, i32)>,
    chunks_to_load: VecDeque<(i32, i32, i32)>,
}

impl ChunkLoading {
    pub fn new() -> Self {
        Self {
            ss: SuperSimplex::new(),
            loaded_chunks: HashSet::new(),
            chunks_to_load: VecDeque::new(),
        }
    }

    fn flood_fill(x: i32, y: i32, z: i32, distance: i32) -> HashSet<(i32, i32, i32)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((x, y, z, distance));

        while !queue.is_empty() {
            let (x, y, z, dist) = queue.pop_front().unwrap();
            visited.insert((x, y, z));
            if dist <= 0 {
                continue;
            }

            queue.push_back((x + 1, y, z, dist - 1));
            queue.push_back((x - 1, y, z, dist - 1));
            queue.push_back((x, y, z + 1, dist - 1));
            queue.push_back((x, y, z - 1, dist - 1));
            if y != 15 {
                queue.push_back((x, y + 1, z, dist - 1));
            }
            if y != 0 {
                queue.push_back((x, y - 1, z, dist - 1));
            }
        }
        visited
    }
}

impl<'a> System<'a> for ChunkLoading {
    type SystemData = (
        ReadStorage<'a, Interpolator<PlayerPhysicsState>>,
        Write<'a, ChunkManager>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_physics_state,
            mut chunk_manager,
        ) = data;

        for player_physics_state in (&player_physics_state).join() {
            let state = player_physics_state.get_latest_state();
            let x = state.position.x as i32 / 16;
            let y = state.position.y as i32 / 16 - 2;
            let z = state.position.z as i32 / 16;
            let visited = Self::flood_fill(x, y, z, 2);

            let old_chunks = self.loaded_chunks.difference(&visited);
            for xyz in old_chunks {
                chunk_manager.remove_chunk(xyz);
            }

            let new_chunks = visited.difference(&self.loaded_chunks);
            self.chunks_to_load.extend(new_chunks);
            self.loaded_chunks = visited;
        }

        if let Some(xyz) = self.chunks_to_load.pop_front() {
            chunk_manager.add_chunk(xyz, Chunk::random());
        }
    }
}