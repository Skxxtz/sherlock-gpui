use elementary::compile::Program;
use elementary::eval::eval_with;
use elementary::parser::parse_tree;

fn run(src: &str, x: f64) -> Option<f64> {
    Program::compile(&parse_tree(src).unwrap(), "x")
        .unwrap()
        .run(x)
}

#[test]
fn matches_tree_eval() {
    for src in [
        "2x + 1",
        "x² - 5x",
        "x + 19%",
        "x - 25%",
        "sin(x) * 2pi",
        "max(x, 3, 1)",
        "x(x + 2)",
        "(x + 1)!",
        "log(x, 2)",
    ] {
        let tree = parse_tree(src).unwrap();
        for x in [-3.0, 0.5, 2.0, 7.0] {
            let a = run(src, x);
            let b = eval_with(&tree, &[("x", x)]).ok();
            assert_eq!(a, b, "{src} at x = {x}");
        }
    }
}

#[test]
fn undefined_is_none() {
    assert_eq!(run("1 / x", 0.0), None);
    assert_eq!(run("tan(x)", 90.0), None);
    assert_eq!(run("sqrt(x)", -1.0), None);
}

#[test]
fn folds_constants() {
    let p = Program::compile(&parse_tree("x + sqrt(16) * 2").unwrap(), "x").unwrap();
    assert_eq!(p.len(), 3); // X, Const(8), Add
}
