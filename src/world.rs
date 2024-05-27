use std::{fmt::Debug, sync::Arc};

use bevy::{
    ecs::system::Resource,
    log::info_span,
    math::{I64Vec3, Vec3},
    render::mesh::Mesh,
};
use noise::NoiseFn;

use super::{
    chunks::chunk::{ChunkCoordinate, ChunkData, ChunkOctree},
    chunks::generate::{generator::WorldGenerator, noise::world_noise},
};

#[derive(Resource)]
pub struct World {
    seed: u32,
    chunks: ChunkOctree,
    generator: WorldGenerator,
}

impl World {
    pub fn new() -> Self {
        Self {
            seed: rand::random(),
            chunks: ChunkOctree::default(),
            generator: WorldGenerator::default(),
        }
    }

    pub fn generate_chunk(
        &mut self,
        chunk_coord: ChunkCoordinate,
        noise_fn: &impl NoiseFn<f64, 2>,
    ) {
        let _ = info_span!("generate_chunk").entered();
        if self.is_chunk_generated(chunk_coord) {
            return;
        }

        let chunk_data = self.generator.generate_chunk(chunk_coord, noise_fn);
        self.chunks.set_chunk_data(chunk_coord, chunk_data);
    }

    pub fn generate_chunks(&mut self, chunk_coords: Vec<ChunkCoordinate>) {
        let noise_fn = world_noise(self.seed);
        for chunk in chunk_coords {
            self.generate_chunk(chunk, &noise_fn);
        }
    }

    pub fn generate_chunk_mesh(&mut self, chunk_coord: ChunkCoordinate) -> Mesh {
        let _ = info_span!("generate_chunk_mesh").entered();
        let chunk_data = self.chunks.get_chunk_data(chunk_coord).unwrap();
        let adjacent_chunks = self.adjacent_chunk_data(chunk_coord);
        self.generator
            .generate_chunk_mesh(&chunk_data, adjacent_chunks)
    }

    pub fn get_chunk_data(&mut self, chunk_coord: ChunkCoordinate) -> Option<Arc<ChunkData>> {
        self.chunks.get_chunk_data(chunk_coord)
    }

    fn adjacent_chunk_data(&mut self, chunk_coord: ChunkCoordinate) -> Vec<Option<Arc<ChunkData>>> {
        chunk_coord
            .adjacent()
            .iter()
            .map(|coord| self.get_chunk_data(*coord))
            .collect()
    }

    pub fn is_chunk_generated(&mut self, chunk_coord: ChunkCoordinate) -> bool {
        self.chunks.get_chunk_data(chunk_coord).is_some()
    }

    pub fn is_chunk_empty(&mut self, chunk_coord: ChunkCoordinate) -> bool {
        self.chunks
            .get_chunk_data(chunk_coord)
            .map(|chunk_data| chunk_data.empty())
            .unwrap_or(false)
    }

    pub fn chunk_to_world(&self, chunk_coord: ChunkCoordinate) -> Vec3 {
        self.chunks.chunk_centre(chunk_coord)
    }

    pub fn block_to_chunk_coordinate(&self, block_coord: I64Vec3) -> ChunkCoordinate {
        (block_coord / self.chunks.chunk_size as i64).into()
    }

    pub fn world_to_chunk_coordinate(&self, world_pos: Vec3) -> ChunkCoordinate {
        ChunkCoordinate(I64Vec3::new(
            (world_pos.x / self.chunks.chunk_size as f32) as i64,
            (world_pos.y / self.chunks.chunk_size as f32) as i64,
            (world_pos.z / self.chunks.chunk_size as f32) as i64,
        ))
    }

    fn block_to_chunk_local(&self, block_coord: I64Vec3) -> ChunkCoordinate {
        (block_coord / self.chunks.chunk_size as i64).into()
    }
}

impl Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World").field("seed", &self.seed).finish()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_block_to_chunk_coordinate() {}

    #[test]
    fn test_is_chunk_generated() {}

    #[test]
    fn test_generate_chunk_updates_chunk_data() {}

    #[test]
    fn test_generate_chunk_mesh_none_for_ungenerated_chunk() {}

    #[test]
    fn test_generate_chunk_mesh_some_for_generated_chunk() {}
}
