#![allow(dead_code)]

use crate::list::{List, NonNullNode};
use crate::Cache;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem;
// 小坑：注意标准库中的map需要调用key对应的一些方法才能正常删除，所以在此期间需要保证key不被释放内存！！！

struct Item<K, V> {
   key: K,
   value: V,
   freq: u32,
}

impl<K, V> Item<K, V> {
   fn new(key: K, value: V) -> Self {
      Self {
         key,
         value,
         freq: 0,
      }
   }
}

struct KeyNode<K, V>(NonNullNode<Item<K, V>>);

impl<K: Eq, V> Eq for KeyNode<K, V> {}

impl<K: Eq, V> PartialEq for KeyNode<K, V> {
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

impl<K: Hash, V> Hash for KeyNode<K, V> {
   fn hash<H: Hasher>(&self, state: &mut H) {
      unsafe { self.0.as_ref().element.key.hash(state) }
   }
}

impl<K: Hash + Eq, V> Borrow<K> for KeyNode<K, V> {
   fn borrow(&self) -> &K {
      unsafe { &self.0.as_ref().element.key }
   }
}

pub(crate) struct LRUkCache<K, V> {
   map: HashMap<KeyNode<K, V>, NonNullNode<Item<K, V>>>,
   fcfo: List<Item<K, V>>,
   lru: List<Item<K, V>>,
   freq: u32,
   cap: usize,
}

impl<K: Hash + Eq, V> LRUkCache<K, V> {
   pub fn with_capacity_freq(cap: usize, freq: u32) -> Self {
      Self {
         map: HashMap::new(),
         fcfo: List::new(),
         lru: List::new(),
         freq,
         cap,
      }
   }

   fn update(&mut self, mut node: NonNullNode<Item<K, V>>) {
      let item = unsafe { &mut node.as_mut().element };
      // item in lru
      if item.freq >= self.freq {
         self.lru.splice_self_front(self.lru.begin_node(), node);
         return;
      }
      // item in fcfo
      item.freq += 1;
      // move to lru list
      if item.freq >= self.freq {
         self
            .lru
            .splice_front(self.lru.begin_node(), &mut self.fcfo, node);
      }
   }

   pub fn len(&self) -> usize {
      self.map.len()
   }

   fn disuse(&mut self) -> Option<()> {
      // disuse fcfo
      if !self.fcfo.is_empty() {
         let item = self.fcfo.front()?;
         self.map.remove(&item.key)?;
         self.fcfo.pop_front()?;
      } else {
         // disuse lru
         let item = self.lru.back()?;
         self.map.remove(&item.key)?;
         self.lru.pop_back()?;
      }
      Some(())
   }
}

impl<K: Hash + Eq, V> Cache<K, V> for LRUkCache<K, V> {
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
         let ret = unsafe { mem::replace(&mut node.as_mut().element.value, v) };
         self.update(node);
         return Some(ret);
      }
      // cache not exist
      // check cap
      if self.map.len() + 1 > self.cap {
         self.disuse();
      }
      // make node and insert
      self.fcfo.push_back(Item::new(k, v));
      let node = self
         .fcfo
         .end_node()
         .expect("end_node must not be none,because just insert in the previous statement");
      let key = KeyNode(node);
      self.map.insert(key, node);
      None
   }

   fn remove(&mut self, k: &K) -> Option<V> {
      let node = self.map.remove(k)?;
      let item: &Item<K, V> = unsafe { &node.as_ref().element };
      // in lru list
      if item.freq >= self.freq {
         return Some(self.lru.remove_node(node).value);
      }
      // in fcfo list
      Some(self.fcfo.remove_node(node).value)
   }

   fn is_emtpy(&self) -> bool {
      self.map.is_empty() && self.fcfo.is_empty() && self.lru.is_empty()
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_cache() {
      let mut cache = LRUkCache::with_capacity_freq(2, 1);

      // fcfo:(1,10) lru:
      cache.insert(1, 10);
      assert_eq!(cache.get(&1), Some(&10));
      // fcfo: lru:(1,10)
      assert_eq!(cache.fcfo.len(), 0);
      assert_eq!(cache.lru.len(), 1);
      assert_eq!(cache.get(&1), Some(&10));
      assert_eq!(cache.fcfo.len(), 0);
      // fcfo:(2,20) lru:(1,10)
      cache.insert(2, 20);
      assert_eq!(cache.fcfo.len(), 1);
      assert_eq!(cache.lru.len(), 1);
      // fcfo:(2,20) lru:(1,10)
      assert_eq!(cache.get(&1), Some(&10));
      assert_eq!(cache.fcfo.len(), 1);
      assert_eq!(cache.lru.len(), 1);
      // fcfo:(3,30) lru:(1,10) disuse:(2,20)
      cache.insert(3, 30);
      assert_eq!(cache.fcfo.front().unwrap().value, 30);
      assert_eq!(cache.lru.front().unwrap().value, 10);
      assert_eq!(cache.get(&2), None);
      // fcfo:  lru: (3,30) (1,10)
      assert_eq!(cache.get(&3), Some(&30));
      assert!(cache.fcfo.is_empty());
      assert_eq!(cache.lru.len(), 2);
      assert_eq!(cache.lru.front().unwrap().value, 30);
      // fcfo:(4,40) lru:(3,30)  disuse:(1,10)
      cache.insert(4, 40);
      assert_eq!(cache.fcfo.len(), 1);
      assert_eq!(cache.lru.len(), 1);
      assert_eq!(cache.lru.front().unwrap().value, 30);
      assert_eq!(cache.len(), 2);
      // fcfo:(4,40) lru:
      assert_eq!(cache.remove(&3).unwrap(), 30);
      assert_eq!(cache.lru.is_empty(), true);
      assert_eq!(cache.fcfo.front().unwrap().value, 40);
      assert_eq!(cache.len(), 1);
      // fcfo: (5,50) (6,60)
      assert_eq!(cache.insert(5, 50), None);
      assert_eq!(cache.insert(6, 60), None);
      // fcfo:empty
      assert_eq!(cache.remove(&5), Some(50));
      assert_eq!(cache.remove(&6), Some(60));
      assert_eq!(cache.is_emtpy(), true);
   }
}
