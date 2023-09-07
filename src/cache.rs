pub struct Cache<const N: usize = 16> {
    entries: [[[Arc<RwLock<Chunk>>; 16]; 16]; 16],
}

impl Cache {}
