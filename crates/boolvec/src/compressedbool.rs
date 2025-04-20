/*
   ___   __        _   __
  / _ | / /__  ___| | / /__ _______ _
 / __ |/ / _ \/ -_) |/ / -_) __/ _ `/
/_/ |_/_/\___/\__/|___/\__/_/  \_,_/

Copyright 2025 Gavin Kellam

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

use crate::BoolArray;
use alloc::{
    boxed::Box,
    collections::btree_map::{self, BTreeMap},
};

/// Represents the state of a fixed-size chunk within a [`CompressedBool`](CompressedBool).
enum BitElementState<const STRIDE: usize> {
    /// Indicates that all `STRIDE` bits within this chunk are set to `1` (true).
    AllOnes,
    /// Indicates that all `STRIDE` bits within this chunk are set to `0` (false).
    AllZeros,
    /// Indicates a mix of `0`s and `1`s within the chunk.
    Map {
        map: Box<BoolArray<usize, STRIDE>>,
        ones: u32,
    },
}

/// A space-efficient data structure for storing large sequences of boolean values.
///
/// [`CompressedBool`](Self) optimizes storage by grouping bits into chunks of a fixed size,
/// defined by `STRIDE`. It leverages the fact that large datasets often contain long
/// contiguous sections of identical values.
///
/// Instead of storing every bit individually, `CompressedBool` uses a [`BTreeMap`]
/// to store only the chunks that are *not* implicitly all zeros. Chunks that are
/// entirely zero are not present in the map, providing significant memory savings and
/// performance when the data is sparse or contains many blocks of zeros.
///
/// The `STRIDE` parameter influences the trade-off between map overhead and chunk density.
/// Larger strides mean fewer entries in the [`BTreeMap`] but potentially more wasted space
/// within chunk variants if they are sparsely populated. Smaller strides lead to more map
/// entries but less wasted space within individual chunks. A good middle ground
/// was picked to be around **64** (the default for [new()](Self::new)).
///
/// # Examples
///
/// ```
/// use boolvec::CompressedBool;
///
/// let mut bits = CompressedBool::new();
///
/// // Set some bits
/// bits.set(10, true);
/// bits.set(65, true);
/// bits.set(1000, true);
///
/// // Check bit values
/// assert_eq!(bits.get(0), false);
/// assert_eq!(bits.get(10), true);
/// assert_eq!(bits.get(65), true);
/// assert_eq!(bits.get(999), false);
/// assert_eq!(bits.get(1000), true);
///
/// // Find the first set bit
/// assert_eq!(bits.find_first_of(true), Some(10));
///
/// // Find the first unset bit starting from index 10
/// assert_eq!(bits.find_first_skipping(10, false), Some(11));
/// ```
///
/// [`BTreeMap`]: alloc::collections::BTreeMap
pub struct CompressedBool<const STRIDE: usize> {
    map: BTreeMap<usize, BitElementState<STRIDE>>,
}

impl CompressedBool<64> {
    /// Creates a new, empty [`CompressedBool`](Self) with a chunk size of 64 bits.
    ///
    /// **64** was picked to be a good middle ground between size and performance. On 64-bit systems
    /// each 'chunk' (with `STRIDE` being 64) stores 4096-bits (_or 512-bytes_).
    ///
    /// This is a convenience function equivalent to [CompressedBool::new_stride()](Self::new_stride).
    ///
    /// Note: All bits default implicitly to `false`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use boolvec::CompressedBool;
    ///
    /// let mut bits = CompressedBool::new();
    ///
    /// bits.set(100, true);
    /// assert_eq!(bits.get(100), true);
    /// ```
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl<const STRIDE: usize> CompressedBool<STRIDE> {
    /// Creates a new, empty [`CompressedBool`](Self) with the specified `STRIDE`.
    ///
    /// All bits default implicitly to `false`. Use this constructor when you
    /// need a chunk size other than **64**.
    ///
    /// ## Panics
    ///
    /// `STRIDE` must be greater or equal to `1`, and must be less than [`isize::MAX`](isize::MAX)
    ///
    /// ## Examples
    ///
    /// ```
    /// use boolvec::CompressedBool;
    ///
    /// // Create a CompressedBool with a chunk size of 128-usizes
    /// let bits = CompressedBool::<128>::new_stride();
    ///
    /// assert_eq!(bits.get(0), false);
    /// ```
    pub const fn new_stride() -> Self {
        assert!(STRIDE >= 1, "Stride must be greater or equal to 1!");
        assert!(
            STRIDE < (isize::MAX as usize),
            "Stride must be less than isize::MAX!"
        );

        Self {
            map: BTreeMap::new(),
        }
    }

    #[inline]
    const fn index_cal(bit_at: usize) -> (usize, usize) {
        let bits_per_element = usize::BITS as usize * STRIDE;
        (bit_at / bits_per_element, bit_at % bits_per_element)
    }

    #[inline]
    const fn reverse_index_cal(chunk_index: usize, inner_index: usize) -> usize {
        let bits_per_element = usize::BITS as usize * STRIDE;
        (chunk_index * bits_per_element) + inner_index
    }

    /// Sets the value of the bit at the specified index.
    ///
    /// If the targeted bit is within a chunk that was previously implicitly all
    /// zeros, a new entry will be created in the case where `bit_of` is `true`.
    /// In the case that `bit_of` is `false`, no change will be done if the targeted chunk
    /// was already implicitly all zeros. Furthermore, if the targeted chunk is _almost_ (`BITS-1`) all
    /// `true`, the chunk will be deallocated and replaced with a internal marker.
    ///
    /// This function will allocate for two reasons:
    /// 1. The previous chunk wasn't present in the array, and `bit_of` is `true`.
    /// 2. The previous chunk was _all-true_ marked, and `bit_of` is `false`.
    ///
    ///
    /// ## Arguments
    ///
    /// - `bit_at`: The 0-based linear index of the bit to set.
    /// - `bit_of`: The boolean value to set the bit.
    ///
    /// ## Complexity
    ///
    /// Typically logarithmic in the number of stored chunks due to the [`BTreeMap`] lookup.
    /// However, operations within a chunk are equivalent to lookups in [`BoolArray`](super::BoolArray) or
    /// otherwise `O(1)`.
    ///
    /// See [`BTreeMap`] for more information on time complexity.
    ///
    /// ## Examples
    ///
    /// ```
    /// use boolvec::CompressedBool;
    ///
    /// let mut bits = CompressedBool::new();
    ///
    /// bits.set(100, true);
    /// assert!(bits.get(100));
    ///
    /// bits.set(100, false);
    /// assert!(!bits.get(100));
    /// ```
    ///
    /// [`BTreeMap`]: alloc::collections::BTreeMap
    pub fn set(&mut self, bit_at: usize, bit_of: bool) {
        let map_bits = STRIDE * usize::BITS as usize;
        let (array_index, inner_index) = Self::index_cal(bit_at);

        match self.map.entry(array_index) {
            // If no element exists and we are trying to set a 'false', we don't need to insert because
            // elements are by default 'AllZeros'.
            btree_map::Entry::Vacant(_) if !bit_of => return,
            btree_map::Entry::Vacant(vacant_entry) => {
                let mut map = Box::new(BoolArray::<usize, STRIDE>::new());

                // Set the single bit
                map.set(inner_index, true);

                vacant_entry.insert(BitElementState::Map { map, ones: 1 });
            }

            btree_map::Entry::Occupied(mut occupied_entry) => {
                let state_change = match occupied_entry.get_mut() {
                    // If this element is already all `true` then we don't need to do anything
                    BitElementState::AllOnes if bit_of => None,
                    BitElementState::AllOnes => {
                        let mut map = Box::new(BoolArray::<usize, STRIDE>::new());

                        // Set the single bit to false
                        map.set_all(true);
                        map.set(inner_index, false);

                        Some(BitElementState::Map {
                            map,
                            ones: (map_bits - 1) as u32,
                        })
                    }
                    BitElementState::Map { map, ones } => {
                        if *ones == 1 && !bit_of {
                            Some(BitElementState::AllZeros)
                        } else if *ones as usize == map_bits - 1 && bit_of {
                            // Here we are inserting a 1 into a nearly full map
                            Some(BitElementState::AllOnes)
                        } else if map.get(inner_index) == bit_of {
                            // Bit is already set correctly
                            None
                        } else {
                            map.set(inner_index, bit_of);

                            if bit_of {
                                *ones += 1;
                            } else {
                                *ones -= 1;
                            }

                            None
                        }
                    }
                    BitElementState::AllZeros => {
                        unreachable!("Entry 'AllZeros' should never appear in the array!")
                    }
                };

                match state_change {
                    Some(BitElementState::AllZeros) => {
                        occupied_entry.remove();
                    }
                    Some(new_state) => {
                        occupied_entry.insert(new_state);
                    }
                    None => (),
                }
            }
        }
    }

    /// Gets the value of the bit at the specified index.
    ///
    /// If the index `bit_at` falls within a chunk not present in the internal map,
    /// it is considered to be implicitly zero, so `false` is returned.
    ///
    /// ## Arguments
    ///
    /// * `bit_at`: The 0-based linear index of the bit to retrieve.
    ///
    /// ## Complexity
    ///
    /// Typically logarithmic in the number of stored chunks due to the [`BTreeMap`] lookup.
    /// However, operations within a chunk are equivalent to lookups in [`BoolArray`](super::BoolArray) or
    /// otherwise `O(1)`.
    ///
    /// See [`BTreeMap`] for more information on time complexity.
    ///
    /// ## Examples
    ///
    /// ```
    /// use boolvec::CompressedBool;
    ///
    /// let mut bits = CompressedBool::new();
    ///
    /// assert_eq!(bits.get(50), false);
    /// bits.set(50, true);
    /// assert_eq!(bits.get(50), true);
    /// ```
    ///
    /// [`BTreeMap`]: alloc::collections::BTreeMap
    pub fn get(&self, bit_at: usize) -> bool {
        let (array_index, inner_index) = Self::index_cal(bit_at);

        match self.map.get(&array_index) {
            None => false,
            Some(BitElementState::AllOnes) => true,
            Some(BitElementState::AllZeros) => {
                unreachable!("Entry 'AllZeros' should never appear in the array!")
            }
            Some(BitElementState::Map { map, .. }) => map.get(inner_index),
        }
    }

    /// Finds the index of the first occurrence of a bit with the specified value.
    ///
    /// Searches the entire conceptual bit array from index 0 upwards. Returning:
    ///
    /// - `Some(index)` containing the 0-based linear index of the first matching bit found. For
    ///   `bit_of` being `false`, this will always return the index of the last set `1` plus 1.
    /// - `None` if no bit with the specified value exists in the array.
    ///
    /// ## Examples
    ///
    /// ```
    /// use boolvec::CompressedBool;
    ///
    /// let mut bits = CompressedBool::new();
    /// bits.set(70, true);
    /// bits.set(150, true);
    ///
    /// assert_eq!(bits.find_first_of(true), Some(70));
    /// assert_eq!(bits.find_first_of(false), Some(0));
    ///
    /// // If we set the first bit
    /// bits.set(0, true);
    ///
    /// // Next bit is implicitly false
    /// assert_eq!(bits.find_first_of(false), Some(1));
    /// ```
    pub fn find_first_of(&self, bit_of: bool) -> Option<usize> {
        self.find_first_skipping(0, bit_of)
    }

    /// Finds the index of the first occurrence of a bit with the specified value,
    /// starting the search from `start_index` (inclusive).
    ///
    /// This allows resuming a search or finding the *next* occurrence after a known index.
    ///
    /// ## Arguments
    ///
    /// * `start_index`: The 0-based linear index from where to begin the search (inclusive).
    /// * `bit_of`: The bool value to search for.
    ///
    /// ## Returns
    ///
    /// * `Some(index)` containing the 0-based linear index of the first matching bit found
    ///   at or after `start_index`.
    /// * `None` if no matching bit is found at or after `start_index`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use boolvec::CompressedBool;
    /// let mut bits = CompressedBool::new();
    /// bits.set(70, true);
    /// bits.set(150, true);
    ///
    /// // Find first 'true' starting from index 0
    /// assert_eq!(bits.find_first_skipping(0, true), Some(70));
    ///
    /// // Find first 'true' starting from index 71
    /// assert_eq!(bits.find_first_skipping(71, true), Some(150));
    ///
    /// // Find first 'true' starting from index 151
    /// assert_eq!(bits.find_first_skipping(151, true), None);
    ///
    /// // Find first 'false' starting from index 70
    /// assert_eq!(bits.find_first_skipping(70, false), Some(71)); // Bit 70 is true
    /// ```
    pub fn find_first_skipping(&self, start_index: usize, bit_of: bool) -> Option<usize> {
        let (mut array_index, mut bit_index) = Self::index_cal(start_index);
        let mut iter = self.map.iter().skip_while(move |(i, _)| array_index > **i);

        loop {
            let Some((&array_e_index, array_e)) = iter.next() else {
                return if bit_of {
                    None
                } else {
                    Some(Self::reverse_index_cal(array_index, bit_index))
                };
            };

            if array_e_index > array_index {
                if !bit_of {
                    return Some(Self::reverse_index_cal(array_index, bit_index));
                }

                array_index = array_e_index;
                bit_index = 0;
            }

            match array_e {
                BitElementState::AllOnes if bit_of => {
                    return Some(Self::reverse_index_cal(array_e_index, bit_index));
                }
                BitElementState::AllOnes => (),
                BitElementState::AllZeros => {
                    unreachable!("Entry 'AllZeros' should never appear in the array!")
                }
                BitElementState::Map { map, .. } => {
                    match map.find_first_skipping(bit_index, bit_of) {
                        Some(found_inner_index) => {
                            return Some(Self::reverse_index_cal(array_e_index, found_inner_index));
                        }
                        None => (),
                    }
                }
            }

            bit_index = 0;
            array_index += 1;
        }
    }

    /// Finds the starting index of the first contiguous sequence of `amount` bits
    /// that all have the specified value (`bit_of`).
    ///
    /// Searches the entire [`CompressedBool`](Self) starting from index 0. This is equivalent
    /// to calling [`find_first_of_many_skipping(0, bit_of, amount)`](Self::find_first_of_many_skipping).
    ///
    /// ## Arguments
    ///
    /// * `bit_of`: The boolean value (`true` or `false`) that all bits in the sequence must have.
    /// * `amount`: The required number of consecutive matching bits. If `amount` is 0,
    ///   `Some(0)` is typically returned.
    ///
    /// ## Returns
    ///
    /// * `Some(index)` containing the 0-based linear starting index of the first sequence found.
    /// * `None` if no such sequence of the required length and value exists.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use boolvec::CompressedBool;
    /// let mut bits = CompressedBool::new();
    ///
    /// // Set bits 100 through 109 to true
    /// for i in 100..110 {
    ///     bits.set(i, true);
    /// }
    ///
    /// // Find the first sequence of 5 true bits
    /// assert_eq!(bits.find_first_of_many(true, 5), Some(100));
    ///
    /// // Find the first sequence of 10 true bits
    /// assert_eq!(bits.find_first_of_many(true, 10), Some(100));
    ///
    /// // Find the first sequence of 11 true bits
    /// assert_eq!(bits.find_first_of_many(true, 11), None);
    ///
    /// // Find the first sequence of 100 false bits
    /// assert_eq!(bits.find_first_of_many(false, 100), Some(0));
    ///
    /// // Find the first sequence of 101 false bits
    /// assert_eq!(bits.find_first_of_many(false, 101), Some(110));
    /// ```
    pub fn find_first_of_many(&self, bit_of: bool, amount: usize) -> Option<usize> {
        self.find_first_of_many_skipping(0, bit_of, amount)
    }

    /// Finds the starting index of the first contiguous sequence of `amount` bits
    /// that all have the specified value (`bit_of`), starting the search from `start_index`.
    ///
    /// The search looks for sequences whose starting position is at or after `start_index`.
    ///
    /// # Arguments
    ///
    /// * `start_index`: The 0-based linear index from where to begin the search for the
    ///   *start* of a qualifying sequence.
    /// * `bit_of`: The boolean value that all bits in the sequence must have.
    /// * `amount`: The required number of consecutive matching bits. If `amount` is 0,
    ///   `Some(start_index)` is typically returned.
    ///
    /// # Returns
    ///
    /// * `Some(index)` containing the 0-based linear starting index of the first sequence found
    ///   whose start is at or after `start_index`.
    /// * `None` if no such sequence is found at or after `start_index`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use boolvec::CompressedBool;
    /// let mut bits = CompressedBool::new();
    ///
    /// // Set bits 10-14 (5 bits) and 20-29 (10 bits) to true
    /// for i in 10..15 { bits.set(i, true); }
    /// for i in 20..30 { bits.set(i, true); }
    ///
    /// // Find sequence of 3 true bits starting search at 0
    /// assert_eq!(bits.find_first_of_many_skipping(0, true, 3), Some(10));
    ///
    /// // Find sequence of 3 true bits starting search at 11
    /// assert_eq!(
    ///     bits.find_first_of_many_skipping(11, true, 3),
    ///     Some(11)
    /// ); // Starts within the 10-14 block
    ///
    /// // Find sequence of 3 true bits starting search at 13
    /// // (only bits 13, 14 are true here) -> must find next block
    /// assert_eq!(bits.find_first_of_many_skipping(13, true, 3), Some(20));
    ///
    /// // Find sequence of 8 true bits starting search at 15
    /// assert_eq!(bits.find_first_of_many_skipping(15, true, 8), Some(20));
    ///
    /// // Find sequence of 12 true bits starting search at 0 (doesn't exist)
    /// assert_eq!(bits.find_first_of_many_skipping(0, true, 12), None);
    /// ```
    pub fn find_first_of_many_skipping(
        &self,
        start_index: usize,
        bit_of: bool,
        amount: usize,
    ) -> Option<usize> {
        let mut starting_offset_bit = self.find_first_skipping(start_index, bit_of)?;
        let mut count = 1;

        loop {
            if count >= amount {
                return Some(starting_offset_bit);
            }

            let inter_bit = self.find_first_skipping(starting_offset_bit + count, bit_of)?;
            if inter_bit != starting_offset_bit + count {
                starting_offset_bit = inter_bit;
                count = 1;
            } else {
                count += 1;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_putting_ones_into_array() {
        let mut v = CompressedBool::new();

        for index in 0..1024 {
            v.set(index, true);
            assert_eq!(v.get(index), true);
        }

        assert_eq!(v.get(1025), false);

        for index in 0..1024 {
            v.set(index, false);
            assert_eq!(v.get(index), false);
        }

        assert_eq!(v.get(1025), false);
    }

    #[test]
    fn test_pattern_bits() {
        let mut v = CompressedBool::new();

        for array_index in 0..16 {
            for bit_index in 0..32 {
                if bit_index % 2 == 0 {
                    v.set(array_index * 8 + bit_index, true);
                }
            }
        }

        for array_index in 0..16 {
            for bit_index in 0..32 {
                assert_eq!(v.get(array_index * 8 + bit_index), bit_index % 2 == 0);
            }
        }

        for array_index in 0..16 {
            for bit_index in 0..32 {
                if bit_index % 2 == 0 {
                    v.set(array_index * 8 + bit_index, false);
                }
            }
        }

        for array_index in 0..16 {
            for bit_index in 0..32 {
                assert_eq!(v.get(array_index * 8 + bit_index), false);
            }
        }
    }

    #[test]
    fn test_find_first_of_false() {
        let mut v = CompressedBool::new();

        for some_bits in 0..1024 {
            v.set(some_bits, true);
        }

        for some_bits in 1025..2048 {
            v.set(some_bits, true);
        }

        assert_eq!(v.find_first_of(false), Some(1024));
    }

    #[test]
    fn test_find_first_of_true() {
        let mut v = CompressedBool::new();

        v.set(7043, true);

        assert_eq!(v.find_first_of(true), Some(7043));
    }

    #[test]
    fn test_find_first_skipping() {
        let mut v = CompressedBool::new();

        v.set(1, true);
        v.set(2, true);
        v.set(3, true);

        assert_eq!(v.find_first_of(true), Some(1));
        assert_eq!(v.find_first_skipping(1, true), Some(1));
        assert_eq!(v.find_first_skipping(2, true), Some(2));
        assert_eq!(v.find_first_skipping(3, true), Some(3));
        assert_eq!(v.find_first_skipping(4, true), None);
    }

    #[test]
    fn test_find_first_of_many() {
        let mut v = CompressedBool::new();

        v.set(1, true);
        v.set(2, true);
        v.set(4, true);
        v.set(6, true);
        v.set(7, true);
        v.set(8, true);
        v.set(11, true);

        assert_eq!(v.find_first_of(true), Some(1));
        assert_eq!(v.find_first_of(false), Some(0));
        assert_eq!(v.find_first_of_many(false, 1), Some(0));
        assert_eq!(v.find_first_of_many(true, 1), Some(1));
        assert_eq!(v.find_first_of_many(true, 2), Some(1));
        assert_eq!(v.find_first_of_many(false, 2), Some(9));
    }

    #[test]
    fn test_setting_really_far_away_bit() {
        let mut v = CompressedBool::new();

        v.set(usize::MAX - 1, true);
        assert_eq!(v.find_first_of(true), Some(usize::MAX - 1));
        assert_eq!(v.find_first_of(false), Some(0));

        assert_eq!(v.get(usize::MAX - 2), false);
        assert_eq!(v.get(usize::MAX - 1), true);
        assert_eq!(v.get(usize::MAX), false);
    }
}
