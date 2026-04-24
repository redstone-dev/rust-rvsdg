use crate::{Context, Origin, User, binop_node_kind_impl, id, node_kind_impl};
use std::any::Any;

pub trait NodeKind: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn node_type(&self) -> &str;
}

/// Only set temporarily before we've initialised a node
#[derive(Debug, Clone)]
pub struct Uninitialized;
node_kind_impl!(Uninitialized, "uninitialized");

#[derive(Debug, Clone)]
pub struct Apply {}
node_kind_impl!(Apply, "apply");

#[derive(Debug, Clone)]
pub struct DoWhile {}
node_kind_impl!(DoWhile, "theta");

#[derive(Debug, Clone)]
pub struct GlobalV {}
node_kind_impl!(GlobalV, "delta");

#[derive(Debug, Clone)]
pub struct Lambda {}
node_kind_impl!(Lambda, "lambda");

#[derive(Debug, Clone)]
pub struct TranslationUnit {}
node_kind_impl!(TranslationUnit, "omega");

#[derive(Debug, Clone)]
pub struct Number(pub i128);
node_kind_impl!(Number, "number");

#[derive(Debug, Clone)]
pub struct Switch;
node_kind_impl!(Switch, "gamma");

#[derive(Debug, Clone)]
pub struct Placeholder(pub &'static str);
node_kind_impl!(Placeholder, "placeholder");

#[derive(Debug, Clone)]
pub struct RecEnv {}
node_kind_impl!(RecEnv, "phi");

#[derive(Debug, Clone)]
pub struct Add {}
binop_node_kind_impl!(Add, "+");

#[derive(Debug, Clone)]
pub struct Sub {}
binop_node_kind_impl!(Sub, "-");

#[derive(Debug, Clone)]
pub struct Mul {}
binop_node_kind_impl!(Mul, "*");

#[derive(Debug, Clone)]
pub struct Div {}
binop_node_kind_impl!(Div, "/");

#[derive(Debug, Clone)]
pub struct Rem {}
binop_node_kind_impl!(Rem, "%");

#[derive(Debug, Clone)]
pub struct LessThan {}
binop_node_kind_impl!(LessThan, "<");

#[derive(Debug, Clone)]
pub struct GreaterThan {}
binop_node_kind_impl!(GreaterThan, ">");

#[derive(Debug, Clone)]
pub struct LessThanInclusive {}
binop_node_kind_impl!(LessThanInclusive, "<=");

#[derive(Debug, Clone)]
pub struct GreaterThanInclusive {}
binop_node_kind_impl!(GreaterThanInclusive, ">=");

#[derive(Debug, Clone)]
pub struct BitAnd {}
binop_node_kind_impl!(BitAnd, "|");

#[derive(Debug, Clone)]
pub struct BitOr {}
binop_node_kind_impl!(BitOr, "&");

#[derive(Debug, Clone)]
pub struct BitXOr {}
binop_node_kind_impl!(BitXOr, "^");

#[derive(Debug, Clone)]
pub struct BitNot {}
binop_node_kind_impl!(BitNot, "!");

#[derive(Debug, Clone)]
pub struct ShiftLeft {}
binop_node_kind_impl!(ShiftLeft, "<<");

#[derive(Debug, Clone)]
pub struct ShiftRight {}
binop_node_kind_impl!(ShiftRight, ">>");

#[derive(Debug, Clone)]
pub struct Eq {}
binop_node_kind_impl!(Eq, "==");

#[derive(Debug, Clone)]
pub struct NotEq {}
binop_node_kind_impl!(NotEq, "!=");

pub trait BinOpKind: NodeKind {
    fn symbol() -> &'static str;
    fn new() -> Self;
}

#[macro_export]
macro_rules! node_kind_impl {
    ($ty:ty, $kind:literal) => {
        impl NodeKind for $ty {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn node_type(&self) -> &str {
                $kind
            }
        }
    };
}

#[macro_export]
macro_rules! binop_node_kind_impl {
    ($ty:ty, $kind:literal) => {
        node_kind_impl!($ty, $kind);

        impl BinOpKind for $ty {
            fn symbol() -> &'static str {
                $kind
            }

            fn new() -> Self {
                Self {}
            }
        }
    };
}

impl Context {
    pub fn move_to_new_recenv(&mut self, [origin_lambda, user_lambda]: [id::Node<Lambda>; 2]) {
        let env = self.add_recenv_node();
        let env_region = self.only_child_region(env.id);

        // Disconnect all connections to these these lambdas
        let mut disconnected: Vec<(Origin, User)> = vec![];
        self.for_each_edge(origin_lambda.id, |origin, user| {
            disconnected.push((origin, user))
        });
        self.for_each_edge(user_lambda.id, |origin, user| {
            disconnected.push((origin, user))
        });

        self.move_node(origin_lambda.id, env_region);
        self.move_node(user_lambda.id, env_region);

        // Re-create all the connections we disconnected
        for (origin, user) in disconnected {
            self.connect(origin, user);
        }
    }
}
