use glam::{Vec2, Vec3};
use serde::Deserialize;

use crate::types::{DirMap, Direction};

fn vec2_y() -> Vec2 {
    Vec2::Y
}

fn vec2_x() -> Vec2 {
    Vec2::X
}

fn vec3_one() -> Vec3 {
    Vec3::ONE
}

#[derive(Debug, Deserialize)]
pub(super) struct Tilelet<'t> {
    #[serde(borrow)]
    pub tile: &'t str,

    #[serde(default = "vec2_y")]
    pub uv0: Vec2,

    #[serde(default = "vec2_x")]
    pub uv1: Vec2,

    #[serde(default)]
    pub cull: Option<Direction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub(super) enum Meshlet<'m> {
    Cuboid {
        #[serde(default)]
        xyz0: Vec3,

        #[serde(default = "vec3_one")]
        xyz1: Vec3,

        #[serde(borrow)]
        #[serde(flatten)]
        faces: DirMap<Tilelet<'m>>,
    },

    Rect {
        xyz0: Vec3,
        xyz1: Vec3,
        xyz2: Vec3,

        #[serde(borrow)]
        #[serde(flatten)]
        face: Tilelet<'m>,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct Block<'b> {
    #[serde(default)]
    pub culls: DirMap<bool>,

    #[serde(borrow)]
    pub parts: Box<[Meshlet<'b>]>,
}
