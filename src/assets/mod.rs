use std::{
    ffi::{OsStr, OsString},
    fs, iter,
    path::{Path, PathBuf}, collections::BTreeMap, array,
};

use arrayvec::ArrayVec;
use glam::{vec2, vec3, Vec2, Vec3};
use image::{imageops, RgbaImage};

use crate::{
    graphics::Vertex,
    types::{DirMap, SideMap},
};

use self::raw::{Meshlet, Tilelet};

mod raw;

pub const N_MIPS: usize = 5;

fn decompose_part<'m>(part: &'m Meshlet<'m>) -> ArrayVec<(Vec3, Vec3, Vec3, &'m Tilelet<'m>), 6> {
    match part {
        Meshlet::Cuboid { xyz0, xyz1, faces } => {
            let x0y0z0 = *xyz0;
            let x0y0z1 = vec3(xyz0.x, xyz0.y, xyz1.z);
            let x0y1z0 = vec3(xyz0.x, xyz1.y, xyz0.z);
            let x0y1z1 = vec3(xyz0.x, xyz1.y, xyz1.z);
            let x1y0z0 = vec3(xyz1.x, xyz0.y, xyz0.z);
            let x1y0z1 = vec3(xyz1.x, xyz0.y, xyz1.z);
            let x1y1z0 = vec3(xyz1.x, xyz1.y, xyz0.z);
            let x1y1z1 = *xyz1;

            let entries = [
                (x0y1z0, x0y0z0, x0y0z1, &faces.west),
                (x1y0z0, x1y1z0, x1y1z1, &faces.east),
                (x0y0z0, x1y0z0, x1y0z1, &faces.south),
                (x1y1z0, x0y1z0, x0y1z1, &faces.north),
                (x0y1z0, x1y1z0, x1y0z0, &faces.down),
                (x0y0z1, x1y0z1, x1y1z1, &faces.up),
            ];

            ArrayVec::from_iter(entries)
        }

        Meshlet::Rect {
            xyz0,
            xyz1,
            xyz2,
            face,
        } => {
            let entry = [(*xyz0, *xyz1, *xyz2, face)];

            ArrayVec::from_iter(entry)
        }
    }
}

const CELL_SIZE: u32 = 16;

pub type Quad = [Vertex; 4];

#[derive(Default, Debug)]
pub struct Block {
    pub culls: DirMap<bool>,
    pub mesh: SideMap<Box<[Quad]>>,
}

#[derive(Debug)]
pub struct Pack {
    pub atlases: [RgbaImage; N_MIPS],
    pub blocks: Box<[(String, Block)]>,
}

fn open_tiles(root: &mut PathBuf) -> Option<([RgbaImage; N_MIPS], Vec<OsString>)> {
    let mut tile_names = fs::read_dir(&root)
        .ok()?
        .map(|entry| Some(entry.ok()?.file_name()))
        .try_collect::<Vec<_>>()?;

    tile_names.sort_unstable();

    let width_cells = {
        let num_cells = tile_names.len().next_power_of_two();
        num_cells.isqrt() as u32
    };

    let mut atlases = array::from_fn(|mip_lvl| {
        let width = (CELL_SIZE >> mip_lvl as u32) * width_cells;
        RgbaImage::new(width, width)
    });

    for (idx, tile_name) in tile_names.iter().enumerate() {
        root.push(tile_name);

        let idx = idx as u32;
        let tile = image::open(&root).ok()?.to_rgba8();

        debug_assert!(
            [tile.width(), tile.height()] == [CELL_SIZE, CELL_SIZE],
            "incorrect tile size",
        );

        for mip_lvl in 0..N_MIPS {
            let x = (CELL_SIZE >> mip_lvl as u32) * (idx % width_cells);
            let y = (CELL_SIZE >> mip_lvl as u32) * (idx / width_cells);

            if mip_lvl > 0 {
                let tile = imageops::resize(&tile, CELL_SIZE >> mip_lvl as u32, CELL_SIZE >> mip_lvl as u32, imageops::FilterType::Lanczos3);
                imageops::replace(&mut atlases[mip_lvl], &tile, x as _, y as _);
            } else {
                imageops::replace(&mut atlases[0], &tile, x as _, y as _);
            }
        }

        root.pop();
    }

    Some((atlases, tile_names))
}

fn open_blocks(root: &mut PathBuf, tile_names: &[OsString]) -> Option<Vec<(String, Block)>> {
    let width_cells = {
        let num_cells = tile_names.len().next_power_of_two();
        num_cells.isqrt() as u32
    };

    let iter = fs::read_dir(root).ok()?.map(|entry| {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let src = fs::read(path).ok()?;
        let mut mesh = SideMap::<Vec<_>>::default();
        let raw::Block { culls, parts } = toml::from_slice(&src).unwrap();

        for (xyz0, xyz1, xyz2, face) in parts.iter().flat_map(decompose_part) {
            let Tilelet {
                tile,
                mut uv0,
                mut uv1,
                cull,
            } = *face;

            let tile_name = OsStr::new(tile);

            let idx = tile_names
                .binary_search_by_key(&tile_name, AsRef::as_ref)
                .ok()? as u32;

            let s = idx % width_cells;
            let t = idx / width_cells;

            for uv in [&mut uv0, &mut uv1] {
                *uv += vec2(s as _, t as _);
                *uv /= Vec2::splat(width_cells as _);
            }

            let xyz3 = xyz2 - (xyz1 - xyz0);
            let u0v0 = uv0;
            let u0v1 = vec2(uv0.x, uv1.y);
            let u1v0 = vec2(uv1.x, uv0.y);
            let u1v1 = uv1;

            let normal = (xyz1 - xyz0).cross(xyz3 - xyz0).normalize();
            let shadow = 1. - 0.2 * normal.x.abs() - 0.4 * normal.y.abs();
            let light = 15;

            #[rustfmt::skip]
            mesh[cull].push([
                Vertex { xyz: xyz0, uv: u1v0, shadow, light },
                Vertex { xyz: xyz1, uv: u0v0, shadow, light },
                Vertex { xyz: xyz2, uv: u0v1, shadow, light },
                Vertex { xyz: xyz3, uv: u1v1, shadow, light },
            ]);
        }

        let block = Block {
            culls,
            mesh: mesh.map(Vec::into_boxed_slice),
        };

        Some((name, block))
    });

    let air = Block {
        culls: DirMap::default(),
        mesh: SideMap::default(),
    };

    let air = Block::default();
    iter::once(Some((String::from("air"), air))).chain(iter).try_collect::<Vec<_>>()
}

pub fn open(path: impl AsRef<Path>) -> Option<Pack> {
    let mut path = path.as_ref().to_path_buf();

    path.push("tiles");
    let (atlases, tile_names) = open_tiles(&mut path)?;

    path.pop();
    path.push("blocks");
    let blocks = open_blocks(&mut path, &tile_names)?;

    for (idx, atlas) in atlases.iter().enumerate() {
        atlas.save(format!("atlas_{}.png", idx));
    }

    Some(Pack {
        atlases,
        blocks: blocks.into_boxed_slice(),
    })
}
