use std::{
    mem,
    ops::{Index, IndexMut},
};

use glam::{IVec3, Vec3};
use serde::Deserialize;

pub const DIRECTIONS: [Direction; mem::variant_count::<Direction>()] = [
    Direction::West,
    Direction::East,
    Direction::South,
    Direction::North,
    Direction::Down,
    Direction::Up,
];

pub const SIDES: [Option<Direction>; mem::variant_count::<Direction>() + 1] = [
    Some(Direction::West),
    Some(Direction::East),
    Some(Direction::South),
    Some(Direction::North),
    Some(Direction::Down),
    Some(Direction::Up),
    None,
];

pub type Side = Option<Direction>;
pub type Layer<T, const N: usize> = [[T; N]; N];
pub type Cube<T, const N: usize> = [Layer<T, N>; N];

#[repr(u8)]
#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    West,
    East,
    South,
    North,
    Down,
    Up,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Direction::West => Direction::East,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::North => Direction::South,
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }
}

impl From<Direction> for IVec3 {
    fn from(value: Direction) -> Self {
        match value {
            Direction::West => IVec3::NEG_X,
            Direction::East => IVec3::X,
            Direction::South => IVec3::NEG_Y,
            Direction::North => IVec3::Y,
            Direction::Down => IVec3::NEG_Z,
            Direction::Up => IVec3::Z,
        }
    }
}

impl From<Direction> for Vec3 {
    fn from(value: Direction) -> Self {
        match value {
            Direction::West => Vec3::NEG_X,
            Direction::East => Vec3::X,
            Direction::South => Vec3::NEG_Y,
            Direction::North => Vec3::Y,
            Direction::Down => Vec3::NEG_Z,
            Direction::Up => Vec3::Z,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DirMap<T> {
    // #[serde(fallback = "x", fallback = "all")]
    pub west: T,

    // #[serde(fallback = "x", fallback = "all")]
    pub east: T,

    // #[serde(fallback = "y", fallback = "all")]
    pub south: T,

    // #[serde(fallback = "y", fallback = "all")]
    pub north: T,

    // #[serde(fallback = "z", fallback = "all")]
    pub down: T,

    // #[serde(fallback = "z", fallback = "all")]
    pub up: T,
}

impl<T> DirMap<T> {
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> DirMap<U> {
        DirMap {
            west: f(self.west),
            east: f(self.east),
            south: f(self.south),
            north: f(self.north),
            down: f(self.down),
            up: f(self.up),
        }
    }
}

impl<T> Index<Direction> for DirMap<T> {
    type Output = T;

    fn index(&self, index: Direction) -> &Self::Output {
        match index {
            Direction::West => &self.west,
            Direction::East => &self.east,
            Direction::South => &self.south,
            Direction::North => &self.north,
            Direction::Down => &self.down,
            Direction::Up => &self.up,
        }
    }
}

impl<T> IndexMut<Direction> for DirMap<T> {
    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        match index {
            Direction::West => &mut self.west,
            Direction::East => &mut self.east,
            Direction::South => &mut self.south,
            Direction::North => &mut self.north,
            Direction::Down => &mut self.down,
            Direction::Up => &mut self.up,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SideMap<T> {
    // #[serde(fallback = "x", fallback = "all")]
    pub west: T,

    // #[serde(fallback = "x", fallback = "all")]
    pub east: T,

    // #[serde(fallback = "y", fallback = "all")]
    pub south: T,

    // #[serde(fallback = "y", fallback = "all")]
    pub north: T,

    // #[serde(fallback = "z", fallback = "all")]
    pub down: T,

    // #[serde(fallback = "z", fallback = "all")]
    pub up: T,

    pub none: T,
}

impl<T> SideMap<T> {
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> SideMap<U> {
        SideMap {
            west: f(self.west),
            east: f(self.east),
            south: f(self.south),
            north: f(self.north),
            down: f(self.down),
            up: f(self.up),
            none: f(self.none),
        }
    }
}

impl<T> Index<Side> for SideMap<T> {
    type Output = T;

    fn index(&self, index: Side) -> &Self::Output {
        match index {
            Some(Direction::West) => &self.west,
            Some(Direction::East) => &self.east,
            Some(Direction::South) => &self.south,
            Some(Direction::North) => &self.north,
            Some(Direction::Down) => &self.down,
            Some(Direction::Up) => &self.up,
            None => &self.none,
        }
    }
}

impl<T> IndexMut<Side> for SideMap<T> {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        match index {
            Some(Direction::West) => &mut self.west,
            Some(Direction::East) => &mut self.east,
            Some(Direction::South) => &mut self.south,
            Some(Direction::North) => &mut self.north,
            Some(Direction::Down) => &mut self.down,
            Some(Direction::Up) => &mut self.up,
            None => &mut self.none,
        }
    }
}
