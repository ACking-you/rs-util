#![allow(dead_code)]

use std::marker::PhantomData;
use std::ptr::NonNull;

pub(crate) type NonNullNode<T> = NonNull<Node<T>>;

pub(crate) struct Node<T> {
   next: Option<NonNullNode<T>>,
   prev: Option<NonNullNode<T>>,
   pub element: T,
}

impl<T> Node<T> {
   fn new(element: T) -> Self {
      Self {
         next: None,
         prev: None,
         element,
      }
   }
}

pub(crate) struct List<T> {
   head: Option<NonNullNode<T>>,
   tail: Option<NonNullNode<T>>,
   len: usize,
   marker: PhantomData<Box<Node<T>>>,
}

impl<T> List<T> {
   pub fn new() -> Self {
      Self {
         head: None,
         tail: None,
         len: 0,
         marker: PhantomData,
      }
   }

   pub fn is_empty(&self) -> bool {
      self.len == 0 && self.head.is_none() && self.tail.is_none()
   }

   pub fn push_back(&mut self, ele: T) {
      let mut node = Box::leak(Box::new(Node::new(ele))).into();
      match self.tail {
         None => {
            assert!(self.is_empty());
            self.head = Some(node);
            self.tail = Some(node);
         }
         Some(mut tail) => {
            unsafe {
               node.as_mut().prev = Some(tail);
               tail.as_mut().next = Some(node);
            }
            self.tail = Some(node);
         }
      }
      self.len += 1;
   }

   pub fn push_front(&mut self, ele: T) {
      let mut node = Box::leak(Box::new(Node::new(ele))).into();
      match self.head {
         None => {
            assert!(self.is_empty());
            self.head = Some(node);
            self.tail = Some(node);
         }
         Some(mut head) => {
            unsafe {
               node.as_mut().next = Some(head);
               head.as_mut().prev = Some(node);
            }
            self.head = Some(node);
         }
      }
      self.len += 1;
   }

   pub fn pop_front(&mut self) -> Option<T> {
      if let Some(e) = self.head {
         let ele = unsafe {
            let node_guard = Box::from_raw(e.as_ptr());
            self.head = e.as_ref().next;
            node_guard.element
         };
         // Prevent dangling pointer
         self.check_head();
         self.len -= 1;
         return Some(ele);
      }
      None
   }

   pub fn pop_back(&mut self) -> Option<T> {
      if let Some(e) = self.tail {
         let ele = unsafe {
            let node_guard = Box::from_raw(e.as_ptr());
            self.tail = e.as_ref().prev;
            node_guard.element
         };
         // Prevent dangling pointer
         self.check_tail();
         self.len -= 1;
         return Some(ele);
      }
      None
   }

   pub fn len(&self) -> usize {
      self.len
   }

   pub fn begin_node(&self) -> Option<NonNullNode<T>> {
      self.head
   }

   pub fn end_node(&self) -> Option<NonNullNode<T>> {
      self.tail
   }

   pub fn front(&self) -> Option<&T> {
      let node = self.begin_node()?;
      unsafe { Some(&node.as_ref().element) }
   }

   pub fn back(&self) -> Option<&T> {
      let node = self.end_node()?;
      unsafe { Some(&node.as_ref().element) }
   }

   pub fn splice_back(
      &mut self,
      dst_node: Option<NonNullNode<T>>,
      src: &mut List<T>,
      src_node: NonNullNode<T>,
   ) {
      src.detach(src_node);
      src.splice_back_node(dst_node, src_node);
      src.len -= 1;
      self.len += 1;
   }

   pub fn splice_front(
      &mut self,
      dst_node: Option<NonNullNode<T>>,
      src: &mut List<T>,
      src_node: NonNullNode<T>,
   ) {
      src.detach(src_node);
      self.splice_front_node(dst_node, src_node);
      src.len -= 1;
      self.len += 1;
   }

   pub fn splice_self_front(&mut self, dst_node: Option<NonNullNode<T>>, src_node: NonNullNode<T>) {
      if let Some(dst_node) = dst_node {
         if dst_node.eq(&src_node) {
            return;
         }
      }
      self.detach(src_node);
      self.splice_front_node(dst_node, src_node);
   }

   pub fn remove_node(&mut self, node: NonNullNode<T>) -> T {
      self.detach(node);
      self.len -= 1;
      unsafe {
         let boxed_node = Box::from_raw(node.as_ptr());
         boxed_node.element
      }
   }

   fn splice_front_node(&mut self, dst_node: Option<NonNullNode<T>>, mut src_node: NonNullNode<T>) {
      match dst_node {
         None => {
            unsafe {
               src_node.as_mut().next = None;
               src_node.as_mut().prev = None;
            }
            self.head = Some(src_node);
            self.tail = Some(src_node);
         }
         Some(mut dst_node) => unsafe {
            let dst_prev_node = dst_node.as_ref().prev;
            src_node.as_mut().next = Some(dst_node);
            src_node.as_mut().prev = dst_prev_node;
            dst_node.as_mut().prev = Some(src_node);
            match dst_prev_node {
               None => self.head = Some(src_node),
               Some(mut node) => node.as_mut().next = Some(src_node),
            }
         },
      }
   }

   fn splice_back_node(&mut self, dst_node: Option<NonNullNode<T>>, mut src_node: NonNullNode<T>) {
      match dst_node {
         None => {
            unsafe {
               src_node.as_mut().next = None;
               src_node.as_mut().prev = None;
            }
            self.head = Some(src_node);
            self.tail = Some(src_node);
         }
         Some(mut dst_node) => unsafe {
            let dst_next_node = dst_node.as_ref().next;
            src_node.as_mut().next = dst_next_node;
            src_node.as_mut().prev = Some(dst_node);
            dst_node.as_mut().next = Some(src_node);
            match dst_next_node {
               None => self.tail = Some(src_node),
               Some(mut node) => node.as_mut().prev = Some(src_node),
            }
         },
      }
   }

   fn detach(&mut self, node: NonNullNode<T>) {
      unsafe {
         match node.as_ref().prev {
            None => {
               self.head = node.as_ref().next;
               self.check_head();
            }
            Some(mut prev) => {
               prev.as_mut().next = node.as_ref().next;
            }
         }
         match node.as_ref().next {
            None => {
               self.tail = node.as_ref().prev;
               self.check_tail();
            }
            Some(mut next) => {
               next.as_mut().prev = node.as_ref().prev;
            }
         }
      }
   }

   // Prevent dangling pointer error in tail
   fn check_tail(&mut self) {
      match self.tail {
         None => {
            if self.head.is_some() {
               self.head = None
            }
         }
         Some(mut node) => unsafe {
            node.as_mut().next = None;
         },
      }
   }

   // Prevent dangling pointer error in head
   fn check_head(&mut self) {
      match self.head {
         None => {
            if self.tail.is_some() {
               self.tail = None
            }
         }
         Some(mut node) => unsafe {
            node.as_mut().prev = None;
         },
      }
   }
}

impl<T> Drop for List<T> {
   fn drop(&mut self) {
      while self.pop_back().is_some() {}
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_list_push_pop() {
      let mut list = List::new();
      // insert:1
      list.push_back(1);
      assert_eq!(list.front(), Some(&1));
      assert_eq!(list.len(), 1);
      // insert:2 1
      list.push_front(2);
      assert_eq!(list.front(), Some(&2));
      assert_eq!(list.back(), Some(&1));
      assert_eq!(list.len(), 2);
      // insert:2 1 3
      list.push_back(3);
      assert_eq!(list.back(), Some(&3));
      assert_eq!(list.len(), 3);
      // insert:1 3 pop:2
      assert_eq!(list.pop_front(), Some(2));
      assert_eq!(list.front(), Some(&1));
      assert_eq!(list.back(), Some(&3));
      assert_eq!(list.len(), 2);
      // insert:3 pop:1
      assert_eq!(list.pop_front(), Some(1));
      assert_eq!(list.front(), Some(&3));
      assert_eq!(list.back(), Some(&3));
      assert_eq!(list.len(), 1);
      // pop:3
      assert_eq!(list.pop_back(), Some(3));
      assert!(list.is_empty());
      assert_eq!(list.pop_front(), None);
      assert_eq!(list.pop_back(), None);
   }

   #[test]
   fn test_list_splice() {
      let mut list1 = List::new();
      let mut list2 = List::new();
      // list1:3 2 1 list2:4 5
      {
         list1.push_front(1);
         list1.push_front(2);
         list1.push_front(3);
         list2.push_back(4);
         list2.push_back(5);
      }
      let node = list2.end_node().unwrap();
      // list1:3 5 2 1 list2:4
      list1.splice_back(list1.begin_node(), &mut list2, node);
      assert_eq!(list1.front(), Some(&3));
      assert_eq!(list2.front(), Some(&4));
      // list1: 5 2 1 list2:4
      assert_eq!(list1.pop_front(), Some(3));
      assert_eq!(list1.front(), Some(&5));
      let node2 = list2.begin_node().unwrap();
      // list1:4 5 2 1 list2:emtpy
      list1.splice_front(list1.begin_node(), &mut list2, node2);
      assert_eq!(list1.front(), Some(&4));
      // list1:5 2 1 list2:emtpy
      assert_eq!(list1.pop_front(), Some(4));
      assert_eq!(list1.front(), Some(&5));
      assert!(list2.is_empty());
   }
}
