use std::{collections::HashMap, marker::PhantomData, mem, ptr::NonNull};

pub struct Node<T> {
    val: T,
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    fn new(val: T) -> Self {
        Self {
            val,
            next: None,
            prev: None,
        }
    }

    fn ref_val(&self) -> &T {
        &self.val
    }

    fn into_val(self) -> T {
        self.val
    }
}

pub struct LinkedList<T> {
    length: usize,
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    _marker: PhantomData<Box<Node<T>>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            length: 0,
            head: None,
            tail: None,
            _marker: PhantomData,
        }
    }

    pub fn insert_front(&mut self, val: T) {
        let mut node = Box::new(Node::new(val));
        let node = NonNull::new(Box::into_raw(node)).unwrap();
        self.insert_front_raw(node);
    }

    pub fn insert_front_raw(&mut self, mut node: NonNull<Node<T>>) {
        unsafe {
            node.as_mut().next = self.head;
            node.as_mut().prev = None;
        }

        match self.head {
            None => self.tail = Some(node),
            Some(head) => unsafe { (*head.as_ptr()).prev = Some(node) },
        }

        self.head = Some(node);
        self.length += 1;
    }

    pub fn remove(&mut self, mut node: NonNull<Node<T>>) -> T {
        let node_mut = unsafe { node.as_mut() };
        self.length -= 1;
        match node_mut.prev {
            Some(prev) => unsafe { (*prev.as_ptr()).next = node_mut.next },
            None => self.head = node_mut.next,
        }
        match node_mut.next {
            Some(next) => unsafe { (*next.as_ptr()).prev = node_mut.prev },
            None => self.tail = node_mut.prev,
        }
        unsafe {
            let n = Box::from_raw(node.as_ptr());
            n.into_val()
        }
    }

    pub fn reinsert_front(&mut self, node: NonNull<Node<T>>) {
        self.remove(node);
        self.insert_front_raw(node);
    }

    pub fn remove_tail(&mut self) -> Option<T> {
        self.tail.map(|node| unsafe {
            self.length -= 1;
            let node = Box::from_raw(node.as_ptr());
            self.tail = node.prev;
            match self.tail {
                Some(tail) => (*tail.as_ptr()).next = None,
                None => self.head = None,
            }
            node.into_val()
        })
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.length,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut LinkedList<T>);
        impl<'a, T> Drop for DropGuard<'a, T> {
            fn drop(&mut self) {
                while self.0.remove_tail().is_some() {}
            }
        }

        while let Some(node) = self.remove_tail() {
            let guard = DropGuard(self);
            drop(node);
            mem::forget(guard);
        }
    }
}

pub struct Iter<'a, T: 'a> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    _marker: PhantomData<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            None
        } else {
            self.head.map(|node| {
                self.len -= 1;

                unsafe {
                    let node = &*node.as_ptr();
                    self.head = node.next;
                    &node.val
                }
            })
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for LinkedList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for cur in self.iter() {
            write!(f, "{:?} ", cur)?;
        }
        Ok(())
    }
}

struct LRUEntry<T: std::fmt::Debug> {
    key: Vec<u8>,
    value: T,
}

impl<T> LRUEntry<T>
where
    T: std::fmt::Debug,
{
    pub fn new(key: &[u8], value: T) -> Self {
        Self {
            key: key.to_vec(),
            value,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for LRUEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.value)?;
        Ok(())
    }
}

pub struct LRUCache<T>
where
    T: std::fmt::Debug,
{
    map: HashMap<Vec<u8>, NonNull<Node<LRUEntry<T>>>>,
    list: LinkedList<LRUEntry<T>>,
    capacity: usize,
}

impl<T> LRUCache<T>
where
    T: std::fmt::Debug,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            list: LinkedList::new(),
            capacity,
        }
    }

    pub fn insert(&mut self, key: &[u8], value: T) -> Option<T> {
        let new_node = LRUEntry::new(key, value);
        let new_node = Box::new(Node::new(new_node));
        let new_node = NonNull::new(Box::into_raw(new_node)).unwrap();

        match self.map.get(key) {
            Some(&entry) => unsafe {
                let val = self.list.remove(entry);
                self.list.insert_front_raw(new_node);
                self.map.insert(key.to_vec(), new_node);
                Some(val.value)
            },
            None => {
                let mut val = None;
                if self.list.length >= self.capacity {
                    // let removed_key = self.list.remove_tail();
                    if let Some(entry) = self.list.remove_tail() {
                        self.map.remove(&entry.key);
                        val = Some(entry.value);
                    }
                }
                self.list.insert_front_raw(new_node);
                self.map.insert(key.to_vec(), new_node);
                val
            }
        }
    }

    pub fn get(&mut self, key: &[u8]) -> Option<&T> {
        match self.map.get(key) {
            Some(&node) => unsafe {
                let value = &node.as_ref().val.value;
                self.list.reinsert_front(node);
                Some(value)
            },
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    mod test_linkedlist {
        use super::super::LinkedList;

        #[test]
        fn test_insert() {
            let mut list: LinkedList<i32> = LinkedList::new();
            list.insert_front(2);
            list.insert_front(3);
            list.insert_front(4);
            let result = format!("{:?}", list);
            assert_eq!("4 3 2 ", result);
        }
    }

    mod test_lru_cache {
        use super::super::LRUCache;
        use rand::prelude::*;

        #[test]
        fn test() {
            println!("single thread.....");
            let mut lru = LRUCache::new(5);
            lru.insert(&5_i32.to_le_bytes(), 5);
            println!("{:?}", lru.list);
            lru.insert(&0_i32.to_le_bytes(), 0);
            println!("{:?}", lru.list);
            lru.insert(&2_i32.to_le_bytes(), 2);
            println!("{:?}", lru.list);
            lru.insert(&6_i32.to_le_bytes(), 6);
            println!("{:?}", lru.list);
            lru.insert(&1_i32.to_le_bytes(), 1);
            println!("{:?}", lru.list);
            lru.insert(&6_i32.to_le_bytes(), 6);
            println!("{:?}", lru.list);
            lru.insert(&8_i32.to_le_bytes(), 8);
            println!("{:?}", lru.list);
            println!();
        }
        #[test]
        fn test1() {
            let mut lru = LRUCache::new(3);
            let arr: Vec<usize> = vec![
                3, 4, 4, 0, 0, 4, 7, 5, 2, 8, 7, 9, 9, 5, 4, 0, 2, 9, 9, 2, 0, 0, 5, 9, 9, 4, 4, 8,
                2, 1, 3, 8, 7, 3, 9, 3, 9, 3, 4, 5, 1, 5, 7, 7, 1, 0, 8, 5, 5, 4, 3, 6, 5, 1, 9, 4,
                0, 5, 9, 8, 5, 4, 1, 0, 4, 0, 2, 3, 9, 3, 8, 7, 8, 3, 0, 5, 3, 7, 1, 6, 4, 2, 7, 8,
                4, 3, 0, 1, 5, 2, 3, 4, 6, 3, 6, 4, 0, 5, 2, 3, 5, 7, 0, 2, 5, 2, 4, 2, 0, 6, 3, 5,
                9, 6, 7, 0, 1, 7, 2, 7, 7, 6, 3, 6, 1, 3, 0, 8, 7, 9, 0, 7, 5, 3, 2, 0, 0, 0, 8, 1,
                6, 7, 9, 3, 6, 5, 9, 8, 6, 3, 0, 6, 9, 1, 3, 2, 9, 7, 5, 4, 9, 3, 1, 1, 6, 4, 6, 6,
                8, 6, 6, 9, 0, 0, 7, 1, 0, 3, 9, 0, 9, 7, 6, 4, 4, 4, 7, 9, 8, 7, 9, 8, 6, 4, 6, 6,
                1, 0, 6, 1, 5, 8, 8, 7, 8, 2, 0, 6, 5, 4, 3, 5, 4, 0, 1, 2, 6, 8, 8, 2, 0, 4, 1, 2,
                6, 6, 6, 4, 4, 8, 0, 4, 0, 8, 9, 0, 9, 5, 8, 6, 8, 2, 6, 3, 4, 6, 9, 1, 5, 1, 9, 2,
                0, 3, 8, 2, 2, 0, 1, 3, 4, 2, 9, 8, 5, 9, 5, 9, 0, 1, 0, 9, 6, 4, 1, 4, 0, 6, 1, 0,
                0, 4, 9, 2, 1, 3, 7, 8, 4, 1, 5, 9, 7, 1, 0, 9, 5, 9, 8, 9, 2, 0, 1, 4, 7, 1, 4, 3,
                3, 8, 2, 7, 4, 3, 3, 0, 1, 6, 2, 9, 3, 7, 7, 4, 5, 9, 3, 9, 0, 1, 3, 7, 7, 4, 2, 3,
                0, 4, 8, 6, 8, 7, 8, 5, 4, 2, 8, 9, 7, 0, 5, 0, 6, 4, 5, 1, 4, 8, 4, 3, 8, 0, 8, 2,
                8, 9, 1, 4, 9, 3, 5, 5, 1, 9, 8, 4, 8, 3, 1, 4, 5, 6, 8, 3, 7, 2, 6, 1, 4, 8, 5, 6,
                2, 2, 3, 2, 2, 1, 6, 3, 9, 5, 1, 3, 6, 3, 4, 4, 0, 2, 2, 4, 8, 1, 6, 9, 4, 7, 4, 5,
                9, 2, 4, 6, 0, 4, 6, 3, 8, 5, 3, 6, 8, 0, 9, 7, 0, 9, 6, 7, 9, 3, 3, 1, 3, 9, 9, 5,
                6, 3, 6, 1, 1, 6, 0, 6, 3, 8, 0, 0, 7, 0, 5, 9, 8, 2, 7, 1, 1, 1, 7, 6, 5, 5, 4, 0,
                6, 3, 5, 7, 9, 4, 9, 4, 8, 6, 4, 5, 3, 4, 4, 9, 6, 9, 3, 3, 2, 3, 2, 4, 5, 0, 1, 5,
                5, 1, 2, 0, 0, 9, 7, 8, 1, 7, 5, 0, 0, 7, 4, 6, 4, 3, 5, 9, 4, 6, 4, 5, 3, 0, 4, 8,
                8, 3, 1, 6, 1, 5, 0, 2, 0, 7, 3, 5, 4, 8, 7, 4, 0, 3, 7, 4, 4, 3, 4, 5, 8, 5, 6, 5,
                4, 3, 3, 6, 1, 4, 8, 3, 0, 2, 2, 9, 8, 6, 3, 5, 3, 5, 6, 5, 0, 8, 2, 6, 7, 2, 2, 0,
                5, 2, 1, 6, 8, 9, 4, 0, 4, 3, 4, 0, 5, 5, 6, 4, 0, 4, 4, 9, 3, 8, 6, 2, 7, 3, 7, 4,
                9, 4, 6, 1, 2, 0, 6, 4, 5, 8, 8, 7, 8, 9, 6, 0, 1, 1, 6, 3, 2, 6, 7, 1, 4, 0, 7, 1,
                0, 9, 3, 4, 9, 6, 4, 7, 2, 8, 0, 8, 1, 6, 6, 7, 9, 7, 6, 5, 5, 9, 3, 3, 6, 9, 1, 5,
                2, 4, 6, 2, 0, 9, 1, 7, 3, 8, 5, 9, 4, 0, 8, 8, 6, 1, 2, 5, 9, 4, 6, 8, 5, 6, 7, 1,
                0, 1, 8, 9, 9, 9, 9, 7, 4, 8, 0, 4, 0, 9, 3, 8, 3, 4, 9, 1, 2, 2, 6, 7, 7, 9, 4, 3,
                0, 1, 7, 9, 9, 8, 4, 0, 8, 1, 2, 1, 7, 5, 5, 5, 8, 0, 5, 1, 3, 3, 5, 8, 7, 5, 9, 4,
                6, 2, 6, 1, 2, 8, 1, 9, 7, 9, 5, 3, 6, 5, 7, 1, 5, 4, 2, 9, 6, 9, 7, 5, 4, 8, 4, 0,
                6, 8, 1, 0, 0, 8, 3, 3, 3, 1, 0, 7, 3, 0, 4, 5, 5, 0, 7, 6, 9, 3, 3, 8, 1, 1, 3, 3,
                3, 6, 4, 2, 9, 0, 1, 9, 8, 3, 9, 8, 0, 9, 4, 9, 3, 3, 5, 9, 8, 7, 8, 0, 0, 2, 1, 0,
                7, 4, 5, 0, 7, 0, 4, 1, 5, 9, 9, 7, 1, 5, 4, 8, 2, 5, 9, 5, 7, 5, 2, 9, 9, 8, 2, 2,
                2, 1, 2, 1, 6, 0, 1, 3, 4, 2, 1, 4, 0, 8, 9, 7, 8, 7, 5, 5, 3, 2, 5, 6, 5, 1, 0, 2,
                5, 4, 5, 9, 7, 2, 3, 9, 6, 5, 6, 4, 8, 9, 1, 8, 9, 6, 0, 9, 8, 1, 7, 8, 5, 8, 4, 7,
                8, 6, 0, 4, 5, 6, 1, 2, 4, 8, 7, 8, 2, 3, 9, 8, 2, 5, 6, 2, 5, 0, 8, 4, 5, 9, 3, 3,
                4, 4, 9, 5, 6, 1, 7, 2, 7, 0, 3, 0, 6, 1, 5, 8, 3, 8, 2, 3, 4, 7, 6, 1, 7, 7, 4, 5,
                7, 2, 2, 7, 3, 8, 9, 0, 4, 6, 6, 4, 3, 1, 4, 8, 6, 6, 7, 6, 2, 5, 6, 0, 5, 3, 1, 5,
                0, 0, 2, 7, 7, 4, 6, 3, 5, 2, 6, 2, 9, 4, 5, 2, 5, 9, 8, 2, 6, 2, 9, 5, 0, 7, 9, 0,
                8, 9, 0, 9, 4, 0, 2, 2, 6, 0, 7, 7, 6, 9, 5, 1, 8, 7, 9, 9, 1, 9, 6, 1, 5, 9, 2, 4,
                6, 0, 1, 1, 7, 4, 9, 1, 6, 1, 0, 9, 5, 0, 1, 1, 2, 6, 4, 9, 6, 4, 2, 2, 6, 4, 0, 5,
                8, 8, 2, 9, 4, 4, 1, 4, 8, 5, 0, 2, 8, 1, 4, 9, 6, 9, 6, 7, 2, 8, 7, 4, 4, 8, 0, 4,
                7, 7, 7, 8, 9, 8, 4, 7, 1, 6, 6, 2, 8, 4, 6, 6, 5, 2, 1, 6, 8, 5, 3, 7, 0, 0, 1, 8,
                1, 7, 6, 6, 4, 5, 3, 9, 6, 1, 8, 2, 1, 0, 7, 0, 0, 3, 7, 2, 5, 6, 7, 8, 5, 0, 8, 6,
                9, 6, 5, 4, 8, 6, 9, 5, 9, 1, 8, 7, 7, 4, 9, 8, 2, 0, 2, 0, 9, 1, 6, 2, 6, 7, 7, 2,
                0, 4, 5, 3, 7, 7, 2, 8, 3, 5, 7, 4, 7, 5, 8, 2, 1, 8, 0, 6, 7, 5, 4, 8, 3, 8, 0, 5,
                9, 2, 7, 8, 3, 0, 2, 7, 7, 2, 1, 6, 6, 8, 9, 0, 6, 3, 8, 3, 0, 4, 4, 9, 6, 8, 0, 0,
                3, 6, 2, 9, 8, 0, 0, 2, 9, 8, 2, 3, 5, 2, 5, 8, 9, 0, 5, 0, 5, 7, 4, 4, 1, 5, 8, 5,
                4, 1, 9, 9, 6, 2, 4, 3, 7, 0, 9, 4, 4, 5, 6, 3, 2, 1, 7, 1, 4, 2, 1, 4, 9, 2, 6, 5,
                3, 2, 7, 8, 0, 4, 3, 7, 9, 9, 5, 5, 6, 7, 4, 5, 9, 3, 6, 7, 7, 9, 4, 7, 8, 1, 2, 2,
                1, 2, 6, 4, 7, 1, 8, 6, 2, 1, 6, 2, 7, 6, 4, 5, 8, 5, 6, 0, 2, 8, 3, 7, 6, 3, 1, 9,
                6, 4, 6, 5, 4, 0, 3, 8, 4, 4, 5, 7, 2, 8, 8, 5, 5, 7, 0, 8, 8, 6, 5, 6, 6, 0, 6, 5,
                0, 1, 7, 4, 3, 7, 1, 1, 4, 4, 7, 1, 8, 8, 6, 7, 7, 5, 3, 0, 5, 2, 4, 0, 9, 9, 9, 8,
                4, 9, 9, 8, 8, 2, 0, 3, 3, 4, 7, 8, 3, 5, 4, 6, 4, 8, 8, 5, 2, 2, 0, 3, 7, 9, 5, 9,
                1, 4, 3, 1, 0, 1, 6, 8, 3, 4, 6, 8, 6, 9, 4, 6, 1, 8, 5, 0, 9, 2, 8, 0, 1, 1, 3, 2,
                1, 5, 1, 5, 8, 6, 8, 3, 5, 8, 7, 5, 5, 3, 9, 8, 7, 8, 5, 7, 7, 9, 7, 8, 8, 6, 2, 9,
                3, 9, 2, 3, 2, 6, 3, 7, 8, 8, 9, 1, 4, 3, 2, 7, 4, 6, 8, 3, 5, 6, 8, 2, 9, 3, 1, 1,
                6, 1, 3, 5, 7, 2, 2, 3, 1, 6, 1, 9, 8, 8, 5, 0, 4, 1, 3, 9, 8, 1, 7, 4, 2, 2, 2, 8,
                1, 8, 5, 5, 2, 9, 4, 9, 2, 4, 7, 7, 1, 0, 1, 1, 3, 8, 7, 2, 8, 9, 8, 8, 2, 9, 0, 1,
                5, 8, 3, 2, 2, 6, 1, 3, 6, 6, 0, 0, 4, 8, 2, 8, 9, 4, 2, 7, 9, 0, 0, 8, 4, 7, 2, 8,
                7, 4, 5, 1, 3, 7, 7, 3, 2, 6, 8, 6, 2, 8, 1, 6, 0, 3, 1, 6, 0, 2, 4, 9, 4, 1, 2, 7,
                0, 6, 4, 9, 1, 4, 2, 1, 5, 5, 7, 1, 6, 9, 2, 3, 6, 6, 1, 6, 7, 3, 0, 6, 6, 8, 5, 0,
                4, 5, 6, 1, 1, 4, 6, 4, 7, 5, 7, 9, 1, 8, 3, 1, 9, 0, 9, 4, 9, 2, 9, 1, 6, 3, 5, 9,
                1, 2, 5, 4, 2, 3, 9, 0, 4, 4, 6, 1, 3, 9, 8, 3, 2, 5, 7, 7, 9, 0, 6, 8, 9, 2, 7, 5,
                9, 8, 4, 8, 2, 6, 5, 6, 8, 2, 5, 5, 4, 3, 6, 8, 5, 5, 3, 2, 8, 0, 3, 2, 3, 5, 2, 0,
                4, 5, 8, 3, 7, 0, 4, 7, 4, 4, 7, 0, 7, 3, 9, 5, 7, 0, 9, 0, 7, 5, 7, 0, 6, 1, 8, 3,
                0, 1, 9, 1, 7, 1, 6, 6, 3, 0, 3, 1, 7, 5, 5, 1, 1, 9, 5, 6, 2, 0, 7, 8, 8, 9, 2, 0,
                3, 9, 2, 0, 2, 7, 1, 4, 5, 9, 6, 6, 5, 6, 8, 2, 0, 2, 4, 8, 0, 4, 2, 6, 9, 7, 3, 2,
                5, 5, 0, 0, 1, 9, 6, 7, 1, 1, 9, 8, 2, 8, 5, 8, 9, 2, 4, 9, 4, 1, 2, 2, 3, 4, 1, 3,
                5, 5, 1, 3, 5, 2, 5, 0, 8, 2, 6, 0, 7, 3, 6, 1, 5, 2, 3, 7, 7, 1, 6, 9, 5, 2, 0, 2,
                0, 4, 3, 6, 3, 8, 6, 6, 3, 0, 5, 8, 5, 8, 7, 7, 0, 4, 6, 8, 3, 4, 3, 6, 4, 5, 2, 3,
                6, 8, 3, 7, 4, 9, 6, 4, 6, 0, 4, 1, 0, 9, 2, 7, 5, 1, 2, 2, 3, 7, 5, 2, 6, 5, 7, 0,
                6, 6, 8, 1, 3, 6, 6, 0, 7, 8, 4, 6, 5, 9, 4, 8, 2, 8, 8, 2, 2, 7, 8, 1, 4, 6, 8, 0,
                3, 7, 5, 1, 6, 5, 5, 5, 6, 9, 4, 8, 3, 7, 3, 3, 6, 3, 4, 3, 8, 0, 4, 0, 9, 0, 1, 6,
                0, 1, 2, 3, 9, 9, 2, 3, 5, 6, 6, 0, 1, 6, 1, 1, 2, 7, 4, 1, 5, 1, 1, 1, 0, 9, 9, 8,
                6, 0, 2, 4, 3, 8, 6, 8, 6, 0, 1, 9, 9, 2, 0, 6, 8, 0, 7, 4, 0, 5, 1, 2, 2, 8, 5, 5,
                3, 8, 4, 6, 2, 2, 9, 3, 4, 8, 2, 7, 9, 4, 5, 4, 0, 4, 9, 2, 7, 2, 2, 8, 7, 2, 5, 7,
                6, 5, 8, 1, 3, 5, 3, 4, 3, 3, 9, 6, 8, 5, 3, 5, 1, 9, 3, 3, 4, 2, 7, 2, 2, 1, 9, 5,
                0, 5, 5, 2, 4, 3, 4, 2, 1, 4, 6, 0, 2, 9, 3, 3, 6, 6, 2, 6, 3, 2, 5, 6, 0, 4, 8, 3,
                8, 3, 7, 5, 5, 0, 8, 5, 0, 0, 4, 6, 7, 0, 1, 6, 9, 5, 5, 6, 5, 9, 1, 2, 9, 5, 7, 4,
                6, 7, 4, 8, 8, 1, 5, 5, 1, 8, 5, 6, 0, 8, 0, 6, 2, 9, 8, 5, 2, 9, 2, 9, 0, 1, 3, 9,
                4, 7, 5, 0, 4, 4, 1, 2, 5, 0, 4, 8, 5, 0, 4, 7, 6, 7, 4, 0, 8, 2, 5, 7, 1, 7, 9, 5,
                7, 0, 6, 5, 0, 3, 3, 6, 5, 8, 0, 4, 8, 7, 7, 8, 4, 4, 5, 8, 7, 0, 3, 3, 2, 8, 0, 7,
                4, 0, 6, 6, 6, 5, 8, 5, 4, 9, 7, 3, 7, 9, 4, 9, 3, 5, 4, 5, 5, 7, 0, 7, 9, 9, 4, 2,
                1, 7, 5, 8, 3, 6, 3, 3, 5, 1, 7, 6, 6, 7, 1, 9, 5, 6, 4, 6, 9, 7, 8, 2, 6, 3, 1, 4,
                8, 1, 2, 7, 1, 2, 7, 1, 4, 5, 2, 8, 9, 9, 3, 8, 7, 7, 1, 0, 7, 1, 9, 5, 7, 5, 3, 3,
                6, 3, 7, 0, 8, 2, 5, 1, 1, 8, 4, 8, 1, 0, 2, 3, 2, 5, 7, 4, 2, 7, 4, 8, 2, 3, 3, 7,
                8, 9, 7, 3, 7, 9, 7, 6, 0, 6, 5, 7, 9, 7, 9, 9, 9, 5, 6, 8, 4, 9, 4, 2, 1, 7, 7, 2,
                8, 3, 1, 4, 5, 1, 4, 2, 6, 3, 4, 5, 2, 3, 9, 2, 7, 9, 6, 2, 3, 6, 3, 7, 2, 2, 3, 5,
                5, 1, 1, 2, 4, 0, 6, 9, 2, 2, 3, 2, 1, 3, 2, 4, 8, 8, 5, 4, 9, 3, 2, 9, 3, 2, 5, 7,
                8, 8, 9, 6, 5, 6, 2, 9, 1, 7, 3, 8, 4, 2, 0, 3, 4, 3, 7, 9, 7, 5, 8, 5, 4, 0, 9, 9,
                1, 0, 8, 0, 8, 6, 3, 0, 0, 8, 2, 3, 7, 8, 9, 7, 8, 4, 2, 0, 1, 9, 4, 9, 1, 1, 5, 4,
                2, 4, 0, 7, 1, 4, 2, 4, 1, 1, 2, 5, 4, 3, 8, 7, 6, 1, 1, 4, 2, 8, 8, 0, 6, 1, 8, 7,
                1, 0, 3, 2, 4, 5, 9, 2, 8, 6, 6, 6,
            ];
            for i in arr.into_iter() {
                lru.insert(&i.to_le_bytes(), i);
            }
        }

        #[test]
        fn test_insert() {
            let mut lru = LRUCache::new(10);
            for _i in 0..5000000_usize {
                let mut rng = rand::thread_rng();
                let n: usize = rng.gen::<usize>() % 100;
                // let n = if i / 10_usize > 0 {
                //     i % 10_usize + 1
                // } else {
                //     i
                // };
                print!("{n},");
                lru.insert(&n.to_le_bytes(), n);
            }
        }
    }
}
