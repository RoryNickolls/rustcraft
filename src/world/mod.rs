use specs::{Component, VecStorage};

use crate::math::Vector3;
use crate::render::mesh::{Mesh, Vertex};
use crate::{vector3, vertex};

// Simple representation of the world.
// TODO: just a placeholder, will need replacing.
pub struct World {
    pub blocks: [[[bool; 16]; 16]; 16],
}

impl World {
    pub fn new() -> World {
        let mut blocks: [[[bool; 16]; 16]; 16] = [[[false; 16]; 16]; 16];

        for x in 0..blocks.len() {
            for y in 0..blocks[x].len() {
                for z in 0..blocks[x][y].len() {
                    blocks[x][y][z] = true;
                }
            }
        }

        World { blocks }
    }

    /// Generates a single chunk mesh from the whole world
    pub fn generate_chunk_mesh(&self) -> Mesh {
        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];

        let mut block = 0;

        for x in 0..self.blocks.len() {
            for y in 0..self.blocks[x].len() {
                for z in 0..self.blocks[x][y].len() {
                    if self.blocks[x][y][z] {
                        let cube = super::render::mesh::primitives::cube();
                        vertices.append(
                            &mut cube
                                .vertices
                                .into_iter()
                                .map(|v| {
                                    vertex!(
                                    position: v.position + vector3!(x as f32, y as f32, z as f32),
                                    normal: v.normal,
                                    uv: v.uv)
                                })
                                .collect(),
                        );
                        indices.append(
                            &mut cube.indices.into_iter().map(|i| i + (block * 24)).collect(),
                        );
                        block += 1;
                    }
                }
            }
        }

        Mesh::new(vertices, indices)
    }
}

#[derive(Default)]
pub struct Transform {
    pub position: Vector3,
    pub scale: Vector3,
}

impl Transform {
    pub fn new(position: Vector3, scale: Vector3) -> Transform {
        Transform {
            position: position,
            scale: scale,
        }
    }
    /// Calculates a model matrix for rendering
    pub fn matrix(&self) -> [[f32; 4]; 4] {
        [
            [self.scale.x, 0.0, 0.0, 0.0],
            [0.0, self.scale.y, 0.0, 0.0],
            [0.0, 0.0, self.scale.z, 0.0],
            [self.position.x, self.position.y, self.position.z, 1.0],
        ]
    }
}

impl Component for Transform {
    type Storage = VecStorage<Self>;
}
