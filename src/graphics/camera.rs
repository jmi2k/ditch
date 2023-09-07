use glam::{EulerRot, Mat4, Quat, Vec3, Vec4};
use std::{f32::{consts::PI, EPSILON}, array};

const Z_NEAR: f32 = 1e-1;
const Z_FAR: f32 = 1e4;
const PITCH_LIMIT: f32 = PI / 2. - 1e-1;

#[derive(Debug, Clone, Copy, Default)]
pub struct Pov {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl Pov {
    pub fn new<Position>(position: Position, yaw: f32, pitch: f32) -> Self
    where
        Position: Into<Vec3>,
    {
        Self {
            position: position.into(),
            yaw,
            pitch,
        }
    }
}

impl From<Pov> for Mat4 {
    fn from(pov: Pov) -> Self {
        let Pov {
            position,
            yaw,
            pitch,
        } = pov;

        Mat4::from_rotation_x(-PI / 2.) * Mat4::from_euler(EulerRot::YXZ, 0., pitch, yaw) * Mat4::from_translation(-position)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Projection {
    #[default]
    Ndc,

    Perspective {
        aspect: f32,
        fov: f32,
    },
}

impl From<Projection> for Mat4 {
    fn from(projection: Projection) -> Self {
        match projection {
            Projection::Ndc => Self::IDENTITY,
            Projection::Perspective { fov, aspect } => {
                Self::perspective_rh(fov, aspect, Z_NEAR, Z_FAR)
            }
        }
    }
}

const REACH: usize = 6;

#[derive(Debug, Clone, Copy, Default)]
pub struct Camera {
    pub pov: Pov,
    pub projection: Projection,
}

impl Camera {
    pub fn turn(&mut self, yaw: f32, pitch: f32) {
        self.pov.yaw += yaw;
        self.pov.pitch = (self.pov.pitch + pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    pub fn walk(&mut self, to: Vec3) {
        self.pov.position += to;
    }

    pub fn reach_ray(&self) -> [Vec3; REACH] {
        println!("yaw={} pitch={}", self.pov.yaw, self.pov.pitch);
        let rotation = Mat4::from_euler(EulerRot::YXZ, 0., self.pov.pitch, self.pov.yaw);
        let step = (rotation * Vec4::Y).truncate();

        array::from_fn(|idx| self.pov.position + step * idx as f32)
    }
}

impl From<Camera> for Mat4 {
    fn from(value: Camera) -> Self {
        Self::from(value.projection) * Self::from(value.pov)
    }
}
