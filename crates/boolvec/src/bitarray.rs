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

pub struct BitArray<T, const N: usize> {
    pub array: [T; N],
}

macro_rules! impl_all {
    ($($ty:ty),*) => {
        $(
            impl<const N: usize> BitArray<$ty, N> {
                pub const fn new() -> Self {
                    Self { array: [0; N] }
                }

                const fn index_cal(bit_index: usize) -> (usize, usize) {
                    let bits_in_el = <$ty>::BITS as usize;
                    assert!(bit_index < (N * bits_in_el), "Index out of bounds");

                    (bit_index / bits_in_el, bit_index % bits_in_el)
                }

                const fn reverse_index_cal(array_index: usize, bit_index: usize) -> usize {
                    let bits_in_el = <$ty>::BITS as usize;
                    (array_index * bits_in_el) + bit_index
                }

                pub const fn get(&self, bit_at: usize) -> bool {
                    let (array_index, bit_index) = Self::index_cal(bit_at);
                    self.array[array_index] & (1 << bit_index) != 0
                }

                pub const fn set(&mut self, bit_at: usize, bit_of: bool) {
                    let (array_index, bit_index) = Self::index_cal(bit_at);

                    if bit_of {
                        self.array[array_index] |= (1 << bit_index);
                    } else {
                        self.array[array_index] &= !(1 << bit_index);
                    }
                }

                pub const fn set_all(&mut self, bit_of: bool) {
                    // This function has to be kinda weird to be 'const' otherwise I would just use .fill
                    let pattern = if bit_of { <$ty>::MAX } else { 0 };

                    let mut i = 0;
                    while i < N {
                        self.array[i] = pattern;
                        i += 1;
                    }
                }

                pub const fn find_first_of(&self, bit_of: bool) -> Option<usize> {
                    self.find_first_skipping(0, bit_of)
                }

                pub const fn find_first_skipping(&self, start_index: usize, bit_of: bool) -> Option<usize> {
                    let mut bit_offset = start_index;

                    while bit_offset < (N * <$ty>::BITS as usize) {
                        let (array_index, mut inner_index) = Self::index_cal(bit_offset);
                        let array_element = self.array[array_index];

                        if (bit_of && array_element == 0) || (!bit_of && array_element == <$ty>::MAX) {
                            bit_offset += <$ty>::BITS as usize;
                            continue;
                        }

                        while inner_index < <$ty>::BITS as usize {
                            if array_element & (1 << inner_index) == ((bit_of as $ty) << inner_index) {
                                return Some(Self::reverse_index_cal(array_index, inner_index));
                            }

                            inner_index += 1;
                        }

                        bit_offset += <$ty>::BITS as usize;
                    }

                    None
                }

                pub const fn find_first_of_many(&self, bit_of: bool, amount: usize) -> Option<usize> {
                    self.find_first_of_many_skipping(0, bit_of, amount)
                }


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

impl_all! { u8, u16, u32, u64, u128, usize }

// Test cases for BitArray
//
// We don't need to use #[test] because the entire struct is designed to be done
// in 'const' contexts!
const _: () = {
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
