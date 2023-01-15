use cgmath::{num_traits::Signed, prelude::*, Deg, Euler, Point3, Quaternion, Rad, Vector3};
use specs::{Component, Entity, Read, ReadStorage, System, VecStorage, WriteStorage};

use crate::{
    input::Input,
    render::{mesh::Mesh, renderer::RENDER_DISTANCE},
    vector3,
    world::CHUNK_SIZE,
    DeltaTime,
};

use super::{bounds::Bounds, Transform};

/// Runs on a single `Entity` designated as the camera. This entity must have a `Transform` component otherwise the system will fail.
pub struct CameraSystem {
    camera: Entity,
}

impl CameraSystem {
    /// Creates a new `CameraSystem`, inserting a default `Camera` entity into the world.
    pub fn new(camera: Entity) -> CameraSystem {
        CameraSystem { camera }
    }
}

impl<'a> System<'a> for CameraSystem {
    type SystemData = (
        ReadStorage<'a, Transform>,
        WriteStorage<'a, Camera>,
        Read<'a, Input>,
        Read<'a, DeltaTime>,
    );

    fn run(&mut self, (transforms, mut cameras, input, delta_time): Self::SystemData) {
        let delta_time = delta_time.0;

        let transform = transforms
            .get(self.camera)
            .expect("No transform found on camera entity");

        let camera = cameras
            .get_mut(self.camera)
            .expect("No camera found on camera entity");

        camera.calculate_view_matrix(
            transform.position,
            camera.look_rotation() * vector3!(0.0, 0.0, 1.0),
            vector3!(0.0, 1.0, 0.0),
        );
        camera.calculate_projection_matrix();

        let sensitivity = 50.0;

        camera.pitch.0 = (camera.pitch.0
            + input.mouse.vertical_motion() * sensitivity * delta_time)
            .clamp(-camera.max_pitch.0, camera.max_pitch.0);
        camera.yaw.0 += input.mouse.horizontal_motion() * sensitivity * delta_time;
    }
}

#[derive(Clone)]
pub struct Camera {
    yaw: Deg<f32>,
    pitch: Deg<f32>,
    max_pitch: Deg<f32>,
    pub aspect_ratio: f32,
    pub near_dist: f32,
    pub far_dist: f32,
    pub fov: f32,
    view_matrix: [[f32; 4]; 4],
    projection_matrix: [[f32; 4]; 4],
}

impl Default for Camera {
    fn default() -> Self {
        let mut cam = Self {
            yaw: Deg(0.0),
            pitch: Deg(0.0),
            max_pitch: Deg(65.0),
            aspect_ratio: 16.0 / 9.0,
            near_dist: 0.1,
            far_dist: RENDER_DISTANCE as f32 * CHUNK_SIZE as f32,
            fov: 3.141592 / 3.0,
            view_matrix: [[0.0; 4]; 4],
            projection_matrix: [[0.0; 4]; 4],
        };
        cam.calculate_projection_matrix();
        cam
    }
}

impl Component for Camera {
    type Storage = VecStorage<Self>;
}

impl Camera {
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        self.projection_matrix
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        self.view_matrix
    }

    fn calculate_projection_matrix(&mut self) {
        let f = Rad::cot(Rad(self.fov / 2.0));
        self.projection_matrix = [
            [f / self.aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [
                0.0,
                0.0,
                (self.far_dist + self.near_dist) / (self.far_dist - self.near_dist),
                1.0,
            ],
            [
                0.0,
                0.0,
                -(2.0 * self.far_dist * self.near_dist) / (self.far_dist - self.near_dist),
                0.0,
            ],
        ]
    }

    pub fn look_rotation(&self) -> Quaternion<f32> {
        Quaternion::from(Euler::new(Deg(0.0), self.yaw, Deg(0.0)))
            * Quaternion::from(Euler::new(self.pitch, Deg(0.0), Deg(0.0)))
    }

    pub fn look_direction(&self) -> Vector3<f32> {
        (self.look_rotation() * vector3!(0.0, 0.0, 1.0)).normalize()
    }

    pub fn yaw(&self) -> Deg<f32> {
        self.yaw
    }

    pub fn calculate_view_matrix(
        &mut self,
        position: Vector3<f32>,
        direction: Vector3<f32>,
        up: Vector3<f32>,
    ) {
        let direction = direction.normalize();
        let s = vector3!(
            up.y * direction.z - up.z * direction.y,
            up.z * direction.x - up.x * direction.z,
            up.x * direction.y - up.y * direction.x
        )
        .normalize();
        let u = vector3!(
            direction.y * s.z - direction.z * s.y,
            direction.z * s.x - direction.x * s.z,
            direction.x * s.y - direction.y * s.x
        );

        let p = vector3!(
            -position.x * s.x - position.y * s.y - position.z * s.z,
            -position.x * u.x - position.y * u.y - position.z * u.z,
            -position.x * direction.x - position.y * direction.y - position.z * direction.z
        );

        self.view_matrix = [
            [s.x, u.x, direction.x, 0.0],
            [s.y, u.y, direction.y, 0.0],
            [s.z, u.z, direction.z, 0.0],
            [p.x, p.y, p.z, 1.0],
        ];
    }

    pub fn is_mesh_visible(
        &self,
        transform: &Transform,

        // TODO: make mesh bounds relative to the mesh, and use mesh_origin to transform them to world space
        mesh_origin: Vector3<f32>,
        mesh: &Mesh,
    ) -> bool {
        let view_frustum = ViewFrustum::new(
            transform.position,
            self.look_direction(),
            self.look_rotation() * vector3!(0.0, 1.0, 0.0),
            self.fov,
            self.near_dist,
            self.far_dist,
            self.aspect_ratio,
        );
        view_frustum.contains_box(mesh.bounds())
    }
}

#[derive(Debug)]
struct ViewFrustum {
    planes: [Plane; 6],
}

impl ViewFrustum {
    pub fn new(
        pos: Vector3<f32>,
        dir: Vector3<f32>,
        up: Vector3<f32>,
        fov: f32,
        near: f32,
        far: f32,
        aspect_ratio: f32,
    ) -> Self {
        let h_near = (fov / 2.0).tan() * near;
        let w_near = h_near * aspect_ratio;

        let z = -dir;
        let x = (up.cross(z)).normalize();
        let y = z.cross(x);

        let (nc, fc) = (pos - z * near, pos - z * far);

        Self {
            planes: [
                Plane {
                    point: nc,
                    normal: -z,
                },
                Plane {
                    point: fc,
                    normal: z,
                },
                Plane {
                    point: nc + y * h_near,
                    normal: ((nc + y * h_near) - pos).normalize().cross(x),
                },
                Plane {
                    point: nc - y * h_near,
                    normal: x.cross(((nc - y * h_near) - pos).normalize()),
                },
                Plane {
                    point: nc - x * w_near,
                    normal: ((nc - x * w_near) - pos).normalize().cross(y),
                },
                Plane {
                    point: nc + x * w_near,
                    normal: y.cross(((nc + x * w_near) - pos).normalize()),
                },
            ],
        }
    }

    pub fn contains_box(&self, bounds: Bounds) -> bool {
        let contains = true;
        for p in self.planes.iter() {
            let (mut v_in, mut v_out) = (0_u32, 0_u32);

            let vs = bounds.vertices();
            for v in &vs {
                if p.distance(*v).is_negative() {
                    v_out += 1;
                } else {
                    v_in += 1;
                }

                if v_out > 0 && v_in > 0 {
                    break;
                }
            }

            if v_in == 0 {
                return false;
            }
        }

        contains
    }
}

#[derive(Debug)]
struct Plane {
    point: Vector3<f32>,
    normal: Vector3<f32>,
}

impl Plane {
    pub fn distance(&self, pos: Vector3<f32>) -> f32 {
        (pos - self.point).dot(self.normal)
    }
}

#[cfg(test)]
mod tests {
    use crate::vector3;

    use super::Plane;

    #[test]
    fn test_plane_distance() {
        let pos = vector3!(5.0, 5.0, 0.0);

        let plane = Plane {
            point: vector3!(0.0, 0.0, 0.0),
            normal: vector3!(1.0, 0.0, 0.0),
        };

        assert_eq!(plane.distance(pos), 5.0);
    }

    #[test]
    fn test_plane_negative_distance() {
        let pos = vector3!(-5.0, 5.0, 10.0);

        let plane = Plane {
            point: vector3!(0.0, 0.0, 0.0),
            normal: vector3!(1.0, 0.0, 0.0),
        };

        assert_eq!(plane.distance(pos), -5.0);
    }
}
