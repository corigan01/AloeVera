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

/// A fixed-size array storing bits packed into integer elements.
///
/// `BitArray` provides efficient storage and manipulation of a sequence of bits.
/// The bits are stored contiguously in an underlying array of type `[T; N]`.
///
/// # Type Parameters
///
/// * `T`: The integer type used for the underlying storage (impl for `u8`, `u16`, `u32`, `u64`, `u128`, `usize`).
/// * `N`: The number of `T` elements in the underlying storage array or `(N * T::BITS)` bits.
///
/// # Bit Capacity
///
/// The total number of bits that can be stored is `N * T::BITS`.
///
/// # Examples
///
/// ```
/// use boolvec::BitArray;
///
/// let mut bits = BitArray::<u8, 2>::new();
///
/// bits.set(9, true);
/// assert_eq!(bits.get(9), true);
///
/// assert_eq!(bits.get(0), false);
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct BitArray<T: _priv::BitInner, const N: usize> {
    pub array: [T; N],
}

macro_rules! impl_all {
    ($($ty:ty),*) => {
        $(
            impl _priv::BitInner for $ty {
                const DEFAULT: Self = 0;
            }

            impl<const N: usize> BitArray<$ty, N> {
                #[doc(hidden)]
                const fn index_cal(bit_index: usize) -> (usize, usize) {
                    let bits_in_el = <$ty>::BITS as usize;
                    assert!(bit_index < (N * bits_in_el), "Index out of bounds");

                    (bit_index / bits_in_el, bit_index % bits_in_el)
                }

                #[doc(hidden)]
                const fn reverse_index_cal(array_index: usize, bit_index: usize) -> usize {
                    let bits_in_el = <$ty>::BITS as usize;
                    (array_index * bits_in_el) + bit_index
                }

                /// Gets the value of the bit at the specified index.
                ///
                /// # Arguments
                ///
                /// * `bit_at`: The linear index of the bit to retrieve.
                ///
                /// # Panics
                ///
                /// Panics if `bit_at` is out of bounds (greater than or equal to T::BITS * N).
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bit_array = BitArray::<usize, 10>::new();
                ///
                /// // Get bits
                /// assert_eq!(bit_array.get(0), false);
                /// assert_eq!(bit_array.get(1), false);
                ///
                /// // Change a bit
                /// bit_array.set(0, true);
                ///
                /// // You can see that bit `0` has changed after the set
                /// assert_eq!(bit_array.get(0), true);
                /// assert_eq!(bit_array.get(1), false);
                /// ```
                pub const fn get(&self, bit_at: usize) -> bool {
                    let (array_index, bit_index) = Self::index_cal(bit_at);
                    self.array[array_index] & (1 << bit_index) != 0
                }

                /// Sets the value of the bit at the specified index.
                ///
                /// # Arguments
                ///
                /// * `bit_at`: The linear index of the bit to set (0-based).
                /// * `bit_of`: The value to set the bit to (`true` for 1, `false` for 0).
                ///
                /// # Panics
                ///
                /// Panics if `bit_at` is out of bounds (greater than or equal to T::BITS * N).
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bit_array = BitArray::<usize, 10>::new();
                ///
                /// // Get bits
                /// assert_eq!(bit_array.get(0), false);
                /// assert_eq!(bit_array.get(1), false);
                ///
                /// // Change a bit
                /// bit_array.set(0, true);
                ///
                /// // You can see that bit `0` has changed after the set
                /// assert_eq!(bit_array.get(0), true);
                /// assert_eq!(bit_array.get(1), false);
                /// ```
                pub const fn set(&mut self, bit_at: usize, bit_of: bool) {
                    let (array_index, bit_index) = Self::index_cal(bit_at);

                    if bit_of {
                        self.array[array_index] |= (1 << bit_index);
                    } else {
                        self.array[array_index] &= !(1 << bit_index);
                    }
                }

                /// Sets all bits in the `BitArray` to the specified value.
                ///
                /// # Arguments
                ///
                /// * `bit_of`: The value to set all bits to.
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bit_array = BitArray::<u8, 2>::new();
                ///
                /// // Set all bits to true
                /// bit_array.set_all(true);
                /// assert!(bit_array.get(0));
                /// assert!(bit_array.get(15));
                ///
                /// // Set all bits back to false
                /// bit_array.set_all(false);
                /// assert!(!bit_array.get(0));
                /// assert!(!bit_array.get(15));
                /// ```
                pub const fn set_all(&mut self, bit_of: bool) {
                    // This function has to be kinda weird to be 'const' otherwise I would just use .fill
                    let pattern = if bit_of { <$ty>::MAX } else { 0 };

                    let mut i = 0;
                    while i < N {
                        self.array[i] = pattern;
                        i += 1;
                    }
                }

                /// Finds the index of the first occurrence of a bit with the specified value.
                ///
                /// Searches the entire `BitArray` from the beginning (index 0).
                ///
                /// # Arguments
                ///
                /// * `bit_of`: The bit value to search for.
                ///
                /// # Returns
                ///
                /// * `Some(index)` containing the 0-based linear index of the first matching bit found.
                /// * `None` if no bit with the specified value is found in the array.
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bit_array = BitArray::<u8, 2>::new();
                /// bit_array.set(5, true);
                /// bit_array.set(10, true);
                ///
                /// assert_eq!(bit_array.find_first_of(true), Some(5));
                /// assert_eq!(bit_array.find_first_of(false), Some(0));
                ///
                /// // If we remove all `true` bits from the array, `find_first_of` will return `None`.
                /// bit_array.set(5, false);
                /// bit_array.set(10, false);
                /// assert_eq!(bit_array.find_first_of(true), None);
                /// ```
                pub const fn find_first_of(&self, bit_of: bool) -> Option<usize> {
                    self.find_first_skipping(0, bit_of)
                }

                /// Finds the index of the first occurrence of a bit with the specified value,
                /// starting the search from `start_index`.
                ///
                /// The search includes the bit at `start_index` itself.
                ///
                /// # Arguments
                ///
                /// * `start_index`: The 0-based linear index from where to begin the search (inclusive).
                /// * `bit_of`: The bit value to search for.
                ///
                /// # Returns
                ///
                /// * `Some(index)` containing the 0-based linear index of the first matching bit found
                ///   at or after `start_index`.
                /// * `None` if no matching bit is found at or after `start_index`.
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bits = BitArray::<u8, 2>::new();
                /// bits.set(5, true);
                /// bits.set(10, true);
                ///
                /// assert_eq!(bits.find_first_skipping(0, true), Some(5));
                /// assert_eq!(bits.find_first_skipping(6, true), Some(10));
                /// assert_eq!(bits.find_first_skipping(11, true), None);
                /// assert_eq!(bits.find_first_skipping(0, false), Some(0));
                /// ```
                pub const fn find_first_skipping(&self, start_index: usize, bit_of: bool) -> Option<usize> {
                    let mut bit_offset = start_index;

                    while bit_offset < (N * <$ty>::BITS as usize) {
                        let (array_index, mut inner_index) = Self::index_cal(bit_offset);
                        let array_element = self.array[array_index];

                        if (bit_of && array_element == 0) || (!bit_of && array_element == <$ty>::MAX) {
                            bit_offset = (<$ty>::BITS as usize) * (array_index + 1);
                            continue;
                        }

                        while inner_index < <$ty>::BITS as usize {
                            if array_element & (1 << inner_index) == ((bit_of as $ty) << inner_index) {
                                return Some(Self::reverse_index_cal(array_index, inner_index));
                            }

                            inner_index += 1;
                        }

                        bit_offset = (<$ty>::BITS as usize) * (array_index + 1);
                    }

                    None
                }

                /// Finds the starting index of the first contiguous sequence of `amount` bits
                /// that all have the specified value (`bit_of`).
                ///
                /// Searches the entire `BitArray` from the beginning (equal to the result
                /// of `find_first_of_many_skipping(0, bit_of, amount)`).
                ///
                /// # Arguments
                ///
                /// * `bit_of`: The bit value that all bits in the sequence must have.
                /// * `amount`: The required number of consecutive matching bits.
                ///
                /// # Returns
                ///
                /// * `Some(index)` containing the 0-based linear starting index of the first sequence found.
                /// * `None` if no such sequence of the required length and value exists.
                ///
                /// # Edge Cases
                ///
                /// * If `amount` is larger than the total number of bits in the array, this will always return `None`.
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bits = BitArray::<u8, 2>::new();
                /// bits.set(4, true);
                /// bits.set(5, true);
                /// bits.set(6, true);
                /// bits.set(10, true);
                /// bits.set(11, true);
                ///
                /// assert_eq!(bits.find_first_of_many(true, 3), Some(4));
                /// assert_eq!(bits.find_first_of_many(true, 2), Some(4));
                /// assert_eq!(bits.find_first_of_many(true, 4), None);
                /// assert_eq!(bits.find_first_of_many(false, 4), Some(0));
                /// ```
                pub const fn find_first_of_many(&self, bit_of: bool, amount: usize) -> Option<usize> {
                    self.find_first_of_many_skipping(0, bit_of, amount)
                }


                /// Finds the starting index of the first contiguous sequence of `amount` bits
                /// that all have the specified value (`bit_of`), starting the search from `start_index`.
                ///
                /// The search considers sequences starting at or after `start_index`.
                ///
                /// # Arguments
                ///
                /// * `start_index`: The 0-based linear index from where to begin the search for the start of a sequence.
                /// * `bit_of`: The bit value that all bits in the sequence must have.
                /// * `amount`: The required number of consecutive matching bits.
                ///
                /// # Returns
                ///
                /// * `Some(index)` containing the 0-based linear starting index of the first sequence found
                ///   at or after `start_index`.
                /// * `None` if no such sequence is found at or after `start_index`.
                ///
                /// # Panics
                ///
                /// Panics if `bit_at` is out of bounds (greater than or equal to T::BITS * N).
                ///
                /// # Edge Cases
                ///
                /// * If `amount` is 0, the behavior might vary (check implementation).
                /// * If `start_index + amount` exceeds the total number of bits, this might return `None` early.
                ///
                /// # Examples
                ///
                /// ```
                /// use boolvec::BitArray;
                ///
                /// let mut bits = BitArray::<u8, 2>::new();
                ///
                /// bits.set(4, true);
                /// bits.set(5, true);
                /// bits.set(6, true);
                /// bits.set(10, true);
                /// bits.set(11, true);
                /// bits.set(12, true);
                ///
                /// assert_eq!(bits.find_first_of_many_skipping(0, true, 3), Some(4));
                /// assert_eq!(bits.find_first_of_many_skipping(5, true, 3), Some(10));
                /// assert_eq!(bits.find_first_of_many_skipping(11, true, 3), None);
                /// assert_eq!(bits.find_first_of_many_skipping(11, true, 2), Some(11));
                /// assert_eq!(bits.find_first_of_many_skipping(0, false, 3), Some(0));
                /// ```
                pub const fn find_first_of_many_skipping(&self, start_index: usize, bit_of: bool, amount: usize) -> Option<usize> {
                    // Too many bits to fit in our array
                    if amount >= (N * <$ty>::BITS as usize) {
                        return None;
                    }

                    let mut starting_point = match self.find_first_skipping(start_index, bit_of) {
                        Some(s) => s,
                        None => return None,
                    };

                    let mut bit_offset = starting_point + 1;
                    let mut count = 1;

                    while bit_offset < (N * <$ty>::BITS as usize) {
                        if count >= amount {
                            return Some(starting_point);
                        }

                        let (array_index, inner_index) = Self::index_cal(bit_offset);
                        let array_el = self.array[array_index];

                        if array_el & (1 << inner_index) != (bit_of as $ty) << inner_index {
                            starting_point = match self.find_first_skipping(bit_offset, bit_of) {
                                Some(s) => s,
                                None => return None,
                            };

                            bit_offset = starting_point + 1;
                            count = 1;
                        } else {
                            count += 1;
                            bit_offset += 1;
                        }
                    }

                    None
                }
            }

            impl<const N: usize> From<&[$ty; N]> for BitArray<$ty, N> {
                fn from(value: &[$ty; N]) -> Self {
                    Self {
                        array: value.clone(),
                    }
                }
            }

            impl<const N: usize> From<[$ty; N]> for BitArray<$ty, N> {
                fn from(value: [$ty; N]) -> Self {
                    Self { array: value }
                }
            }

            impl<const N: usize> From<&mut [$ty; N]> for BitArray<$ty, N> {
                fn from(value: &mut [$ty; N]) -> Self {
                    Self {
                        array: value.clone(),
                    }
                }
            }
        )*
    };
}

#[doc(hidden)]
mod _priv {
    // Stop gap until we get const traits
    pub trait BitInner {
        const DEFAULT: Self;
    }
}

impl_all! { u8, u16, u32, u64, u128, usize }

impl<T: _priv::BitInner, const N: usize> BitArray<T, N> {
    /// Creates a new `BitArray` with all bits initialized to `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use boolvec::BitArray;
    ///
    /// let mut my_bits = BitArray::<usize, 10>::new();
    ///
    /// assert_eq!(my_bits.get(0), false);
    /// assert_eq!(my_bits.get(127), false);
    ///
    /// my_bits.set(1, true);
    /// my_bits.set(10, true);
    ///
    /// assert_eq!(my_bits.get(0), false);
    /// assert_eq!(my_bits.get(1), true);
    /// assert_eq!(my_bits.get(10), true);
    /// assert_eq!(my_bits.get(127), false);
    /// ```
    pub const fn new() -> Self {
        Self {
            array: [<T as _priv::BitInner>::DEFAULT; N],
        }
    }
}

// Test cases for BitArray
//
// We don't need to use #[test] because the entire struct is designed to be done
// in 'const' contexts!
const _: () = {
    // I want to make sure we allow for infering the type
    let _infer_bit_type: BitArray<u8, 1> = BitArray::new();

    let mut bits = BitArray::<u8, 2>::new();

    bits.set(0, true);
    assert!(bits.array[0] == 1);

    bits.set(0, false);
    assert!(bits.array[0] == 0);

    bits.set_all(true);
    assert!(bits.array[0] == 0xFF);
    assert!(bits.array[1] == 0xFF);

    bits.set_all(false);
    assert!(bits.array[0] == 0x00);
    assert!(bits.array[1] == 0x00);

    let mut n = 0;
    while n < 16 {
        bits.set(n, true);
        assert!(bits.find_first_of(true).unwrap() == n);
        bits.set(n, false);
        n += 1;
    }

    let mut bits = BitArray::<u128, 2>::new();

    bits.set(0, true);
    assert!(bits.array[0] == 1);

    bits.set(0, false);
    assert!(bits.array[0] == 0);

    bits.set_all(true);
    assert!(bits.array[0] == u128::MAX);
    assert!(bits.array[1] == u128::MAX);

    bits.set_all(false);
    assert!(bits.array[0] == 0x00);
    assert!(bits.array[1] == 0x00);

    let mut n = 0;
    while n < 256 {
        bits.set(n, true);
        assert!(bits.find_first_of(true).unwrap() == n);
        bits.set(n, false);
        n += 1;
    }

    bits.set(0, false);
    bits.set(1, true);
    bits.set(2, true);
    bits.set(3, true);
    bits.set(4, true);
    bits.set(5, true);
    bits.set(6, false);
    bits.set(7, true);
    bits.set(8, true);
    bits.set(9, true);
    bits.set(10, false);
    bits.set(11, false);

    assert!(bits.find_first_of(false).unwrap() == 0);
    assert!(bits.find_first_of(true).unwrap() == 1);

    assert!(bits.find_first_skipping(1, false).unwrap() == 6);
    assert!(bits.find_first_skipping(2, false).unwrap() == 6);
    assert!(bits.find_first_skipping(1, true).unwrap() == 1);
    assert!(bits.find_first_skipping(2, true).unwrap() == 2);

    assert!(bits.find_first_of_many(true, 5).unwrap() == 1);
    assert!(bits.find_first_of_many(false, 1).unwrap() == 0);
    assert!(bits.find_first_of_many(false, 2).unwrap() == 10);
};
