use crate::hash::Hashable;

pub trait Set<T> {
    fn contains(&self, element: T) -> bool;
}

pub trait MutSet<T>: Set<T> {
    fn add(&mut self, element: T) -> bool;
    fn remove(&mut self, element: T) -> bool;
}

struct Node<T> {
    item: T,
    next: Link<T>,
}

type Link<T> = Option<Box<Node<T>>>;

impl<T> Node<T> {
    fn insert(at: &mut Link<T>, item: T) {
        let new_node = Node { item, next: at.take() };
        *at = Some(Box::new(new_node));
    }
    fn remove(at: &mut Link<T>) -> Result<T, &'static str> {
        match at.take() {
            Some(node) => {
                *at = node.next;
                Ok(node.item)
            },
            None => Err("cannot remove from empty list"),
        }
    }
}

impl<T: Hashable> Node<T> {
    fn find(from: &Link<T>, key: u64) -> (&Link<T>, bool) {
        match from {
            Some(node) if node.hash() < key => Self::find(&node.next, key),
            Some(node) if node.hash() == key => (from, true),
            _ => (from, false),
        }
    }
    fn find_mut(from: &mut Link<T>, key: u64) -> (&mut Link<T>, bool) {
        match from {
            Some(node) if node.hash() < key => {},
            Some(node) if node.hash() == key => return (from, true),
            _ => return (from, false),
        }
        match from {
            Some(node) if node.hash() < key => {
                Self::find_mut(&mut node.next, key)
            },
            _ => panic!(),
        }
    }
    // requires Polonius (NLL problem case #3)
    // fn find_mut(from: &mut Link<T>, key: u64) -> (&mut Link<T>, bool) {
    //     match from {
    //         Some(node) if node.hash() < key => {
    //             Self::find_mut(&mut node.next, key)
    //         },
    //         Some(node) if node.hash() == key => (from, true),
    //         _ => (from, false),
    //     }
    // }
}

impl<T: Hashable> Hashable for Node<T> {
    fn hash(&self) -> u64 { self.item.hash() }
}

pub struct SeqListSet<T: Hashable> {
    head: Link<T>,
}

impl<T: Hashable> SeqListSet<T> {
    pub fn new() -> Self {
        SeqListSet { head: None }
    }
}

impl<T: Hashable> Set<T> for SeqListSet<T> {
    fn contains(&self, element: T) -> bool {
        let (_node, present) = Node::find(&self.head, element.hash());
        present
    }
}

impl<T: Hashable> MutSet<T> for SeqListSet<T> {
    fn add(&mut self, element: T) -> bool {
        let (node, present) = Node::find_mut(&mut self.head, element.hash());
        if !present { Node::insert(node, element); }
        !present
    }
    fn remove(&mut self, element: T) -> bool {
        let (node, present) = Node::find_mut(&mut self.head, element.hash());
        if present { assert!(Node::remove(node).is_ok()); }
        present
    }
}
