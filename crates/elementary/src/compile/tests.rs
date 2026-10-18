use super::*;
use crate::parser::parse_tree;

#[test]
fn folds_constants() {
    let p = Program::compile(&parse_tree("x + sqrt(16) * 2").unwrap(), "x").unwrap();
    assert_eq!(p.code.len(), 3); // X, Const(8), Add
}
