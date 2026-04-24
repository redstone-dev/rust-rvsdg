//! # rust-rvsdg
//!
//! This crate provides a mostly type-safe way of constructing and analyzing a [RVSDG](https://arxiv.org/abs/1912.05036)
//!
//! For developer familiarity, most node kinds have been renamed.
//!
//! * Gamma -> Switch
//! * Theta -> DoWhile
//! * Delta -> GlobalV
//! * Phi   -> RecEnv
//! * Omega -> TranslationUnit
//!
//! RecEnv nodes are created automatically when cyclic Lambda connections are made.
//!
//! # Constructing an RVSDG
//!
//! ```rust
//! use rvsdg::{Context, nodes::Add};
//!
//! fn main() {
//!     let mut ctx = Context::new("my testing graph");
//!
//!     let (f_output, f_region) = ctx.add_lambda_node();
//!     ctx.in_region(f_region, |ctx| {
//!         let one = ctx.add_number_node(1);
//!         let two = ctx.add_number_node(2);
//!
//!         let ([x, y], addition) = ctx.add_binop_node::<Add>();
//!
//!         ctx.connect(one, x);
//!         ctx.connect(two, y);
//!
//!         let returned = ctx.add_result();
//!
//!         ctx.connect(addition, returned);
//!     });
//!
//!     let apply_input = ctx.add_apply_node();
//!     ctx.connect(f_output, apply_input);
//! }
//! ```
//!
//! # Status
//!
//! Currently this crate is in an highly experimental phase.
//!
//! # Exporting an RVSDG
//!
//! If [rvsdg-viewer](https://github.com/phate/rvsdg-viewer) is installed,
//! you can use [`Context::open_rvsdg_viewer`].

use cranelift_entity::{EntityList, ListPool, PrimaryMap, SecondaryMap};
use std::io::Write;
use tracing::{info, trace};

mod edge;
mod entity_iterator;
pub use edge::{Argument, Input, Origin, Output, Result, User};
pub use entity_iterator::EntityIter;
pub mod id;
pub mod nodes;
use nodes::*;
pub use nodes::{BinOpKind, NodeKind};
#[cfg(test)]
mod tests;
mod xml;

/// The context for a whole translation unit and the core struct of this crate.
#[derive(Debug)]
pub struct Context {
    nodes: PrimaryMap<id::AnyNode, Node>,
    regions: PrimaryMap<id::Region, Region>,

    symbols: SecondaryMap<id::AnyNode, String>,

    pub node_id_pool: ListPool<id::AnyNode>,
    pub region_id_pool: ListPool<id::Region>,

    pub region: id::Region,
}

#[derive(Debug)]
pub(crate) struct Node {
    // Self-referential node id
    id: id::AnyNode,
    region: Option<id::Region>,

    inputs: PrimaryMap<id::Input, Option<Origin>>,
    outputs: PrimaryMap<id::Output, Vec<User>>,

    input_to_argument_offset: i32,
    output_to_result_offset: i32,

    regions: EntityList<id::Region>,

    hooks: NodeHooks,

    kind: Box<dyn NodeKind + Send + Sync>,
}

/// Event handlers for changes being made to the node or its contained regions.
///
/// Mainly used for ensuring inputs/outputs are correctly mapped to the nodes arguments/results
/// according to the node kinds requirements.
#[derive(Debug)]
pub struct NodeHooks {
    /// Default: Forward this input as an argument to each contained region
    pub on_input: for<'a> fn(&'a mut Context, Input<id::AnyNode>),
    /// Default: Forward this output as an result to each contained region
    pub on_output: for<'a> fn(&'a mut Context, Output<id::AnyNode>),

    /// Default: Panic, arguments may only be added implicitly by new inputs being added to the node
    pub on_argument: for<'a> fn(&'a mut Context, Argument),
    /// Default: Panic, results may only be added implicitly by new outputs being added to the node
    pub on_result: for<'a> fn(&'a mut Context, Result),
}

impl Default for NodeHooks {
    fn default() -> Self {
        Self {
            on_input: |ctx, input| {
                let mut regions = ctx.regions(input.node.id);
                while let Some(region) = regions.next(&ctx.region_id_pool) {
                    ctx.regions[region].arguments.push(vec![]);
                }
            },
            on_output: |ctx, output| {
                let mut regions = ctx.regions(output.node.id);
                while let Some(region) = regions.next(&ctx.region_id_pool) {
                    ctx.regions[region].results.push(None);
                }
            },

            on_argument: |_, _| {
                panic!("node kind can not take explicit arguments in region");
            },
            on_result: |_, _| {
                panic!("node kind can not take explicit results in region");
            },
        }
    }
}

#[derive(Debug)]
struct Region {
    container_node: id::AnyNode,

    arguments: PrimaryMap<id::Argument, Vec<User>>,
    results: PrimaryMap<id::Result, Option<Origin>>,

    nodes: EntityList<id::AnyNode>,
}

impl Context {
    /// Initialize a new RVSDG translation unit.
    ///
    /// Sets the current region to the region of an implicitly declared `TranslationUnit` (omega ω) node.
    pub fn new(unit_symbol: impl Into<String>) -> Self {
        let mut ctx = Context {
            nodes: PrimaryMap::new(),
            regions: PrimaryMap::new(),
            symbols: SecondaryMap::new(),
            node_id_pool: ListPool::new(),
            region_id_pool: ListPool::new(),
            region: id::Region::from_u32(0),
        };

        let omega = ctx.add_omega();
        ctx.region = ctx.only_child_region(omega.id);
        ctx.add_symbol(omega.id, unit_symbol);
        ctx.node_mut(omega.id).region = None;

        ctx
    }

    /// Perform `f` while [`Context::region`] is set to `region`.
    pub fn in_region<T>(&mut self, region: id::Region, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.region;
        self.region = region;
        let v = f(self);
        self.region = previous;
        v
    }

    /// Returns an non-borrowing iterator over regions of `node`.
    ///
    /// Use [`EntityIter::next`] with `Context::region_id_pool` as parameter to progress.
    pub fn regions(&self, node: id::AnyNode) -> EntityIter<id::Region> {
        EntityIter::from(self.nodes[node].regions.clone())
    }

    pub fn inputs(
        &self,
        node_id: id::AnyNode,
    ) -> impl Iterator<Item = Input<id::AnyNode>> + 'static {
        self.nodes[node_id].inputs.keys().map(move |id| Input {
            id,
            node: id::Node::new(node_id),
        })
    }
    pub fn outputs(
        &self,
        node_id: id::AnyNode,
    ) -> impl Iterator<Item = Output<id::AnyNode>> + 'static {
        self.nodes[node_id].outputs.keys().map(move |id| Output {
            id,
            node: id::Node::new(node_id),
        })
    }
    pub fn arguments(&self, region: id::Region) -> impl Iterator<Item = Argument> + 'static {
        self.regions[region]
            .arguments
            .keys()
            .map(move |id| Argument { region, id })
    }
    pub fn results(&self, region: id::Region) -> impl Iterator<Item = Result> + 'static {
        self.regions[region]
            .results
            .keys()
            .map(move |id| Result { region, id })
    }

    /// Returns an iterator of all nodes that are direct children of `region`.
    pub fn nodes(&self, region: id::Region) -> impl Iterator<Item = id::AnyNode> {
        self.regions[region]
            .nodes
            .as_slice(&self.node_id_pool)
            .iter()
            .copied()
    }

    /// Create a new empty node of any kind and manually initialize it with `init`
    ///
    /// WARNING: Only use this for constructing your own custom nodes. The nodes defined in
    /// `crate::nodes` **must** be created by their corresponding `add_` prefixed methods.
    ///
    /// See:
    ///  * [`Context::add_binop_node`]
    ///  * [`Context::add_lambda_node`]
    ///  * [`Context::add_switch_node`]
    ///  * [`Context::add_globalv_node`]
    ///  * [`Context::add_dowhile_node`]
    ///  * [`Context::add_number_node`]
    ///  * [`Context::add_apply_node`]
    pub fn add_node<F, K: NodeKind>(&mut self, init: F) -> id::Node<K>
    where
        F: FnOnce(&mut Self, id::Node<K>) -> K,
    {
        let node = Node {
            kind: Box::new(Uninitialized),
            region: Some(self.region),
            inputs: PrimaryMap::new(),
            outputs: PrimaryMap::new(),
            regions: EntityList::new(),
            id: self.nodes.next_key(),
            hooks: NodeHooks::default(),
            input_to_argument_offset: 0,
            output_to_result_offset: 0,
        };

        let node = id::Node::new(self.nodes.push(node));

        if node.id == id::AnyNode::from_u32(0) {
            // edge-cased to not add a region for root (normally omega) node
            trace!("adding {node}");
            self.node_mut(node.id).region = None;
        } else {
            trace!("adding {node} in {}", self.region);
            self.regions[self.region]
                .nodes
                .push(node.id, &mut self.node_id_pool);
        }

        let kind = init(self, node);
        self.node_mut(node.id).kind = Box::new(kind);

        node
    }

    pub fn add_symbol(&mut self, node: id::AnyNode, sym: impl Into<String>) {
        self.symbols[node] = sym.into();
    }

    /// SAFETY: This function allows you to violate the rules of the RVSDG. To create regions safely,
    /// use the safe functions such as [`Context::add_switch_branch`]
    pub unsafe fn add_region(
        &mut self,
        container: id::AnyNode,
        arguments: u32,
        results: u32,
    ) -> id::Region {
        let region = self.regions.push(Region {
            container_node: container,
            arguments: (0..arguments).map(|_| vec![]).collect(),
            results: (0..results).map(|_| None).collect(),
            nodes: EntityList::new(),
        });

        self.nodes[container]
            .regions
            .push(region, &mut self.region_id_pool);

        region
    }

    /// Get the only singular region. Panics if there's not exactly one region
    pub fn only_child_region(&self, node: id::AnyNode) -> id::Region {
        let mut regions = self.regions(node);
        let region = regions
            .next(&self.region_id_pool)
            .expect("node does not have any regions");
        if regions.next(&self.region_id_pool).is_some() {
            panic!("`region` can not be called for node with multiple regions",)
        }
        region
    }

    pub fn parent_region(&self, node: id::AnyNode) -> Option<id::Region> {
        self.node(node).region
    }

    fn node(&self, node_id: id::AnyNode) -> &Node {
        &self.nodes[node_id]
    }
    fn node_mut(&mut self, node_id: id::AnyNode) -> &mut Node {
        &mut self.nodes[node_id]
    }

    pub fn node_hooks_mut(&mut self, node: id::AnyNode, mut f: impl FnMut(&mut NodeHooks)) {
        f(&mut self.node_mut(node).hooks)
    }

    fn add_omega(&mut self) -> id::Node<TranslationUnit> {
        let node = self.add_node(|ctx, node| unsafe {
            ctx.add_region(node.id, 0, 0);
            TranslationUnit {}
        });

        // Allow adding results
        self.node_mut(node.id).hooks.on_result = |_, _| {};

        // Allow adding arguments
        self.node_mut(node.id).hooks.on_argument = |_, _| {};

        node
    }

    /// Create a BinOpKind simple node.
    ///
    /// BinOpKind nodes have two inputs, and one output.
    pub fn add_binop_node<N: BinOpKind>(&mut self) -> ([Input<N>; 2], Output<N>) {
        let node = self.add_node(|_, _| N::new());
        self.add_symbol(node.id, N::symbol());

        let x = self.add_input(node);
        let y = self.add_input(node);

        let out = self.add_output(node);

        ([x, y], out)
    }

    /// Create a lambda node.
    ///
    /// Lambda nodes have a singular region.
    /// Lambda nodes have a singular output, representing itself.
    /// Lambda nodes may have arguments manually declared in their region as long as no inputs have been added.
    pub fn add_lambda_node(&mut self) -> (Output<Lambda>, id::Region) {
        let node = self.add_node(|ctx, node| unsafe {
            ctx.add_region(node.id, 0, 0);
            Lambda {}
        });

        self.node_hooks_mut(node.id, |hooks| {
            // Allow adding arguments
            hooks.on_argument = |ctx, arg| {
                let node_id = ctx.regions[arg.region].container_node;
                ctx.node_mut(node_id).input_to_argument_offset += 1;
            };

            // Allow adding results
            hooks.on_result = |_, _| {};

            // Don't forward outputs to region results
            hooks.on_output = |_, _| {};
        });

        let region = self.only_child_region(node.id);

        (self.add_output(node), region)
    }

    /// Create a new region in `node` which will correspond to the next number.
    pub fn add_switch_branch(&mut self, node: id::Node<Switch>) -> (id::Region, usize) {
        let i = self.node(node.id).regions.len(&self.region_id_pool);

        let n_of_arguments = self.node(node.id).inputs.len() - 1;
        let n_of_results = self.node(node.id).outputs.len();

        unsafe {
            let region = self.add_region(node.id, n_of_arguments as u32, n_of_results as u32);

            (region, i)
        }
    }

    // Create a globalv (delta) node.
    //
    // GlobalV nodes have a singular region representing the initialization of a value.
    // GlobalV nodes regions have singular results, representing the initialized values.
    // GlobalV nodes have a singular output, representing the initialized value.
    pub fn add_globalv_node(&mut self) -> (Result, Output<GlobalV>) {
        let node = self.add_node(|ctx, node| unsafe {
            ctx.add_region(node.id, 0, 1);
            GlobalV {}
        });

        let output = self.add_output(node);
        let region = self.only_child_region(node.id);
        let id = self.regions[region].results.push(None);
        let result = Result { region, id };

        (result, output)
    }

    // Create a DoWhile (theta) node.
    //
    // DoWhile nodes have a singular region that represent their loop body
    // The first region result represents the predicate.
    pub fn add_dowhile_node(&mut self) -> (Result, id::Node<DoWhile>) {
        let node = self.add_node(|_, _| DoWhile {});

        // compensate for do-while node regions not having the predicate result forwarded
        self.node_mut(node.id).output_to_result_offset = 1;

        let region = unsafe { self.add_region(node.id, 0, 1) };

        // do-while forwards inputs to arguments, results, and outputs
        self.node_mut(node.id).hooks.on_input = |ctx, input| {
            let mut regions = ctx.regions(input.node.id);
            while let Some(region) = regions.next(&ctx.region_id_pool) {
                let arg_id = ctx.regions[region].arguments.push(vec![]);
                let result_id = ctx.regions[region].results.push(None);
                debug_assert_eq!(input.id.as_u32(), arg_id.as_u32());
                debug_assert_eq!(input.id.as_u32(), result_id.as_u32() - 1);
            }
            ctx.nodes[input.node.id].outputs.push(vec![]);
        };
        // adding an output or an input are equivalent for dowhile
        self.node_mut(node.id).hooks.on_output = |ctx, output| {
            let mut regions = ctx.regions(output.node.id);
            while let Some(region) = regions.next(&ctx.region_id_pool) {
                let arg_id = ctx.regions[region].arguments.push(vec![]);
                let result_id = ctx.regions[region].results.push(None);
                debug_assert_eq!(output.id.as_u32(), arg_id.as_u32());
                debug_assert_eq!(output.id.as_u32(), result_id.as_u32() - 1);
            }
        };

        let result = Result {
            region,
            id: id::Result::from_u32(0),
        };
        trace!("adding predicate result {result}");

        (result, node)
    }

    // Create a RecEnv (phi) node.
    //
    // RecEnv nodes have a singular region, containing lambdas that can be mutually recursive.
    // RecEnv nodes have an output for each contained lambda.
    pub fn add_recenv_node(&mut self) -> id::Node<RecEnv> {
        let node = self.add_node(|ctx, node| unsafe {
            ctx.add_region(node.id, 0, 0);
            RecEnv {}
        });

        self.node_mut(node.id).hooks.on_output =
            |_, _| panic!("can not add outputs to recenv node");

        node
    }
    /// Create a switch (gamma) node.
    ///
    /// Switch nodes first input is a predicate which determines which region is under evaluation.
    /// Switch node regions have the same amount of results as the node has outputs.
    /// Switch node region results are mapped to switch node outputs.
    ///
    /// See [`Context::add_switch_branch`]
    pub fn add_switch_node(&mut self) -> (Input<Switch>, id::Node<Switch>) {
        let node = self.add_node(|_, _| Switch);

        // don't forward predicate to regions
        self.node_mut(node.id).hooks.on_input = |ctx, input| {
            if input.id != id::Input(0) {
                (NodeHooks::default().on_input)(ctx, input)
            }
        };

        // compensate for switch node regions not having the predicate input forwarded
        self.node_mut(node.id).input_to_argument_offset = -1;

        let input = self.add_input(node);

        (input, node)
    }

    // Create a number node.
    //
    // Number nodes have no regions and have one output representing the numeric value.
    pub fn add_number_node(&mut self, n: i128) -> Output<Number> {
        let node = self.add_node(|_, _| Number(n));
        self.add_symbol(node.id, n.to_string());
        self.add_output(node)
    }

    // Create an apply node.
    //
    // Apply nodes take a lambda as first input. The rest of the inputs will be mapped to the
    // argument for the lambda's region.
    pub fn add_apply_node(&mut self) -> Input<Apply> {
        let node = self.add_node(|_, _| Apply {});
        self.add_symbol(node.id, "apply");
        self.add_input(node)
    }

    // Convenience method for [`add_apply_node`]
    pub fn add_and_connect_apply_node<const N: usize>(
        &mut self,
        f: impl Into<Origin>,
        params: &[impl Into<Origin> + Clone],
    ) -> [Output<Apply>; N] {
        let input = self.add_apply_node();
        self.connect(f, input);

        for p in params {
            let input = self.add_input(input.node);
            self.connect(p.clone(), input);
        }

        [(); N].map(|_| self.add_output(input.node))
    }

    // Create a placeholder node.
    //
    // Placeholder nodes have no regions and may take any amount of inputs and outputs.
    //
    // They're meant to act as a "todo" node.
    pub fn add_placeholder_node<const N: usize, const ON: usize>(
        &mut self,
        name: &'static str,
        inputs: [Origin; N],
    ) -> [Output<Placeholder>; ON] {
        let node = self.add_node(|_, _| Placeholder(name));
        self.add_symbol(node.id, name);

        for origin in inputs {
            let input = self.add_input(node);
            self.connect(origin, input);
        }

        [(); ON].map(|_| self.add_output(node))
    }

    fn debug_node(&self, node: id::AnyNode) -> String {
        let sym = &self.symbols[node];
        if sym == "" {
            format!("{node}")
        } else {
            format!("{node}·{sym}")
        }
    }

    fn debug_origin(&self, origin: impl Into<Origin>) -> String {
        let origin = origin.into();
        match origin {
            Origin::Output(output) => format!("{}·{}", self.debug_node(output.node.id), output.id),
            Origin::Argument(argument) => {
                let node = self.regions[argument.region].container_node;
                format!(
                    "{}·{}·{}",
                    self.debug_node(node),
                    argument.region,
                    argument.id
                )
            }
        }
    }

    fn debug_user(&self, user: impl Into<User>) -> String {
        let user = user.into();
        match user {
            User::Input(input) => format!("{}·{}", self.debug_node(input.node.id), input.id),
            User::Result(result) => {
                let node = self.regions[result.region].container_node;
                format!("{}·{}·{}", self.debug_node(node), result.region, result.id)
            }
        }
    }

    pub fn add_input<K>(&mut self, node: id::Node<K>) -> Input<K> {
        let input = self.node_mut(node.id).inputs.push(None);
        let input = Input { id: input, node };

        trace!("added input {input} for {}", self.debug_node(node.id));

        (self.node_mut(node.id).hooks.on_input)(self, input.upcast());

        input
    }

    pub fn add_output<K>(&mut self, node: id::Node<K>) -> Output<K> {
        let output = self.node_mut(node.id).outputs.push(vec![]);
        let output = Output { id: output, node };

        (self.node_mut(node.id).hooks.on_output)(self, output.upcast());

        trace!("added output {output} for {}", self.debug_node(node.id));

        output
    }

    pub fn result_as_output<K>(&self, result: Result) -> Output<K> {
        let container = self.regions[result.region].container_node;
        let offset = self.node(container).output_to_result_offset;

        // TODO: Is this valid for all nodes? Probably not right?
        //
        // We could assert but I think its better to stop this statically
        let output = id::Output::from_u32(result.id.as_u32().checked_sub_signed(offset).unwrap());
        id::Node::new(container).output(output)
    }

    pub fn try_output_as_result<K>(&self, region: id::Region, output: Output<K>) -> Result {
        let offset = self.node(output.node.id).output_to_result_offset;
        let id = id::Result::from_u32(output.id.as_u32().checked_add_signed(offset).unwrap());

        assert_eq!(self.regions[region].container_node, output.node.id);

        region.result(id)
    }

    pub fn add_argument(&mut self) -> Argument {
        let id = self.regions[self.region].arguments.push(vec![]);
        let arg = Argument {
            id,
            region: self.region,
        };

        let node_id = self.regions[self.region].container_node;
        (self.node_mut(node_id).hooks.on_argument)(self, arg);

        trace!("added argument {arg} for {}", self.region);

        arg
    }

    pub fn add_result(&mut self) -> Result {
        let id = self.regions[self.region].results.push(None);
        let result = Result {
            id,
            region: self.region,
        };

        let node_id = self.regions[self.region].container_node;
        (self.node_mut(node_id).hooks.on_result)(self, result);

        trace!("added result {result} for {}", self.region);

        result
    }

    /// Connect `origin` to `user`, and use pathfinding in case they're of different regions to
    /// automatically form the connections needed to make `origin` available to `user`.
    ///
    /// If both the `origin` and `user` ports attach to lambdas in the same region, then `connect`
    /// may move the lambdas into a node RecEnv (phi ϕ) node - re-arranging all connections to those
    /// lambdas as-needed.
    pub fn connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) {
        let origin = origin.into();
        let user = user.into();
        match self.try_connect(origin, user) {
            Connection::RecEnv(_) | Connection::Ok => {}
            connection => panic!("invalid connection: {connection}"),
        }
    }

    /// Retrieve the `Origin` directly attached to the port `user`.
    ///
    /// # Arguments
    ///
    /// * `user` a node [`Input`] or region [`Result`]
    pub fn get_user(&self, user: impl Into<User>) -> Option<Origin> {
        match user.into() {
            User::Input(input) => self.nodes[input.node.id].inputs[input.id],
            User::Result(result) => self.regions[result.region].results[result.id],
        }
    }

    fn find_cycle(
        &self,
        [node_with_origin, node_with_user]: [id::AnyNode; 2],
    ) -> Option<Output<id::AnyNode>> {
        self.search_each_connected_input(node_with_origin, &mut |_, connection| match connection {
            Origin::Output(output) => (output.node.id == node_with_user).then(|| output),
            Origin::Argument(_) => None,
        })
    }

    /// Same as [`Context::connect`] except returns [`Connection`] result instead of panicing.
    pub fn try_connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) -> Connection {
        let mut origin = origin.into();
        let user = user.into();

        trace!("trying to connect {origin:?} -> {user:?}");

        let region_with_origin = self
            .region_containing_origin(origin)
            .expect("cannot connect output of omega node");

        let region_with_user = self.region_containing_user(user);

        if let Origin::Output(output) = origin {
            if let Some((recenv, i)) = self.is_lambda_in_recenv(output.node.id) {
                if region_with_origin == region_with_user {
                    origin = self
                        .only_child_region(recenv.id)
                        .argument(id::Argument::from_u32(i))
                        .into();
                } else {
                    origin = recenv.output(id::Output::from_u32(i)).into();
                }
                return self.try_connect(origin, user);
            }
        }

        // TODO: Is this one needed?
        // if let User::Input(input) = user {
        //     if region_with_origin != region_with_user {
        //         if let Some((recenv, i)) = self.is_lambda_in_recenv(input.node.id) {
        //             user = recenv.result(id::Result::from_u32(i)).into();
        //             return self.try_connect(origin, user);
        //         }
        //     }
        // }

        let node_with_origin = self.origin_associated_node(origin);
        let node_with_user = self.user_associated_node(user);

        if let Some(_cycle) = self.find_cycle([node_with_origin, node_with_user]) {
            if region_with_origin == region_with_user {
                if let Some([node_with_origin, node_with_user]) =
                    self.try_downcast_to_lambdas([node_with_origin, node_with_user])
                {
                    // If we're making a cyclic connection of two lambda nodes in the same region.
                    //
                    // Create a new RecEnv and move them to it.
                    self.move_to_new_recenv([node_with_origin, node_with_user]);

                    // Try again which should successfully connect this time
                    return self.try_connect(origin, user);
                }
            }

            return Connection::Cyclic;
        }

        // Find a connection path from origin to the current region
        let origin_same_region_as_user = self.fold_regions_from(
            origin,
            [region_with_user, region_with_origin],
            &mut |ctx, origin, (node, region_in_node)| {
                for input in ctx.inputs(node) {
                    let Some(argument) = ctx.try_input_as_argument(region_in_node, input) else {
                        continue;
                    };

                    if ctx.get_user(input) == Some(origin) {
                        trace!("connection already exists, returning existing argument");
                        return argument.into();
                    }
                }

                let input = ctx.add_input(id::Node::<id::AnyNode>::new(node));

                unsafe {
                    ctx.connect_same_region(origin, input.into());
                    let argument = ctx.input_as_argument(region_in_node, input);
                    argument.into()
                }
            },
        );

        match origin_same_region_as_user {
            None => Connection::NoPath(origin, user),
            Some(origin) => unsafe {
                self.connect_same_region(origin, user);
                Connection::Ok
            },
        }
    }

    fn is_lambda_in_recenv(&self, node_id: id::AnyNode) -> Option<(id::Node<RecEnv>, u32)> {
        let region = self.node(node_id).region?;

        let container = self.regions[region].container_node;

        let recenv_node = self.downcast::<RecEnv>(container)?;

        Some((
            recenv_node,
            self.regions[region]
                .nodes
                .as_slice(&self.node_id_pool)
                .iter()
                .position(|n| *n == node_id)
                .unwrap() as u32,
        ))
    }

    fn try_downcast_to_lambdas(
        &self,
        [origin, user]: [id::AnyNode; 2],
    ) -> Option<[id::Node<Lambda>; 2]> {
        self.downcast(origin)
            .and_then(|origin| self.downcast(user).map(|user| [origin, user]))
    }

    fn fold_regions_from<F, T>(
        &mut self,
        init: T,
        [from, up_to]: [id::Region; 2],
        f: &mut F,
    ) -> Option<T>
    where
        F: FnMut(&mut Self, T, (id::AnyNode, id::Region)) -> T,
    {
        if from == up_to {
            return Some(init);
        }

        let container_node = self.regions[from].container_node;

        let Some(upper_region) = self.node(container_node).region else {
            // We've reached omega without finding `origin` by traversing regions.
            //
            // Therefore a connection path is not possible.
            return None;
        };

        self.fold_regions_from(init, [upper_region, up_to], f)
            .map(|value| f(self, value, (container_node, from)))
    }

    fn search_each_connected_input<T, F>(&self, node: id::AnyNode, f: &mut F) -> Option<T>
    where
        F: FnMut(Input<id::AnyNode>, Origin) -> Option<T>,
    {
        self.inputs(node).find_map(|input| {
            let connected = self.get_user(input)?;

            if let Some(target) = f(input, connected) {
                return Some(target);
            }

            match connected {
                Origin::Output(output) => self.search_each_connected_input(output.node.id, f),
                Origin::Argument(argument) => self
                    .try_argument_as_input(argument)
                    .and_then(|input| self.search_each_connected_input(input.node.id, f)),
            }
        })
    }

    unsafe fn connect_output_to_input<K>(&mut self, output: Output<K>, input: Input<K>) {
        self.node_mut(input.node.id).inputs[input.id] = Some(output.into());
        self.node_mut(output.node.id).outputs[output.id].push(input.into());
    }

    unsafe fn connect_argument_to_result(&mut self, argument: Argument, result: Result) {
        self.regions[argument.region].arguments[argument.id].push(User::from(result));
        self.regions[result.region].results[result.id] = Some(argument.into());
    }

    unsafe fn connect_output_to_result<K>(&mut self, output: Output<K>, result: Result) {
        self.node_mut(output.node.id).outputs[output.id].push(result.into());
        self.regions[result.region].results[result.id] = Some(output.into());
    }

    unsafe fn connect_argument_to_input<K>(&mut self, argument: Argument, input: Input<K>) {
        self.regions[argument.region].arguments[argument.id].push(input.into());
        self.node_mut(input.node.id).inputs[input.id] = Some(argument.into());
    }

    /// Returns the statically typed version of a node if it downcasts to `K`.
    pub fn downcast<K: NodeKind>(&self, node_id: id::AnyNode) -> Option<id::Node<K>> {
        self.nodes[node_id]
            .kind
            .as_any()
            .downcast_ref::<K>()
            .is_some()
            .then(|| id::Node::new(node_id))
    }

    /// Connects `origin` to `user` without any checks for cycles of recenv-transformation.
    ///
    /// PANICS: If `origin` and `user` ports aren't in the same region.
    pub unsafe fn connect_same_region(&mut self, origin: Origin, user: User) {
        info!("{} -> {}", self.debug_origin(origin), self.debug_user(user));

        debug_assert_eq!(
            self.region_containing_origin(origin),
            Some(self.region_containing_user(user))
        );

        unsafe {
            match user {
                User::Input(input) => match origin {
                    Origin::Output(output) => self.connect_output_to_input(output, input),
                    Origin::Argument(argument) => self.connect_argument_to_input(argument, input),
                },
                User::Result(result) => match origin {
                    Origin::Output(output) => self.connect_output_to_result(output, result),
                    Origin::Argument(argument) => self.connect_argument_to_result(argument, result),
                },
            }
        }
    }

    fn region_containing_origin(&self, origin: Origin) -> Option<id::Region> {
        match origin {
            Origin::Output(output) => self.node(output.node.id).region,
            Origin::Argument(argument) => Some(argument.region),
        }
    }
    fn region_containing_user(&self, user: User) -> id::Region {
        match user {
            User::Input(input) => self
                .node(input.node.id)
                .region
                .expect("inputs on omega node is not possible"),
            User::Result(result) => result.region,
        }
    }

    fn move_node(&mut self, node: id::AnyNode, to: id::Region) {
        #[cfg(debug_assertions)]
        self.for_each_edge(node, |origin, user| {
            panic!("cannot move node with connection: {origin} -> {user}")
        });

        // Remove the node from the previous region
        if let Some(region) = self.node(node).region {
            let i = self.regions[region]
                .nodes
                .as_slice(&self.node_id_pool)
                .iter()
                .position(|n| *n == node)
                .unwrap();

            self.regions[region].nodes.remove(i, &mut self.node_id_pool);
        }

        // Add the node to the new region
        self.node_mut(node).region = Some(to);
        self.regions[to].nodes.push(node, &mut self.node_id_pool);

        // If the node we move to is an recenv node, also add corresponding result/argument/output
        let container = self.regions[to].container_node;
        if let Some(recenv) = self.downcast::<RecEnv>(container) {
            let Some(lambda) = self.downcast::<Lambda>(node) else {
                panic!("cannot move non-lambda to recenv node");
            };

            self.regions[to].arguments.push(vec![]);
            self.regions[to]
                .results
                .push(Some(lambda.output(id::Output::from_u32(0)).into()));
            self.nodes[recenv.id].outputs.push(vec![]);
        }
    }

    /// Calls `f` for each connection to inputs and outputs of `node`.
    pub fn for_each_edge<F>(&mut self, node: id::AnyNode, mut f: F)
    where
        F: FnMut(Origin, User),
    {
        for output in self.outputs(node) {
            for user in self.node_mut(output.node.id).outputs[output.id].drain(..) {
                f(Origin::from(output), user);
            }
        }

        for input in self.inputs(node) {
            if let Some(origin) = self.nodes[input.node.id].inputs[input.id].take() {
                f(origin, User::from(input));
            }
        }
    }

    /// Launches [`rvsdg-viewer`](https://github.com/phate/rvsdg-viewer) to visualize the current RVSDG.
    ///
    /// NOTE: Only works on valid RVSDG's.
    pub fn open_rvsdg_viewer(&mut self) {
        let xml = self.to_xml();
        xml::open_viewer(xml)
    }

    /// Prints which regions belong to which nodes.
    pub fn dump_region_mapping(&mut self) {
        println!("Region Mapping:");
        for (r, region) in self.regions.iter() {
            let node_name = self.debug_node(region.container_node);
            println!("  {r} -> {node_name}");
        }
    }
}

/// The result of attempting to connect an origin to a user
///
/// See: [`Context::connect`]
#[derive(PartialEq, Eq, Debug)]
pub enum Connection {
    Ok,
    Cyclic,
    RecEnv(id::Node<RecEnv>),
    CantRecEnvAcrossRegions,
    NoPath(Origin, User),
}

impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Connection::Ok => "ok".fmt(f),
            Connection::Cyclic => "forms cycle".fmt(f),
            Connection::RecEnv(node) => write!(f, "ok (recenv {} created)", node.id),
            Connection::CantRecEnvAcrossRegions => "cannot create recenv across regions".fmt(f),
            Connection::NoPath(origin, user) => {
                write!(f, "no available path from {origin} to {user}")
            }
        }
    }
}
