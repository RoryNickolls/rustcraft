use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cgmath::{One, Quaternion, Vector2};
use specs::prelude::*;

use crate::{
    render::{mesh::Mesh, renderer::RenderMesh},
    vector2, vector3,
    world::{CHUNK_SIZE, WORLD_HEIGHT},
};

use super::{bounds::Bounds, camera::Camera, Transform};
pub struct ChunkGenerator {
    generate_distance: u32,
}

impl ChunkGenerator {
    pub fn new(generate_distance: u32) -> Self {
        Self { generate_distance }
    }
}

impl<'a> System<'a> for ChunkGenerator {
    type SystemData = (
        ReadStorage<'a, Camera>,
        WriteStorage<'a, Transform>,
        Write<'a, crate::world::World>,
    );

    fn run(&mut self, (cameras, transforms, mut game_world): Self::SystemData) {
        let (_, transform) = (&cameras, &transforms).join().next().unwrap();
        let camera_chunk = game_world.world_to_chunk(transform.position);

        let mut chunks: Vec<Vector2<i32>> = all_chunks(camera_chunk, self.generate_distance)
            .filter(|chunk| !game_world.is_chunk_generated(*chunk))
            .collect();
        chunks.sort_by(|c1, c2| {
            chunk_distance(camera_chunk, *c1).cmp(&chunk_distance(camera_chunk, *c2))
        });

        for chunk in chunks.iter().take(4) {
            game_world.generate_chunk(*chunk);
        }
    }
}

pub struct ChunkRenderer {
    render_distance: u32,
    active_chunks: HashSet<Vector2<i32>>,
    chunk_meshes: HashMap<Vector2<i32>, Arc<Mesh>>,
    chunk_entities: HashMap<Vector2<i32>, Entity>,
}

impl ChunkRenderer {
    pub fn new(render_distance: u32) -> Self {
        Self {
            render_distance,
            active_chunks: HashSet::new(),
            chunk_meshes: HashMap::new(),
            chunk_entities: HashMap::new(),
        }
    }
}

impl<'a> System<'a> for ChunkRenderer {
    type SystemData = (
        ReadStorage<'a, Camera>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, RenderMesh>,
        WriteStorage<'a, Bounds>,
        Write<'a, crate::world::World>,
        Entities<'a>,
    );

    fn run(
        &mut self,
        (cameras, mut transforms, mut render_meshes, mut bounds, game_world, entities): Self::SystemData,
    ) {
        let (_, transform) = (&cameras, &transforms).join().next().unwrap();
        let camera_chunk = game_world.world_to_chunk(transform.position);

        let all_chunks = all_chunks(camera_chunk, self.render_distance)
            .filter(|chunk| game_world.is_chunk_generated(*chunk))
            .filter(|chunk| game_world.are_neighbours_generated(*chunk))
            .collect::<HashSet<Vector2<i32>>>();

        let mut to_load = all_chunks
            .difference(&self.active_chunks)
            .cloned()
            .collect::<Vec<Vector2<i32>>>();
        to_load.sort_by(|c1, c2| {
            chunk_distance(camera_chunk, *c1).cmp(&chunk_distance(camera_chunk, *c2))
        });

        for chunk in to_load.into_iter().take(2) {
            self.active_chunks.insert(chunk);

            if let Some(e) = self.chunk_entities.get(&chunk) {
                render_meshes.get_mut(*e).unwrap().visible = true;
                continue;
            }

            let mesh = self
                .chunk_meshes
                .entry(chunk)
                .or_insert(Arc::new(game_world.generate_chunk_mesh(chunk)));

            let (t, r, b) = chunk_components(chunk, mesh.clone());
            let entity = entities
                .build_entity()
                .with(t, &mut transforms)
                .with(r, &mut render_meshes)
                .with(b, &mut bounds)
                .build();
            self.chunk_entities.insert(chunk, entity);
        }

        // TODO: ensure we remove old meshes so space is freed on the GPU
        for chunk in self
            .active_chunks
            .difference(&all_chunks)
            .cloned()
            .take(2)
            .collect::<Vec<Vector2<i32>>>()
        {
            let e = self.chunk_entities.remove(&chunk).unwrap();
            entities.delete(e).unwrap();
            self.active_chunks.remove(&chunk);
        }
    }
}

fn all_chunks(centre: Vector2<i32>, distance: u32) -> impl Iterator<Item = Vector2<i32>> {
    let (x_min, x_max) = (centre.x - distance as i32, centre.x + distance as i32);
    let (z_min, z_max) = (centre.y - distance as i32, centre.y + distance as i32);
    (x_min..x_max).flat_map(move |a| (z_min..z_max).map(move |b| vector2!(a, b)))
}

fn chunk_distance(chunk1: Vector2<i32>, chunk2: Vector2<i32>) -> u32 {
    ((chunk2.x - chunk1.x).abs() + (chunk2.y - chunk1.x).abs()) as u32
}

fn chunk_components(chunk: Vector2<i32>, mesh: Arc<Mesh>) -> (Transform, RenderMesh, Bounds) {
    let chunk_world_pos = vector3!(
        (chunk.x * CHUNK_SIZE as i32) as f32,
        0.0,
        (chunk.y * CHUNK_SIZE as i32) as f32
    );
    let t = Transform::new(chunk_world_pos, vector3!(1.0, 1.0, 1.0), Quaternion::one());
    let r = RenderMesh::new(mesh, true);
    let b = Bounds::new(
        vector3!(
            CHUNK_SIZE as f32 / 2.0,
            WORLD_HEIGHT as f32 / 2.0,
            CHUNK_SIZE as f32 / 2.0
        ),
        vector3!(CHUNK_SIZE as f32, WORLD_HEIGHT as f32, CHUNK_SIZE as f32),
    );

    (t, r, b)
}
