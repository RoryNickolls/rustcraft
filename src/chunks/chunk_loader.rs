use std::collections::{HashMap, HashSet, VecDeque};

use bevy::{
    asset::{AssetServer, Assets},
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::Parent,
    math::{I64Vec3, Vec3},
    pbr::{PbrBundle, StandardMaterial},
    prelude::default,
    render::{
        camera::Camera, color::Color, mesh::Mesh, primitives::Aabb, render_resource::Face,
        texture::Image,
    },
    transform::components::{GlobalTransform, Transform},
};

use super::chunk::ChunkCoordinate;
use crate::{player::PlayerLook, world::World};

use crate::player::Player;

#[derive(Component)]
pub struct Chunk {
    coord: ChunkCoordinate,
    dirty: bool,
}

#[derive(Resource)]
pub struct ChunkLoader {
    render_distance: u32,
    generate_queue: VecDeque<ChunkCoordinate>,
    load_queue: VecDeque<ChunkCoordinate>,
    unload_queue: VecDeque<ChunkCoordinate>,
    loaded: HashMap<ChunkCoordinate, Entity>,
}

impl ChunkLoader {
    pub fn new(render_distance: u32) -> Self {
        Self {
            render_distance,
            generate_queue: VecDeque::new(),
            load_queue: VecDeque::new(),
            unload_queue: VecDeque::new(),
            loaded: HashMap::new(),
        }
    }
}

pub fn gather_chunks(
    mut chunk_loader: ResMut<ChunkLoader>,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
    camera_query: Query<(&Parent, &GlobalTransform), (With<Camera>, Without<PlayerLook>)>,
) {
    let player = player_query.get_single().expect("could not find player");
    let (_, camera) = camera_query.get_single().expect("could not find camera");

    let queued_for_generation = chunk_loader
        .generate_queue
        .iter()
        .cloned()
        .collect::<HashSet<ChunkCoordinate>>();

    let queued_for_loading = chunk_loader
        .load_queue
        .iter()
        .cloned()
        .collect::<HashSet<ChunkCoordinate>>();

    let queued_for_unload = chunk_loader
        .unload_queue
        .iter()
        .cloned()
        .collect::<HashSet<ChunkCoordinate>>();

    let all_chunks: Vec<ChunkCoordinate> = all_chunks(
        player.translation,
        camera.forward(),
        chunk_loader.render_distance,
        &world,
    )
    .collect();

    let all_chunks_set: HashSet<ChunkCoordinate> = all_chunks.iter().cloned().collect();

    let loaded = chunk_loader
        .loaded
        .keys()
        .cloned()
        .collect::<HashSet<ChunkCoordinate>>();

    let to_unload = loaded
        .difference(&all_chunks_set)
        .filter(|chunk| !queued_for_unload.contains(chunk));

    for chunk in to_unload {
        chunk_loader.unload_queue.push_front(*chunk);
    }

    let to_generate = all_chunks
        .iter()
        .filter(|chunk| !queued_for_generation.contains(chunk))
        .filter(|chunk| !queued_for_loading.contains(chunk))
        .filter(|chunk| !loaded.contains(*chunk))
        .filter(|chunk| !world.is_chunk_empty(**chunk))
        .take(16);

    for chunk in to_generate {
        chunk_loader.generate_queue.push_front(*chunk);
    }
}

pub fn generate_chunks(mut world: ResMut<World>, mut chunk_loader: ResMut<ChunkLoader>) {
    while let Some(chunk) = chunk_loader.generate_queue.pop_front() {
        let mut chunks = vec![chunk];
        chunks.extend(chunk.adjacent());

        world.generate_chunks(chunks);

        chunk_loader.load_queue.push_front(chunk);
    }
}

pub fn load_chunks(
    mut commands: Commands,
    mut chunk_loader: ResMut<ChunkLoader>,
    mut world: ResMut<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut generated_meshes = vec![];
    while let Some(chunk) = chunk_loader.load_queue.pop_front() {
        if world.is_chunk_empty(chunk) {
            continue;
        }

        generated_meshes.push((chunk, world.generate_chunk_mesh(chunk)));
    }

    for (chunk, mesh) in generated_meshes.into_iter() {
        let (t, aabb) = chunk_components(chunk);
        let entity = commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(asset_server.load::<Image>("textures/blocks.png")),
                        reflectance: 0.0,
                        cull_mode: Some(Face::Front),
                        alpha_mode: bevy::pbr::AlphaMode::Blend,
                        ..default()
                    }),
                    transform: t,
                    ..default()
                },
                aabb,
                Chunk {
                    coord: chunk,
                    dirty: false,
                },
            ))
            .id();
        chunk_loader.loaded.insert(chunk, entity);
    }
}

pub fn unload_chunks(mut commands: Commands, mut chunk_loader: ResMut<ChunkLoader>) {
    while let Some(chunk) = chunk_loader.unload_queue.pop_front() {
        if let Some(entity) = chunk_loader.loaded.get(&chunk) {
            commands.entity(*entity).despawn();
            chunk_loader.loaded.remove(&chunk);
        }
    }
}

#[tracing::instrument]
fn all_chunks(
    camera_pos: Vec3,
    camera_forward: Vec3,
    max_distance: u32,
    world: &World,
) -> impl Iterator<Item = ChunkCoordinate> {
    let camera_chunk = world.block_to_chunk_coordinate(I64Vec3::new(
        camera_pos.x as i64,
        camera_pos.y as i64,
        camera_pos.z as i64,
    ));

    let mut stack = VecDeque::new();
    stack.push_back((camera_chunk, 0));

    let mut seen = HashSet::new();
    let mut all_chunks = Vec::new();
    while !stack.is_empty() {
        let (next, distance) = stack.pop_front().unwrap();
        all_chunks.push(next);
        seen.insert(next);

        if distance >= max_distance {
            continue;
        }

        for neighbour in next.adjacent().into_iter() {
            let direction: Vec3 =
                (world.chunk_to_world(neighbour) - world.chunk_to_world(camera_chunk)).normalize();
            let dot = camera_forward.dot(direction);
            if !seen.contains(&neighbour) && dot > 0.5 {
                stack.push_back((
                    neighbour,
                    (neighbour.0 - camera_chunk.0).abs().max_element() as u32,
                ));
            }
            seen.insert(neighbour);
        }
    }
    all_chunks.into_iter()
}

fn chunk_world_pos(chunk: ChunkCoordinate) -> Vec3 {
    Vec3::new(
        (chunk.0.x * 16) as f32,
        (chunk.0.y * 16) as f32,
        (chunk.0.z * 16) as f32,
    )
}

fn chunk_components(chunk: ChunkCoordinate) -> (Transform, Aabb) {
    let pos = chunk_world_pos(chunk);
    let t = Transform::from_translation(Vec3::new(pos.x, pos.y, pos.z));
    let aabb = Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(16.0, 16.0, 16.0));
    (t, aabb)
}
