use std::hash::{Hash, Hasher};
use std::ops::Deref;

pub fn hash<H: Hash>(x: &H) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}

pub trait Hashable { fn hash(&self) -> u64; }

pub struct Hashed<T: Hash> {
    item: T,
    key: u64,
}

impl<T: Hash> Hashed<T> {
    pub fn item(item: T) -> Self {
        let key = hash(&item);
        Hashed { item, key }
    }
}

impl<T: Hash> Deref for Hashed<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.item }
}

impl<T: Hash> Hashable for Hashed<T> {
    fn hash(&self) -> u64 { self.key }
}
