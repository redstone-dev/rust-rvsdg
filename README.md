<!-- # rvsdg -->

`rvsdg` is a Rust library for constructing and analyzing
[Regionalised Value State Dependence Graphs](https://arxiv.org/abs/1912.05036).

The crate provides a mostly type-safe API for building RVSDGs, connecting ports with pathfinding,
and handling transformation of recursive lambdas into RecEnv (phi) nodes automatically.

## Terminology

For developer familiarity and personal preference, most node kinds have been renamed.

- `Gamma` -> `Switch`
- `Theta` -> `DoWhile`
- `Delta` -> `GlobalV`
- `Phi` -> `RecEnv`
- `Omega` -> `TranslationUnit`

## Example

```rust
use rvsdg::{Context, nodes::Add};

let mut ctx = Context::new("my testing graph");

let (f_output, f_region) = ctx.add_lambda_node();
ctx.in_region(f_region, |ctx| {
    let one = ctx.add_number_node(1);
    let two = ctx.add_number_node(2);

    let ([x, y], addition) = ctx.add_binop_node::<Add>();

    ctx.connect(one, x);
    ctx.connect(two, y);

    let returned = ctx.add_result();
    ctx.connect(addition, returned);
});

let apply_input = ctx.add_apply_node();
ctx.connect(f_output, apply_input);
```

## Exporting and Visualization

If [`rvsdg-viewer`](https://github.com/phate/rvsdg-viewer) is installed, you
can visualize the graph with:

```rust
ctx.open_rvsdg_viewer();
```

> On NixOS, enter the dev shell with `nix develop`.

## Custom Nodes

For simple custom nodes, implement `NodeKind` and construct the node with
`Context::add_node`:

```rust
use rvsdg::{Context, Input, Output, node_kind_impl};

#[derive(Debug, Clone)]
struct Increment;
node_kind_impl!(Increment, "inc");

fn add_increment_node(ctx: &mut Context) -> (Input<Increment>, Output<Increment>) {
    let node = ctx.add_node(|_, _| Increment);
    let input = ctx.add_input(node);
    let output = ctx.add_output(node);
    (input, output)
}
```

More advanced node behavior can be customized through `Context::node_hooks_mut`.

## Status

This crate is currently acting as an experiment on whether its viable to create a standalone RVSDG implementation.
Longterm goal is use it for the [lumina](https://github.com/luminalang/lumina) programming language.
