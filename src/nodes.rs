//! Default Node kinds and traits for declaring your own
//!
//! To create your own node, implement the `NodeKind` trait and declare your own constructor.
//! ```rust
//! use rvsdg::{node_kind_impl, Context, Input, Output};
//!
//! #[derive(Debug, Clone)]
//! struct Increment {}
//! node_kind_impl!(Increment, "inc");
//!
//! fn add_increment_node(ctx: &mut Context) -> (Input<Increment>, Output<Increment>) {
//!     let node = ctx.add_node(|_, _| Increment {});
//!     let input = ctx.add_input(node);
//!     let output = ctx.add_output(node);
//!     (input, output)
//! }
//! ```
//!
//! For defining new advanced node kinds, see [`Context::node_hooks_mut`]

use crate::{
    Argument, Context, Input, Origin, Output, Result, User, binop_node_kind_impl, id,
    node_kind_impl,
};
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
        impl $crate::NodeKind for $ty {
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

        impl $crate::BinOpKind for $ty {
            fn symbol() -> &'static str {
                $kind
            }

            fn new() -> Self {
                Self {}
            }
        }
    };
}

pub trait InputOutForwarding: NodeKind {
    fn as_result_offset() -> u32;
    fn as_output_offset() -> u32;
}

impl InputOutForwarding for DoWhile {
    fn as_result_offset() -> u32 {
        1
    }

    fn as_output_offset() -> u32 {
        0
    }
}

pub trait ResultOutputForwarding: NodeKind {}

impl ResultOutputForwarding for GlobalV {}
impl ResultOutputForwarding for Switch {}
impl ResultOutputForwarding for RecEnv {}
impl ResultOutputForwarding for DoWhile {}

impl Context {
    pub(crate) fn move_to_new_recenv(
        &mut self,
        [origin_lambda, user_lambda]: [id::Node<Lambda>; 2],
    ) {
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

    /// Retrieve the `Result` that will be mapped to the given `input`.
    ///
    /// Available for `DoWhile` (theta Θ) nodes.
    pub fn input_as_result<N: InputOutForwarding>(&self, input: Input<N>) -> Result {
        let region = self.only_child_region(input.node.id);
        let offset = N::as_result_offset();
        Result {
            region,
            id: id::Result::from_u32(input.id.as_u32() + offset),
        }
    }

    /// Since DoWhile (theta Θ) nodes must have at least outputs and results that match the
    /// nodes inputs, its possible to retrieve an corresponding Output node from an Input.
    pub fn input_as_output(&self, input: Input<DoWhile>) -> Output<DoWhile> {
        input.node.output(id::Output::from_u32(input.id.as_u32()))
    }

    /// Since DoWhile (theta Θ) nodes must have at least outputs and results that match the
    /// nodes inputs, its possible to retrieve an corresponding Input node from an Output.
    pub fn output_as_input(&self, output: Output<DoWhile>) -> Input<DoWhile> {
        output.node.input(id::Input::from_u32(output.id.as_u32()))
    }

    /// Retrieve the input that maps to `argument`.
    ///
    /// Returns None if this argument is not mapped to an input, such as for Translation Unit (Omega ω)
    /// or lambda function parameter arguments.
    pub fn try_argument_as_input(&self, argument: Argument) -> Option<Input<id::AnyNode>> {
        let node_id = self.regions[argument.region].container_node;

        // Omega nodes can not forward arguments as inputs
        self.node(node_id).region?;

        let offset = self.node(node_id).input_to_argument_offset;
        argument
            .id
            .as_u32()
            .checked_sub_signed(offset)
            .map(id::Input::from_u32)
            .map(|id| Input {
                id,
                node: id::Node::new(node_id),
            })
    }

    /// Get the argument in `region` that maps to the given `input`
    ///
    /// For Lambda nodes, this will be offset by the amount of function parameter arguments.
    /// For Switch nodes, this will be offset by 1 as the predicate is not forwarded as an argument to the regions.
    ///
    /// PANICS: If `region` must be direct child of the `input` node.
    pub fn input_as_argument<N>(&self, region: id::Region, input: Input<N>) -> Argument {
        self.try_input_as_argument(region, input).unwrap()
    }

    /// Get the argument in `region` that maps to the given `input`
    ///
    /// Returns `None` if this input isn't mapped to any argument (such as Switch node predicate)
    pub fn try_input_as_argument<N>(
        &self,
        region: id::Region,
        input: Input<N>,
    ) -> Option<Argument> {
        let offset = self.node(input.node.id).input_to_argument_offset;
        let arg = id::Argument::from_u32(input.id.as_u32().checked_add_signed(offset)?);

        #[cfg(debug_assertions)]
        {
            let mut is_child = false;
            let mut iter = self.regions(input.node.id);
            while let Some(r) = iter.next(&self.region_id_pool) {
                is_child |= r == region;
                assert!(
                    self.regions[r].arguments.get(arg).is_some(),
                    "input was not forwarded to {} in regions of this node kind",
                    arg
                );
            }
            if !is_child {
                panic!("`{region}` is not a direct child of `{}`", input.node);
            }
        }

        Some(region.argument(arg))
    }

    pub fn result_as_output<K: ResultOutputForwarding>(&self, result: Result) -> Output<K> {
        let container = self.regions[result.region].container_node;
        let offset = self.node(container).output_to_result_offset;

        let output = id::Output::from_u32(result.id.as_u32().checked_sub_signed(offset).unwrap());
        id::Node::new(container).output(output)
    }

    pub fn output_as_result<K: ResultOutputForwarding>(
        &self,
        region: id::Region,
        output: Output<K>,
    ) -> Result {
        let offset = self.node(output.node.id).output_to_result_offset;
        let id = id::Result::from_u32(output.id.as_u32().checked_add_signed(offset).unwrap());

        assert_eq!(self.regions[region].container_node, output.node.id);

        region.result(id)
    }
}
