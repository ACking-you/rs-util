use std::hash::Hash;

mod list;
pub mod lru;
pub mod lru_k;

pub trait Cache<K: Hash + Eq, V> {
   fn get(&mut self, k: &K) -> Option<&V>;
   fn insert(&mut self, k: K, v: V) -> Option<V>;
   fn remove(&mut self, k: &K) -> Option<V>;
   fn is_emtpy(&self) -> bool;
}
