/*
  ____                 __               __   _ __
 / __ \__ _____ ____  / /___ ____ _    / /  (_) /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / /__/ / _ \
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /____/_/_.__/
    Part of the Quantum OS Project

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

#![no_std]

extern crate alloc;
use alloc::vec::Vec;

mod bitarray;
mod compressedbool;

pub use bitarray::*;
pub use compressedbool::*;

type BackingType = u64;

#[derive(Clone, Debug)]
pub struct BoolVec(Vec<BackingType>);

impl BoolVec {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    const fn index_of(bit_index: usize) -> (usize, usize) {
        (
            bit_index / (BackingType::BITS as usize),
            bit_index % (BackingType::BITS as usize),
        )
    }

    const fn recombine_to(array_index: usize, bit_index: usize) -> usize {
        (array_index * (BackingType::BITS as usize)) + bit_index
    }

    pub fn set(&mut self, index: usize, state: bool) {
        let (array_idx, bit_idx) = Self::index_of(index);

        // If the index is larger then the vec has capacity for
        // and that the state is false, we don't need to expand the
        // array.
        if array_idx >= self.0.capacity() && !state {
            return;
        }

        // Expand our vector until we hit array_idx
        let before_len = self.0.len();
        for _ in before_len..=array_idx {
            self.0.push(0);
        }

        if state {
            self.0[array_idx] |= 1 << bit_idx;
        } else {
            self.0[array_idx] &= !(1 << bit_idx);
        }
    }

    pub fn set_many(&mut self, start: usize, len: usize, state: bool) {
        for bit in start..(start + len) {
            self.set(bit, state);
        }
    }

    pub fn get(&self, index: usize) -> bool {
        let (array_idx, bit_idx) = Self::index_of(index);

        if array_idx >= self.0.len() {
            return false;
        }

        self.0[array_idx] & (1 << bit_idx) != 0
    }

    pub fn find_first_of(&self, state: bool) -> Option<usize> {
        if state {
            // Find a 1 in the array
            for (array_idx, el) in self.0.iter().enumerate() {
                if *el != 0 {
                    for bit_idx in 0..BackingType::BITS {
                        if *el & (1 << bit_idx) != 0 {
                            return Some(Self::recombine_to(array_idx, bit_idx as usize));
                        }
                    }
                }
            }

            return None;
        } else {
            // Find a 0 in the array
            for (array_idx, el) in self.0.iter().enumerate() {
                if *el != BackingType::MAX {
                    for bit_idx in 0..BackingType::BITS {
                        if *el & (1 << bit_idx) == 0 {
                            return Some(Self::recombine_to(array_idx, bit_idx as usize));
                        }
                    }
                }
            }

            return Some(Self::recombine_to(self.0.len(), 0));
        }
    }

    pub fn find_first_of_many(&self, state: bool, count: usize) -> Option<usize> {
        // FIXME: There is a better way to do this, but I need this working. Please
        //        refactor to be less painful to look at :)
        if count == 0 {
            return None;
        }

        let mut found_index = None;

        if state {
            // Find a 1 in the array
            for (array_idx, el) in self.0.iter().enumerate() {
                if *el != 0 {
                    for bit_idx in 0..BackingType::BITS {
                        if *el & (1 << bit_idx) != 0 {
                            if found_index.is_none() {
                                found_index = Some(Self::recombine_to(array_idx, bit_idx as usize));
                            }
                        } else {
                            found_index = None;
                        }

                        if let Some(prev_index) = found_index {
                            if Self::recombine_to(array_idx, bit_idx as usize) - prev_index
                                >= count - 1
                            {
                                return found_index;
                            }
                        }
                    }
                }
            }
            return found_index;
        } else {
            // Find a 0 in the array
            for (array_idx, el) in self.0.iter().enumerate() {
                if *el != BackingType::MAX {
                    for bit_idx in 0..BackingType::BITS {
                        if *el & (1 << bit_idx) == 0 {
                            if found_index.is_none() {
                                found_index = Some(Self::recombine_to(array_idx, bit_idx as usize));
                            }
                        } else {
                            found_index = None;
                        }

                        if let Some(prev_index) = found_index {
                            if Self::recombine_to(array_idx, bit_idx as usize) - prev_index
                                >= count - 1
                            {
                                return found_index;
                            }
                        }
                    }
                }
            }

            return Some(Self::recombine_to(self.0.len(), 0));
        }
    }

    pub fn find_first_of_many_skipping(
        &self,
        skip: usize,
        state: bool,
        count: usize,
    ) -> Option<usize> {
        // FIXME: There is a better way to do this, but I need this working. Please
        //        refactor to be less painful to look at :)
        if count == 0 {
            return None;
        }

        let mut found_index = None;
        let (array_offset, mut bit_offset) = Self::index_of(skip);

        if state {
            // Find a 1 in the array
            for (array_idx, el) in self.0.iter().skip(array_offset).enumerate() {
                if *el != 0 {
                    for bit_idx in (bit_offset as u32)..BackingType::BITS {
                        bit_offset = 0;
                        if *el & (1 << bit_idx) != 0 {
                            if found_index.is_none() {
                                found_index = Some(Self::recombine_to(array_idx, bit_idx as usize));
                            }
                        } else {
                            found_index = None;
                        }

                        if let Some(prev_index) = found_index {
                            if Self::recombine_to(array_idx, bit_idx as usize) - prev_index
                                >= count - 1
                            {
                                return found_index;
                            }
                        }
                    }
                }
            }
            return found_index;
        } else {
            // Find a 0 in the array
            for (array_idx, el) in self.0.iter().skip(array_offset).enumerate() {
                if *el != BackingType::MAX {
                    for bit_idx in (bit_offset as u32)..BackingType::BITS {
                        bit_offset = 0;
                        if *el & (1 << bit_idx) == 0 {
                            if found_index.is_none() {
                                found_index = Some(Self::recombine_to(array_idx, bit_idx as usize));
                            }
                        } else {
                            found_index = None;
                        }

                        if let Some(prev_index) = found_index {
                            if Self::recombine_to(array_idx, bit_idx as usize) - prev_index
                                >= count - 1
                            {
                                return found_index;
                            }
                        }
                    }
                }
            }

            return Some(Self::recombine_to(self.0.len(), 0));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_putting_ones_into_array() {
        let mut v = BoolVec::new();

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
        let mut v = BoolVec::new();

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
        let mut v = BoolVec::new();

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
        let mut v = BoolVec::new();

        v.set(7043, true);

        assert_eq!(v.find_first_of(true), Some(7043));
    }

    #[test]
    fn test_find_first_of_many() {
        let mut v = BoolVec::new();

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
}
