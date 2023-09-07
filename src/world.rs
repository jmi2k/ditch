use std::{collections::HashMap, sync::Arc};

use glam::IVec3;

use crate::{chunk::{Chunk, Mesher}, graphics::Vertex, assets::Pack};

#[derive(Default)]
pub struct World {
    pub loaded_chunks: HashMap<[i32; 3], Chunk>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        Self::default()
    }

    pub fn build_meshes<'a: 'b + 'c, 'b: 'a, 'c: 'a>(&'a self, mesher: &'b mut Mesher, location: IVec3, pack: &'c Pack, distance: i32) -> impl Iterator<Item = (IVec3, Arc<(u32, Vec<Vertex>, Vec<u32>)>)> + 'a + 'b + 'c {
        self.loaded_chunks
            .iter()
            .filter(move |(pos, _)| {
                let chunk_loc: IVec3 = location.clone() >> 5;
                chunk_loc.distance_squared(IVec3::from_array(**pos)) < distance*distance
            })
            .map(|(pos, chunk)| {
                (IVec3::from_slice(pos), mesher.build_mesh(chunk, pos.clone(), pack).clone())
            })
    }
}
