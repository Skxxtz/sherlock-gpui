use parley::{Intent, SlotKind};

#[test]
fn every_intent_has_slots_from_all() {
    for i in Intent::ALL {
        assert!(!i.slots().is_empty(), "{i:?} has no slots");
        for s in i.slots() {
            assert!(SlotKind::ALL.contains(s));
        }
    }
}

#[test]
fn names_are_unique() {
    let mut n: Vec<_> = Intent::ALL.iter().map(|i| i.name()).collect();
    n.sort();
    n.dedup();
    assert_eq!(n.len(), Intent::ALL.len());
    let mut s: Vec<_> = SlotKind::ALL.iter().map(|s| s.name()).collect();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), SlotKind::ALL.len());
}
