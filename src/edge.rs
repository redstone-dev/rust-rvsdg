use super::{Context, id};
use std::fmt;

/// An user port
#[derive(Clone, PartialEq, Eq, Debug, Copy, Hash)]
pub enum User {
    Input(Input<id::AnyNode>),
    Result(Result),
}

/// An origin port
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Origin {
    Output(Output<id::AnyNode>),
    Argument(Argument),
}

/// Input `id` belonging to `node`
#[derive(Hash)]
pub struct Input<K> {
    pub node: id::Node<K>,
    pub id: id::Input,
}

/// Output `id` belonging to `node`
#[derive(Hash)]
pub struct Output<K> {
    pub node: id::Node<K>,
    pub id: id::Output,
}

/// Argument `id` belonging to `region`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Argument {
    pub region: id::Region,
    pub id: id::Argument,
}

/// Result `id` belonging to `region`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Result {
    pub region: id::Region,
    pub id: id::Result,
}

impl<K> Output<K> {
    pub fn upcast(self) -> Output<id::AnyNode> {
        Output {
            node: id::Node::new(self.node.id),
            id: self.id,
        }
    }
}

impl<K> Input<K> {
    pub fn upcast(self) -> Input<id::AnyNode> {
        Input {
            node: id::Node::new(self.node.id),
            id: self.id,
        }
    }
}

impl Context {
    pub(crate) fn user_associated_node(&self, user: User) -> id::AnyNode {
        match user {
            User::Input(input) => input.node.id,
            User::Result(result) => self.regions[result.region].container_node,
        }
    }
    pub(crate) fn origin_associated_node(&self, origin: Origin) -> id::AnyNode {
        match origin {
            Origin::Output(output) => output.node.id,
            Origin::Argument(argument) => self.regions[argument.region].container_node,
        }
    }

    /// Traverse nodes from `user` invoking `f` for each node `user` directly or indirectly depends on.
    pub fn visit_nodes_upwards<T, F>(&self, user: impl Into<User>, f: &mut F) -> Option<T>
    where
        F: FnMut(&Self, id::AnyNode) -> Option<T>,
    {
        let user = user.into();
        match user {
            User::Input(input) => {
                if let Some(t) = f(self, input.node.id) {
                    return Some(t);
                }

                let onode = self
                    .get_user(input)
                    .map(|origin| self.origin_associated_node(origin))?;

                self.inputs(onode)
                    .map(User::from)
                    .find_map(|user| self.visit_nodes_upwards(user, f))
            }
            User::Result(result) => {
                let node_id = self.regions[result.region].container_node;

                if let Some(t) = f(self, node_id) {
                    return Some(t);
                }

                self.inputs(node_id)
                    .map(User::from)
                    .find_map(|user| self.visit_nodes_upwards(user, f))
            }
        }
    }
}

impl<K> Clone for Input<K> {
    fn clone(&self) -> Self {
        Input {
            node: self.node,
            id: self.id,
        }
    }
}
impl<K> Clone for Output<K> {
    fn clone(&self) -> Self {
        Output {
            node: self.node,
            id: self.id,
        }
    }
}

impl<K> Copy for Input<K> {}
impl<K> Copy for Output<K> {}

impl<K> PartialEq for Output<K> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.id == other.id
    }
}
impl<K> Eq for Output<K> {}

impl<K> PartialEq for Input<K> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.id == other.id
    }
}
impl<K> Eq for Input<K> {}

impl<K> From<Input<K>> for User {
    fn from(input: Input<K>) -> Self {
        User::Input(input.upcast())
    }
}

impl From<Result> for User {
    fn from(result: Result) -> Self {
        User::Result(result)
    }
}

impl<K> From<Output<K>> for Origin {
    fn from(output: Output<K>) -> Self {
        Origin::Output(output.upcast())
    }
}

impl From<Argument> for Origin {
    fn from(argument: Argument) -> Self {
        Origin::Argument(argument)
    }
}

impl<K> fmt::Display for Input<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.node.id, self.id)
    }
}

impl<K> fmt::Display for Output<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.node.id, self.id)
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.region, self.id)
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.region, self.id)
    }
}

impl<K> fmt::Debug for Input<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.node.id, self.id)
    }
}

impl<K> fmt::Debug for Output<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.node.id, self.id)
    }
}

impl fmt::Debug for Argument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.region, self.id)
    }
}

impl fmt::Debug for Result {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}·{}", self.region, self.id)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Origin::Output(output) => output.fmt(f),
            Origin::Argument(argument) => argument.fmt(f),
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            User::Input(input) => input.fmt(f),
            User::Result(result) => result.fmt(f),
        }
    }
}
