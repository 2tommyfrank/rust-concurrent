use std::hash::{Hash, Hasher};

pub trait Hashable { fn hash(&self) -> u64; }

impl<H: Hash> Hashable for H {
    fn hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub struct Hashed<T: Hash> {
    item: T,
    key: u64,
}

impl<T: Hash> Hashed<T> {
    pub fn new(item: T) -> Self {
        let key = Hashable::hash(&item);
        Hashed { item, key }
    }
    pub fn get(self) -> T { self.item }
}

impl<T: Hash> Hashable for Hashed<T> {
    fn hash(&self) -> u64 { self.key }
}
