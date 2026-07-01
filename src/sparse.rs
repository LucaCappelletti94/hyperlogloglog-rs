//! Sparse register storage for HyperLogLog-log.
//!
//! Stores (register\_index, register\_value) pairs in a sorted, fixed-capacity array.
//! Lookup is O(log s) via binary search; insertion is O(s) via insertion sort.
//! This matches the C++ reference implementation's sorted bit-packed array approach.

use core::fmt::Debug;
use core::hash::Hash;

/// A sparse register entry: a register index and its raw (uncompressed) value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SparseEntry {
    /// Register index (which bucket this value belongs to).
    pub index: usize,
    /// Raw register value (the rho value, not an offset).
    pub value: u8,
}

impl Ord for SparseEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl PartialOrd for SparseEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Fixed-capacity sorted array of sparse register entries.
///
/// Backed by a `[Option<SparseEntry>; CAPACITY]` array. Maintains sorted order
/// by index. Binary search for lookup, insertion sort for maintain.
///
/// This is the `no_std` equivalent of the C++ reference's sorted bit-packed array.
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Clone, Debug, Hash)]
pub struct SparseRegisters<const CAPACITY: usize> {
    entries: [Option<SparseEntry>; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> Default for SparseRegisters<CAPACITY> {
    fn default() -> Self {
        Self {
            entries: core::array::from_fn(|_| None),
            len: 0,
        }
    }
}

impl<const CAPACITY: usize> PartialEq for SparseRegisters<CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        for i in 0..self.len {
            if self.entries[i] != other.entries[i] {
                return false;
            }
        }
        true
    }
}

impl<const CAPACITY: usize> Eq for SparseRegisters<CAPACITY> {}

impl<const CAPACITY: usize> SparseRegisters<CAPACITY> {
    /// Creates a new empty sparse register array.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            len: 0,
        }
    }

    /// Returns the number of sparse entries.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if there are no sparse entries.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true if the sparse array is full.
    #[inline]
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len >= CAPACITY
    }

    /// Looks up the value for a given register index.
    ///
    /// Returns `Some(value)` if the index is present in the sparse array,
    /// `None` if it is not (meaning the register is in the dense array).
    #[inline]
    #[allow(clippy::comparison_chain)]
    #[must_use]
    pub fn get(&self, index: usize) -> Option<u8> {
        // Binary search for the index.
        let mut lo = 0usize;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.entries[mid] {
                Some(entry) => {
                    if entry.index == index {
                        return Some(entry.value);
                    } else if entry.index < index {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                None => {
                    hi = mid;
                }
            }
        }
        None
    }

    /// Inserts or updates a (index, value) pair.
    ///
    /// If the index already exists, updates the value. Otherwise, inserts a new entry.
    /// Returns `Err(())` if the sparse array is full.
    #[inline]
    #[allow(clippy::comparison_chain, clippy::result_unit_err)]
    pub fn insert(&mut self, index: usize, value: u8) -> Result<(), ()> {
        // Check for existing entry.
        let mut lo = 0usize;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if let Some(entry) = self.entries[mid] {
                if entry.index == index {
                    self.entries[mid] = Some(SparseEntry { index, value });
                    return Ok(());
                } else if entry.index < index {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            } else {
                hi = mid;
            }
        }

        // Insert new entry at position `lo`.
        if self.len >= CAPACITY {
            return Err(());
        }

        // Shift entries to make room.
        for i in (lo..self.len).rev() {
            self.entries[i + 1] = self.entries[i];
        }
        self.entries[lo] = Some(SparseEntry { index, value });
        self.len += 1;
        Ok(())
    }

    /// Removes an entry by index.
    ///
    /// Returns `true` if the entry was found and removed.
    #[inline]
    #[allow(clippy::comparison_chain)]
    pub fn remove(&mut self, index: usize) -> bool {
        let mut lo = 0usize;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if let Some(entry) = self.entries[mid] {
                if entry.index == index {
                    // Shift entries to fill the gap.
                    for i in mid..self.len - 1 {
                        self.entries[i] = self.entries[i + 1];
                    }
                    self.entries[self.len - 1] = None;
                    self.len -= 1;
                    return true;
                } else if entry.index < index {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            } else {
                hi = mid;
            }
        }
        false
    }

    /// Clears all entries.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns an iterator over the sparse entries in index order.
    #[inline]
    #[allow(clippy::iter_without_into_iter)]
    #[must_use]
    pub fn iter(&self) -> SparseIter<'_, CAPACITY> {
        SparseIter {
            sparse: self,
            index: 0,
        }
    }

    /// Iterates over all entries, calling `f` for each (index, value) pair.
    #[inline]
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, u8),
    {
        for i in 0..self.len {
            if let Some(entry) = self.entries[i] {
                f(entry.index, entry.value);
            }
        }
    }
}

/// Iterator over sparse register entries.
pub struct SparseIter<'a, const CAPACITY: usize> {
    sparse: &'a SparseRegisters<CAPACITY>,
    index: usize,
}

impl<const CAPACITY: usize> Iterator for SparseIter<'_, CAPACITY> {
    type Item = (usize, u8);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.sparse.len {
            if let Some(entry) = self.sparse.entries[self.index] {
                self.index += 1;
                return Some((entry.index, entry.value));
            }
            self.index += 1;
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sparse.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<const CAPACITY: usize> ExactSizeIterator for SparseIter<'_, CAPACITY> {}
