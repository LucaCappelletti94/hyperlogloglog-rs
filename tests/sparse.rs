#![allow(clippy::unwrap_used, missing_docs)]

use hyperlogloglog::sparse::SparseRegisters;

#[test]
fn test_sparse_empty() {
    let sparse: SparseRegisters<8> = SparseRegisters::new();
    assert!(sparse.is_empty());
    assert_eq!(sparse.get(0), None);
    assert_eq!(sparse.get(7), None);
}

#[test]
fn test_sparse_insert_and_get() {
    let mut sparse = SparseRegisters::<8>::new();
    sparse.insert(3, 5).unwrap();
    sparse.insert(1, 2).unwrap();
    sparse.insert(7, 8).unwrap();

    assert_eq!(sparse.len(), 3);
    assert_eq!(sparse.get(1), Some(2));
    assert_eq!(sparse.get(3), Some(5));
    assert_eq!(sparse.get(7), Some(8));
    assert_eq!(sparse.get(0), None);
    assert_eq!(sparse.get(4), None);
    assert_eq!(sparse.get(6), None);
}

#[test]
fn test_sparse_update_existing() {
    let mut sparse = SparseRegisters::<8>::new();
    sparse.insert(3, 5).unwrap();
    sparse.insert(3, 7).unwrap();

    assert_eq!(sparse.len(), 1);
    assert_eq!(sparse.get(3), Some(7));
}

#[test]
fn test_sparse_remove() {
    let mut sparse = SparseRegisters::<8>::new();
    sparse.insert(1, 2).unwrap();
    sparse.insert(3, 5).unwrap();
    sparse.insert(7, 8).unwrap();

    assert!(sparse.remove(3));
    assert_eq!(sparse.len(), 2);
    assert_eq!(sparse.get(3), None);
    assert_eq!(sparse.get(1), Some(2));
    assert_eq!(sparse.get(7), Some(8));

    assert!(!sparse.remove(5));
    assert_eq!(sparse.len(), 2);
}

#[test]
fn test_sparse_full() {
    let mut sparse = SparseRegisters::<2>::new();
    sparse.insert(1, 2).unwrap();
    sparse.insert(3, 5).unwrap();
    assert!(sparse.is_full());
    assert!(sparse.insert(7, 8).is_err());
}

#[test]
fn test_sparse_clear() {
    let mut sparse = SparseRegisters::<8>::new();
    sparse.insert(1, 2).unwrap();
    sparse.insert(3, 5).unwrap();
    sparse.clear();
    assert!(sparse.is_empty());
    assert_eq!(sparse.get(1), None);
}

#[test]
fn test_sparse_iter() {
    let mut sparse = SparseRegisters::<8>::new();
    sparse.insert(5, 10).unwrap();
    sparse.insert(2, 3).unwrap();
    sparse.insert(8, 1).unwrap();

    let entries: Vec<_> = sparse.iter().collect();
    assert_eq!(entries, vec![(2, 3), (5, 10), (8, 1)]);
}

#[test]
fn test_sparse_sorted_insertion() {
    let mut sparse = SparseRegisters::<16>::new();
    sparse.insert(10, 1).unwrap();
    sparse.insert(2, 2).unwrap();
    sparse.insert(7, 3).unwrap();
    sparse.insert(1, 4).unwrap();
    sparse.insert(15, 5).unwrap();

    let indices: Vec<_> = sparse.iter().map(|(i, _)| i).collect();
    assert_eq!(indices, vec![1, 2, 7, 10, 15]);
}

#[test]
fn test_sparse_partial_eq() {
    let mut s1 = SparseRegisters::<8>::new();
    let mut s2 = SparseRegisters::<8>::new();

    // Empty registers are equal.
    assert_eq!(s1, s2);

    s1.insert(1, 2).unwrap();
    s1.insert(3, 5).unwrap();
    s2.insert(1, 2).unwrap();
    s2.insert(3, 5).unwrap();

    // Same entries are equal.
    assert_eq!(s1, s2);

    s2.insert(7, 8).unwrap();

    // Different entries are not equal.
    assert_ne!(s1, s2);
}

#[test]
fn test_sparse_entry_ord() {
    use hyperlogloglog::sparse::SparseEntry;

    let a = SparseEntry { index: 1, value: 5 };
    let b = SparseEntry { index: 3, value: 2 };
    let c = SparseEntry { index: 1, value: 5 };

    assert!(a < b);
    assert_eq!(a, c);
    assert!(b.partial_cmp(&a).is_some());
}
