use std::{time::{Instant, Duration}, collections::HashMap, sync::{Arc, atomic::AtomicU32}, f64::consts::SQRT_2, ops::{Index, IndexMut}};
use noise::{Perlin, NoiseFn};
use rand_xoshiro::rand_core::{RngCore, SeedableRng};

use glam::{Vec3, IVec3, ivec3, vec2, vec3};

use crate::{graphics::Vertex, BlockData, assets::Pack, types::{SIDES, SideMap, DirMap, Direction}};

#[derive(Debug)]
pub struct Chunk {
	pub nonce: u32,
    contents: Box<[[[i16; 32]; 32]; 32]>,
}

pub static mut MESHING_DURATION: Duration = Duration::ZERO;
pub static mut MESHING_TIMES: usize = 0;

static NONCE: AtomicU32 = AtomicU32::new(0);

fn fresh_nonce() -> u32 {
	NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Index<IVec3> for Chunk {
    type Output = i16;

    fn index(&self, index: IVec3) -> &Self::Output {
        let IVec3 { x, y, z } = index & 31;

        unsafe {
            // Location is already masked into range
            self.contents
                .get_unchecked(z as usize)
                .get_unchecked(y as usize)
                .get_unchecked(x as usize)
        }
    }
}

impl IndexMut<IVec3> for Chunk {
    fn index_mut(&mut self, index: IVec3) -> &mut Self::Output {
        let IVec3 { x, y, z } = index & 31;

        unsafe {
            // Location is already masked into range
            self.contents
                .get_unchecked_mut(z as usize)
                .get_unchecked_mut(y as usize)
                .get_unchecked_mut(x as usize)
        }
    }
}

impl Chunk {
	pub fn place(&mut self, location: IVec3, block: i16) {
		let IVec3 { x, y, z } = location & 31;
		self.contents[z as usize][y as usize][x as usize] = block;
		self.nonce = fresh_nonce();
	}

	pub fn generate(location: IVec3, pack: &Pack) -> Self {
		let mut chunk = Self::default();
		let mut rand = rand_xoshiro::Xoshiro256PlusPlus::seed_from_u64((2 * location.x + 3 * location.y + 5 * location.z) as u64);
		let perlin1 = Perlin::new(1);
		let perlin2 = Perlin::new(2);
		let perlin3 = Perlin::new(3);

		let b_air = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("air")).unwrap();
		let b_bedrock = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("bedrock.toml")).unwrap();
		let b_cactus = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("cactus.toml")).unwrap();
		let b_cobblestone = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("cobblestone.toml")).unwrap();
		let b_dirt = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("dirt.toml")).unwrap();
		let b_grass = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("grass.toml")).unwrap();
		let b_gravel = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("gravel.toml")).unwrap();
		let b_obsidian = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("obsidian.toml")).unwrap();
		let b_pumpkin = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("pumpkin.toml")).unwrap();
		let b_sand = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("sand.toml")).unwrap();
		let b_stone = pack.blocks.binary_search_by(|(n, _)| n.as_str().cmp("stone.toml")).unwrap();

        // jmi2k: placeholder to have something nice to test
        'layer: for k in 0..32 {
            for j in 0..32 {
                for i in 0..32 {
                    let block_loc = ivec3(i as _, j as _, k as _);
                    let IVec3 { x, y, z } = (location << 5) | block_loc;
					let fx = x as f64;
					let fy = y as f64;
					let factor = SQRT_2 / 1000.;
					let val1 = perlin1.get([fx * factor, fy * factor]) * 0.9;
					let val2 = perlin2.get([fx * 10. * factor, fy * 10. * factor]) * 0.09;
					let val3 = perlin3.get([fx * 100. * factor, fy * 100. * factor]) * 0.009;
					let val = val1 + val2 + val3;
					let min_h = -30.;
					let max_h = 30.;
					let h = (val * (max_h - min_h) + min_h) as i32;
					if !(-128..31).contains(&z) {
						continue 'layer;
					}
					let sea = [x, y, z].map(i32::to_be_bytes);
					let rand = rand.next_u32() as usize;

                    let idx = {
                        if z == -128 { b_bedrock }
                        else if z == -127 { [b_stone, b_cobblestone, b_bedrock, b_bedrock][rand % 4] }
                        else if ((-126..(h - 9)).contains(&z)) { [b_stone, b_cobblestone][(rand >> 2) % 2] }
                        else if ((h - 9)..(h - 4)).contains(&z) { [b_dirt, b_dirt, b_stone, b_cobblestone][(rand >> 3) % 4] }
                        else if ((h-4)..h).contains(&z) { b_dirt }
						else if z == h { [
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_grass, b_grass, b_grass,
							b_grass, b_dirt, b_grass, b_dirt,
							b_grass, b_air, b_grass, b_cobblestone][(rand >> 5) % 16] }
                        else { b_air }
                    };

                    unsafe { *chunk.contents.get_unchecked_mut(block_loc.z as usize).get_unchecked_mut(block_loc.y as usize).get_unchecked_mut(block_loc.x as usize) = idx as i16; }
                }
            }
        }

        chunk
	}
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
			nonce: fresh_nonce(),
            contents: unsafe { Box::new_zeroed().assume_init() },
        }
    }
}

pub struct Mesher {
	cached_meshes: HashMap<IVec3, Arc<(u32, Vec<Vertex>, Vec<u32>)>>,
}

impl Mesher {
	pub fn new() -> Self {
		Self {
			cached_meshes: HashMap::default(),
		}
	}

    pub fn build_mesh(&mut self, chunk: &Chunk, position: [i32; 3], pack: &Pack) -> Arc<(u32, Vec<Vertex>, Vec<u32>)> {
		if let Some(entry) = self.cached_meshes.get(&IVec3::from_array(position)) {
			if entry.0 == chunk.nonce {
				return entry.clone();
			}
		};

        let [x, y, z] = position;
		let then = Instant::now();
        let mut vertices = Vec::with_capacity(32_768);
        let mut indices = Vec::with_capacity(65_536);

        for (k, layer) in chunk.contents.into_iter().enumerate() {
            for (j, row) in layer.into_iter().enumerate() {
                for (i, block) in row.into_iter().enumerate() {
					let neighbors = DirMap {
						west: if i > 0 { chunk.contents[k][j][i - 1] } else { 0 },
						east: if i < 31 { chunk.contents[k][j][i + 1] } else { 0 },
						south: if j > 0 { chunk.contents[k][j - 1][i] } else { 0 },
						north: if j < 31 { chunk.contents[k][j + 1][i] } else { 0 },
						down: if k > 0 { chunk.contents[k - 1][j][i] } else { 0 },
						up: if k < 31 { chunk.contents[k + 1][j][i] } else { 0 },
					};
					let (_, block) = &pack.blocks[block as usize];
					let mesh = &block.mesh;
					let quads = SIDES.into_iter().flat_map(|side| {
						if let Some(dir) = side {
							let (_, b) = &pack.blocks[neighbors[dir] as usize];
							if b.culls[dir.opposite()] {
								[].iter()
							} else {
								mesh[side].iter()
							}
						} else {
							mesh[side].iter()
						}
					});
					let mut num_vertices = 0;
					let base = vertices.len() as u32;
					quads.flatten().map(|vertex| {
						num_vertices += 1;
						let mut xyz = vertex.xyz;
                        xyz += Vec3::new(
                            32. * x as f32 + i as f32,
                            32. * y as f32 + j as f32,
                            32. * z as f32 + k as f32,
                        );
						Vertex { xyz, ..*vertex }
					}).collect_into(&mut vertices);
					let num_quads = num_vertices / 4;

					(0..num_quads)
						.flat_map(|n| [0u32, 1, 2, 3, 0, 2].map(|idx| base + 4 * n as u32 + idx))
						.collect_into(&mut indices);
                }
            }
        }

		unsafe {
			MESHING_DURATION += then.elapsed();
			MESHING_TIMES += 1;
		}

		let mesh = Arc::new((chunk.nonce, vertices, indices));
		self.cached_meshes.insert(IVec3::from_array(position), mesh.clone());
		mesh
    }
}
