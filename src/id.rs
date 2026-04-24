//! Raw ID types mapping to vectors in [`crate::Context`]

use std::fmt;
use std::marker::PhantomData;

use cranelift_entity::entity_impl;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnyNode(u32);
entity_impl!(AnyNode, "node");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region(u32);
entity_impl!(Region, "region");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Argument(u32);
entity_impl!(Argument, "a");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Result(u32);
entity_impl!(Result, "r");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input(pub(crate) u32);
entity_impl!(Input, "i");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Output(pub(crate) u32);
entity_impl!(Output, "o");

#[derive(Debug, Hash)]
pub struct Node<K> {
    pub id: AnyNode,
    _kind: PhantomData<K>,
}

impl<K> Clone for Node<K> {
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}
impl<K> Copy for Node<K> {}

impl<K> PartialEq for Node<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<K> Eq for Node<K> {}

impl<K> Node<K> {
    pub(super) fn new(id: AnyNode) -> Self {
        Self {
            id,
            _kind: PhantomData,
        }
    }
}

impl From<AnyNode> for Node<AnyNode> {
    fn from(id: AnyNode) -> Self {
        Node::new(id)
    }
}

impl<K> fmt::Display for Node<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.id.fmt(f)
    }
}

impl Region {
    pub fn argument(self, id: Argument) -> super::Argument {
        super::Argument { region: self, id }
    }

    pub fn result(self, id: Result) -> super::Result {
        super::Result { region: self, id }
    }
}

impl<K> Node<K> {
    pub fn input(self, id: Input) -> super::Input<K> {
        super::Input { id, node: self }
    }

    pub fn output(self, id: Output) -> super::Output<K> {
        super::Output { id, node: self }
    }
}
