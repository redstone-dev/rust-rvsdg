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

    let [first] = ctx.add_placeholder_node("first", []);
    let [second] = ctx.add_placeholder_node("second", []);

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

    let (fa, fa_region) = ctx.add_lambda_node();
    ctx.add_symbol(fa.node.id, "fa");
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
            let (fb, fb_region) = ctx.add_lambda_node();
            ctx.add_symbol(fb.node.id, "fb");
            let fb_result = ctx.in_region(fb_region, |ctx| ctx.add_result());

            let apply = ctx.add_apply_node();
            ctx.connect(fb, apply);

            let fb_applied_output = ctx.add_output(apply.node);

            ctx.connect(
                fb_applied_output,
                ctx.try_output_as_result(switch_region_1, switch_output),
            );

            fb_result
        });

        (fa_region_argument_2, fb_result)
    });

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

    let (fa, fa_region) = ctx.add_lambda_node();
    ctx.add_symbol(fa.node.id, "fa");
    let fa_fb_input = ctx.in_region(fa_region, |ctx| {
        let x = ctx.add_argument();
        let fb_input = ctx.add_input(fa.node);
        let fb = ctx.input_as_argument(fa_region, fb_input);

        let y = ctx.add_number_node(1);

        let ([add_x, add_y], out) = ctx.add_binop_node::<Add>();
        ctx.connect(x, add_x);
        ctx.connect(y, add_y);

        let [apply_output] = ctx.add_and_connect_apply_node(fb, &[out]);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);

        fb_input
    });

    let (fb, fb_region) = ctx.add_lambda_node();
    ctx.add_symbol(fb.node.id, "fb");
    let fb_fa_input = ctx.in_region(fb_region, |ctx| {
        let x = ctx.add_argument();
        let fa_input = ctx.add_input(fb.node);
        let fa = ctx.input_as_argument(fb_region, fa_input);

        let y = ctx.add_number_node(1);

        let ([sub_x, sub_y], out) = ctx.add_binop_node::<Sub>();
        ctx.connect(x, sub_x);
        ctx.connect(y, sub_y);

        let [apply_output] = ctx.add_and_connect_apply_node(fa, &[out]);

        let result = ctx.add_result();
        ctx.connect(apply_output, result);

        fa_input
    });

    let (main, main_region) = ctx.add_lambda_node();
    ctx.add_symbol(main.node.id, "main");
    let main_fa_input = ctx.in_region(main_region, |ctx| {
        let main_fa_input = ctx.add_input(main.node);
        let main_fa_arg = ctx.input_as_argument(main_region, main_fa_input);

        let init = ctx.add_number_node(10);

        let [apply_output] = ctx.add_and_connect_apply_node(main_fa_arg, &[init]);

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

// Test for [Figure 2 (d)](https://arxiv.org/abs/1912.05036)
#[test]
fn theta() {
    test_logger();

    let mut ctx = Context::new("test(theta)");

    let init = ctx.add_number_node(1);

    let (predicate, theta) = ctx.add_dowhile_node();

    let theta_a = ctx.add_input(theta);
    let theta_b = ctx.add_input(theta);

    ctx.connect(init, theta_a);
    ctx.connect(init, theta_b);

    ctx.in_region(predicate.region, |ctx| {
        let theta_arg_a = ctx.input_as_argument(predicate.region, theta_a);
        let theta_arg_b = ctx.input_as_argument(predicate.region, theta_b);

        let one = ctx.add_number_node(1);
        let five = ctx.add_number_node(5);

        let ([add_x, add_y], add_out) = ctx.add_binop_node::<Add>();
        let ([less_x, less_y], less_out) = ctx.add_binop_node::<LessThan>();
        let ([mul_x, mul_y], mul_out) = ctx.add_binop_node::<Mul>();

        ctx.connect(theta_arg_a, add_x);
        ctx.connect(one, add_y);

        ctx.connect(add_out, less_x);
        ctx.connect(five, less_y);

        ctx.connect(theta_arg_a, mul_x);
        ctx.connect(theta_arg_b, mul_y);

        ctx.connect(less_out, predicate);
        let theta_result_a = ctx.input_as_result(theta_a);
        let theta_result_b = ctx.input_as_result(theta_b);
        ctx.connect(add_out, theta_result_a);

        ctx.connect(mul_out, theta_result_b);
    });

    if OPEN_VIEWER {
        ctx.open_rvsdg_viewer();
    }
}

// TODO: We need to ban output_as_result for things like lambda
//
// Can we do that statically? or must we use assertion?

// Test for [Figure 1 (d)](https://arxiv.org/abs/1912.05036)
#[test]
fn temp_png_graph() {
    test_logger();

    let mut ctx = Context::new("test(temp.png graph)");

    let (f, f_region) = ctx.add_lambda_node();
    ctx.add_symbol(f.node.id, "f");

    ctx.in_region(f_region, |ctx| {
        let f_result = ctx.add_result();

        let f_arguments = [(); 4].map(|_| ctx.add_argument());

        let ([add_x, add_y], add_out) = ctx.add_binop_node::<Add>();
        let ([sub_x, sub_y], sub_out) = ctx.add_binop_node::<Sub>();
        let [ud] = ctx.add_placeholder_node("ud", []);

        ctx.connect(f_arguments[1], add_x);
        ctx.connect(f_arguments[2], add_y);
        ctx.connect(f_arguments[1], sub_x);
        ctx.connect(f_arguments[3], sub_y);

        let (theta_predicate, theta) = ctx.add_dowhile_node();

        let theta_inputs = [(); 6].map(|_| ctx.add_input(theta));

        ctx.connect(f_arguments[0], theta_inputs[0]);
        ctx.connect(f_arguments[1], theta_inputs[1]);
        ctx.connect(f_arguments[2], theta_inputs[2]);
        ctx.connect(f_arguments[3], theta_inputs[3]);
        ctx.connect(add_out, theta_inputs[4]);
        ctx.connect(ud, theta_inputs[5]);

        ctx.in_region(theta_predicate.region, |ctx| {
            let theta_arguments =
                theta_inputs.map(|input| ctx.input_as_argument(theta_predicate.region, input));

            let ([mul_x, mul_y], mul_out) = ctx.add_binop_node::<Mul>();
            let ([upper_gt_x, upper_gt_y], upper_gt_out) = ctx.add_binop_node::<GreaterThan>();
            let ([lower_gt_x, lower_gt_y], lower_gt_out) = ctx.add_binop_node::<GreaterThan>();
            let ([upper_shl_x, upper_shl_y], upper_shl_out) = ctx.add_binop_node::<ShiftLeft>();
            let ([lower_shl_x, lower_shl_y], lower_shl_out) = ctx.add_binop_node::<ShiftLeft>();

            ctx.connect(theta_arguments[0], mul_x);
            ctx.connect(theta_arguments[4], mul_y);

            ctx.connect(mul_out, upper_gt_x);
            ctx.connect(theta_arguments[3], upper_gt_y);

            ctx.connect(mul_out, upper_shl_x);
            ctx.connect(theta_arguments[1], upper_shl_y);

            let (gamma_predicate, gamma) = ctx.add_switch_node();
            let gamma_out = ctx.add_output(gamma);
            let gamma_inputs = [(); 3].map(|_| ctx.add_input(gamma));

            let (gamma_region0, _) = ctx.add_switch_branch(gamma);
            let (gamma_region1, _) = ctx.add_switch_branch(gamma);

            ctx.connect(upper_gt_out, gamma_predicate);
            ctx.connect(mul_out, gamma_inputs[0]);
            ctx.connect(mul_out, gamma_inputs[1]);
            ctx.connect(theta_arguments[2], gamma_inputs[2]);

            // 0
            ctx.in_region(gamma_region0, |ctx| {
                let argument = ctx.input_as_argument(gamma_region0, gamma_inputs[0]);
                let result = ctx.try_output_as_result(gamma_region0, gamma_out);
                ctx.connect(argument, result);
            });
            // 1
            ctx.in_region(gamma_region1, |ctx| {
                let [argument_b, argument_c] = [gamma_inputs[1], gamma_inputs[2]]
                    .map(|input| ctx.input_as_argument(gamma_region1, input));

                let ([rem_x, rem_y], rem_out) = ctx.add_binop_node::<Rem>();
                ctx.connect(argument_b, rem_x);
                ctx.connect(argument_c, rem_y);

                let three = ctx.add_number_node(3);

                let ([add_x, add_y], add_out) = ctx.add_binop_node::<Add>();
                ctx.connect(three, add_x);
                ctx.connect(rem_out, add_y);

                let result = ctx.try_output_as_result(gamma_region1, gamma_out);
                ctx.connect(add_out, result);
            });

            ctx.connect(gamma_out, lower_gt_x);
            ctx.connect(upper_shl_out, lower_gt_y);
            ctx.connect(gamma_out, lower_shl_x);
            ctx.connect(theta_arguments[1], lower_shl_y);

            let theta_results = theta_inputs.map(|input| ctx.input_as_result(input));

            ctx.connect(lower_gt_out, theta_predicate);
            ctx.connect(upper_shl_out, theta_results[0]);
            ctx.connect(theta_arguments[1], theta_results[1]);
            ctx.connect(theta_arguments[2], theta_results[2]);
            ctx.connect(theta_arguments[3], theta_results[3]);
            ctx.connect(theta_arguments[4], theta_results[4]);
            ctx.connect(lower_shl_out, theta_results[5]);
        });

        let ([lower_add_x, lower_add_y], lower_add_out) = ctx.add_binop_node::<Add>();

        ctx.connect(sub_out, lower_add_x);
        let theta_last_output = ctx.input_as_output(theta_inputs[5]);
        ctx.connect(theta_last_output, lower_add_y);
        ctx.connect(lower_add_out, f_result);
    });

    if OPEN_VIEWER {
        ctx.open_rvsdg_viewer();
    }
}
