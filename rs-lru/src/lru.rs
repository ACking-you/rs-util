#![allow(dead_code)]

use crate::list::{List, NonNullNode};
use crate::Cache;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem;

struct Item<K, V> {
   key: K,
   value: V,
}

impl<K, V> Item<K, V> {
   fn new(key: K, value: V) -> Self {
      Self { key, value }
   }
}

struct KeyRef<K, V>(NonNullNode<Item<K, V>>);

impl<K: Eq, V> Eq for KeyRef<K, V> {}

impl<K: Eq, V> PartialEq for KeyRef<K, V> {
   fn eq(&self, other: &Self) -> bool {
      unsafe {
         self
            .0
            .as_ref()
            .element
            .key
            .eq(&other.0.as_ref().element.key)
      }
   }
}

impl<K: Hash, V> Hash for KeyRef<K, V> {
   fn hash<H: Hasher>(&self, state: &mut H) {
      unsafe { self.0.as_ref().element.key.hash(state) }
   }
}

impl<K: Hash + Eq, V> Borrow<K> for KeyRef<K, V> {
   fn borrow(&self) -> &K {
      unsafe { &self.0.as_ref().element.key }
   }
}

struct LRUCache<K, V> {
   map: HashMap<KeyRef<K, V>, NonNullNode<Item<K, V>>>,
   list: List<Item<K, V>>,
   cap: usize,
}

impl<K: Hash + Eq, V> LRUCache<K, V> {
   pub fn with_capacity(cap: usize) -> Self {
      Self {
         map: HashMap::new(),
         list: List::new(),
         cap,
      }
   }

   fn update(&mut self, node: NonNullNode<Item<K, V>>) {
      if self.list.is_empty() {
         return;
      }
      self.list.splice_self_front(self.list.begin_node(), node);
   }
}

impl<K: Hash + Eq, V> Cache<K, V> for LRUCache<K, V> {
   fn get(&mut self, k: &K) -> Option<&V> {
      let op = self.map.get(k);
      if let Some(&node) = op {
         self.update(node);
         let value = unsafe { &node.as_ref().element.value };
         return Some(value);
      }
      None
   }

   fn insert(&mut self, k: K, v: V) -> Option<V> {
      // check cache
      // cache exist
      if let Some(node) = self.map.get(&k) {
         let mut node = *node;
         self.update(node);
         let value = unsafe { mem::replace(&mut node.as_mut().element.value, v) };
         return Some(value);
      }
      // cache not exist
      // check cap
      if self.map.len() + 1 > self.cap {
         // Pay attention to the lifetime of the pointer and don't let it die before the map removes
         if let Some(e) = self.list.back() {
            self.map.remove(&e.key);
         }
         self.list.pop_back();
      }
      // make node and insert
      self.list.push_front(Item::new(k, v));
      let iter = self.list.begin_node().unwrap();
      self.map.insert(KeyRef(iter), iter);
      None
   }

   fn remove(&mut self, k: &K) -> Option<V> {
      if let Some(node) = self.map.remove(k) {
         return Some(self.list.remove_node(node).value);
      }
      None
   }

   fn is_emtpy(&self) -> bool {
      self.map.is_empty() && self.list.is_empty()
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_cache() {
      let mut cache = LRUCache::with_capacity(2);

      // insert full
      assert_eq!(cache.insert(1, 100), None);
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.insert(2, 200), None);
      assert_eq!(cache.is_emtpy(), false);

      // test lru strategy
      // head:(2,200) tail:(1,100)
      assert_eq!(cache.get(&1), Some(&100));
      assert_eq!(cache.is_emtpy(), false);
      // head:(1,100) tail:(2,200) disuse:(2,200)
      assert_eq!(cache.insert(3, 300), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(3,300) tail:(1,100)
      assert_eq!(cache.get(&1), Some(&100));
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&2), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(3,300) tail:(1,100) disuse:(1,100)
      assert_eq!(cache.insert(4, 400), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(4,400) tail:(3,300) disuse:(3,300)
      assert_eq!(cache.insert(5, 500), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(5,500) tail:(4,400)
      assert_eq!(cache.get(&3), None);
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&4), Some(&400));
      assert_eq!(cache.is_emtpy(), false);
      // head:(5,500) tail:(4,400) disuse:(4,400)
      assert_eq!(cache.insert(6, 600), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(6,600) tail:(5,500)
      assert_eq!(cache.get(&2), None);
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&6), Some(&600));
      assert_eq!(cache.is_emtpy(), false);
      // head:(6,600) tail:(5,500) change:(6,600)->(6,700)
      assert_eq!(cache.insert(6, 700), Some(600));
      assert_eq!(cache.is_emtpy(), false);
      // head:(6,700) tail:(5,500) disuse:(5,500)
      assert_eq!(cache.insert(8, 800), None);
      assert_eq!(cache.is_emtpy(), false);
      // head:(8,800) tail:(6,700)
      assert_eq!(cache.get(&5), None);
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&8), Some(&800));
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&6), Some(&700));
      assert_eq!(cache.is_emtpy(), false);
      // remove
      assert_eq!(cache.remove(&6), Some(700));
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.get(&6), None);
      assert_eq!(cache.is_emtpy(), false);
      assert_eq!(cache.remove(&8), Some(800));
      assert_eq!(cache.is_emtpy(), true);
      assert_eq!(cache.get(&8), None);
      assert_eq!(cache.is_emtpy(), true);
   }
}
