use super::{Apply, Context, Lambda, id};
use cranelift_entity::SecondaryMap;

impl Context {
    pub fn opt_inline(&mut self, lambda: id::Node<Lambda>, apply: id::Node<Apply>) {
        let lregion = self.only_child_region(lambda.id);
        let to_region = self.parent_region(apply.id).unwrap();

        let mut arguments = SecondaryMap::new();
        let mut results = SecondaryMap::new();

        let mut argument_counter = 0;

        // Inline the arguments
        for input in self.inputs(apply.id).skip(1) {
            let arg = id::Argument::from_u32(argument_counter);
            argument_counter += 1;
            arguments[arg] = self.get_user(input);
        }

        // Inline the other lambda inputs
        for input in self.inputs(lambda.id) {
            let arg = id::Argument::from_u32(argument_counter);
            argument_counter += 1;
            arguments[arg] = self.get_user(input);
        }

        for output in self.outputs(apply.id) {
            let res = id::Result::from_u32(output.id.0);
            results[res] = Some(self.get_origins(output).to_vec());
        }

        self.in_region(to_region, |ctx| {
            ctx.deep_clone_nodes_from(lregion, &arguments, &results)
        });

        self.remove_node_from_region(apply.id);
    }
}

// // Expression rewrite stuff that may or may not be used
//
// struct NodeDescriptor<'s> {
//     node: Box<dyn NodeKind>,
//     inputs: Vec<Pattern<'s>>,
//     outputs: Vec<Pattern<'s>>,
// }
//
// impl<'s> NodeDescriptor<'s> {
//     fn new<N: NodeKind>(kind: N) -> Self {
//         Self {
//             node: Box::new(kind),
//             inputs: vec![],
//             outputs: vec![],
//         }
//     }
//
//     fn i(mut self, pat: Pattern<'s>) -> Self {
//         self.inputs.push(pat);
//         self
//     }
//
//     fn o(mut self, pat: Pattern<'s>) -> Self {
//         self.outputs.push(pat);
//         self
//     }
// }
//
// type Pattern<'s> = &'s str;
//
// fn fast_double<'s>() -> (Vec<NodeDescriptor<'s>>, Vec<NodeDescriptor<'s>>) {
//     let from = vec![
//         NodeDescriptor::new(nodes::Number(2)).o("y"),
//         NodeDescriptor::new(nodes::Mul).i("x").i("y").o("z"),
//     ];
//
//     let to = vec![
//         NodeDescriptor::new(nodes::Number(1)).o("y"),
//         NodeDescriptor::new(nodes::ShiftLeft).i("x").i("y").o("z"),
//     ];
//
//     (from, to)
// }
