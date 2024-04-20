use std::fmt;
use std::option::Option;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

pub struct IntoIter<T> {
    cur: Option<Box<Node<T>>>
}

pub struct Iter<'a, T> {
    cur: &'a Option<Box<Node<T>>>
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        match self.cur {
            Some(node) => {
                self.cur = &node.next;
                Some(&node.value)
            },
            None => None
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self.cur.take() {
            Some(node) => {
                self.cur = node.next;
                Some(node.value)
            },
            None => None
        }
    }
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value, next}
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        let node;
        if self.next.is_none() {
            node = Self::new(self.value.clone(), None);
        }
        else {
            node = Self::new(self.value.clone(), self.next.clone());
        }
        node
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}

impl<T: fmt::Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        let mut new = Self::new();
        new.head = self.head.clone();
        new.size = self.size;

        new
    }
}

impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        
        let mut head = &self.head;
        let mut other_head = &other.head;

        while let (Some(cur), Some(other_cur)) = (head, other_head) {
            if cur.value != other_cur.value {
                return false;
            }
            head = &cur.next;
            other_head = &other_cur.next;
        }
        
        true
    }
}

impl<T> IntoIterator for LinkedList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter { cur: self.head.take() }
    }
}

impl<'a, T: Clone> IntoIterator for &'a LinkedList<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        Iter { cur: &self.head }        
    }
}