use crate::*;

use std::sync::Once;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, registry::Registry};

static TRACING_INIT: Once = Once::new();

const OPEN_VIEWER: bool = false;

pub fn test_logger() {
    TRACING_INIT.call_once(|| {
        let filter = EnvFilter::from_default_env();

        let layer = tracing_tree::HierarchicalLayer::default()
            .with_writer(std::io::stdout)
            .with_indent_lines(true)
            .with_indent_amount(2)
            .with_verbose_entry(false)
            .with_verbose_exit(false)
            .with_targets(true);

        let subscriber = Registry::default().with(layer).with(filter);

        tracing::subscriber::set_global_default(subscriber).unwrap();
    });
}

#[test]
#[should_panic(expected = "invalid connection: forms cycle")]
fn simple_cyclic() {
    test_logger();
    let mut ctx = Context::new("test(connect from deeply nested)");

    let first = ctx.add_placeholder_node("first");
    let second = ctx.add_placeholder_node("second");

    let first_input = ctx.add_input(first.node);
    let second_input = ctx.add_input(second.node);

    ctx.connect(first, second_input);
    ctx.connect(second, first_input);

    if OPEN_VIEWER {
        ctx.open_rvsdg_viewer();
    }
}

#[test]
fn connect_from_deeply_nested() {
    test_logger();
    let mut ctx = Context::new("test(connect from deeply nested)");

    let fa = ctx.add_lambda_node();
    ctx.add_symbol(fa.node.id, "fa");
    let fa_region = ctx.only_child_region(fa.node.id);
    let (fa_region_argument_2, fb_result) = ctx.in_region(fa_region, |ctx| {
        ctx.add_argument();
        let fa_region_argument_2 = ctx.add_argument();

        let n = ctx.add_number_node(1);

        let (predicate, switch) = ctx.add_switch_node();
        ctx.add_symbol(switch.id, "switch");
        let switch_output = ctx.add_output(switch);

        ctx.connect(n, predicate);

        ctx.add_switch_branch(switch);
        let (switch_region_1, _) = ctx.add_switch_branch(switch);

        let fb_result = ctx.in_region(switch_region_1, |ctx| {
            let fb = ctx.add_lambda_node();
            ctx.add_symbol(fb.node.id, "fb");
            let fb_region = ctx.only_child_region(fb.node.id);
            let fb_result = ctx.in_region(fb_region, |ctx| ctx.add_result());

            let apply = ctx.add_apply_node();
            ctx.connect(fb, apply);

            let fb_applied_output = ctx.add_output(apply.node);

            ctx.connect(
                fb_applied_output,
                ctx.output_as_result(switch_region_1, switch_output),
            );

            fb_result
        });

        (fa_region_argument_2, fb_result)
    });

    ctx.dump_region_mapping();
    ctx.connect(fa_region_argument_2, fb_result);

    if OPEN_VIEWER {
        ctx.open_rvsdg_viewer();
    }
}

// fn fa x = fb (x + 1)
// fn fb y = fa (y - 1)
// fn main = fa 10
#[test]
fn phi() {
    test_logger();

    let mut ctx = Context::new("test(phi)");

    let fa = ctx.add_lambda_node();
    ctx.add_symbol(fa.node.id, "fa");
    let fa_region = ctx.only_child_region(fa.node.id);
    let fa_fb_input = ctx.in_region(fa_region, |ctx| {
        let x = ctx.add_argument();
        let fb_input = ctx.add_input(fa.node);
        let fb = ctx.input_as_argument(fa_region, fb_input).unwrap();

        let num = ctx.add_number_node(1);

        let plus = ctx.add_placeholder_node("+");
        let plus_x = ctx.add_input(plus.node);
        let plus_y = ctx.add_input(plus.node);
        ctx.connect(x, plus_x);
        ctx.connect(num, plus_y);

        let apply = ctx.add_apply_node();
        let apply_output = ctx.add_output(apply.node);
        let apply_parameter = ctx.add_input(apply.node);
        ctx.connect(plus, apply_parameter);
        ctx.connect(fb, apply);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);

        fb_input
    });

    let fb = ctx.add_lambda_node();
    ctx.add_symbol(fb.node.id, "fb");
    let fb_region = ctx.only_child_region(fb.node.id);
    let fb_fa_input = ctx.in_region(fb_region, |ctx| {
        let x = ctx.add_argument();
        let fa_input = ctx.add_input(fb.node);
        let fa = ctx.input_as_argument(fb_region, fa_input).unwrap();

        let num = ctx.add_number_node(1);

        let minus = ctx.add_placeholder_node("-");
        let minus_x = ctx.add_input(minus.node);
        let minus_y = ctx.add_input(minus.node);
        ctx.connect(x, minus_x);
        ctx.connect(num, minus_y);

        let apply = ctx.add_apply_node();
        let apply_output = ctx.add_output(apply.node);
        let apply_parameter = ctx.add_input(apply.node);
        ctx.connect(minus, apply_parameter);
        ctx.connect(fa, apply);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);

        fa_input
    });

    let main = ctx.add_lambda_node();
    ctx.add_symbol(main.node.id, "main");
    let main_region = ctx.only_child_region(main.node.id);
    let main_fa_input = ctx.in_region(main_region, |ctx| {
        let main_fa_input = ctx.add_input(main.node);
        let main_fa_arg = ctx.input_as_argument(main_region, main_fa_input).unwrap();

        let init = ctx.add_number_node(10);
        let apply = ctx.add_apply_node();
        ctx.connect(main_fa_arg, apply);
        let apply_arg = ctx.add_input(apply.node);
        ctx.connect(init, apply_arg);

        let apply_output = ctx.add_output(apply.node);
        let result = ctx.add_result();
        ctx.connect(apply_output, result);

        main_fa_input
    });

    ctx.connect(fa, fb_fa_input);
    ctx.connect(fb, fa_fb_input);
    ctx.connect(fa, main_fa_input);

    if OPEN_VIEWER {
        ctx.open_rvsdg_viewer();
    }
}
