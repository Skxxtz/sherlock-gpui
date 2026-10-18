use elementary::func::Func;

#[test]
fn names_round_trip() {
    for name in ["sqrt", "log2", "sin", "max"] {
        assert_eq!(Func::from_name(name).unwrap().name(), name);
    }
    assert_eq!(Func::from_name("foo"), None);
}
