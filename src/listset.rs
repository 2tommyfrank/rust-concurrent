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
    next: Link<T>
}

type Link<T> = Option<Box<Node<T>>>;

impl<T: Hashable> Hashable for Node<T> {
    fn hash(&self) -> u64 { self.item.hash() }
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
            Some(node) if node.hash() < key => {
                Self::find_mut(&mut node.next, key)
            },
            Some(node) if node.hash() == key => (from, true),
            _ => (from, false),
        }
    }
}

pub struct SeqListSet<T> {
    head: Link<T>
}

impl<T> SeqListSet<T> {
    pub fn new() -> Self {
        SeqListSet { head: None }
    }
}

impl<T: Hashable> Set<T> for SeqListSet<T> {
    fn contains(&self, element: T) -> bool {
        Node::find(&self.head, element.hash()).1
    }
}

impl<T: Hashable> MutSet<T> for SeqListSet<T> {
    fn add(&mut self, element: T) -> bool {
        let (node, present) = Node::find_mut(&mut self.head, element.hash());
        if !present {
            let new_node = Node { item: element, next: node.take() };
            *node = Some(Box::new(new_node));
        }
        !present
    }
    fn remove(&mut self, element: T) -> bool {
        let (node, present) = Node::find_mut(&mut self.head, element.hash());
        if present {
            // if find_mut().1 is true, then find_mut().0 should be Some
            *node = node.take().expect("find_mut violated contract").next;
        }
        present
    }
}
