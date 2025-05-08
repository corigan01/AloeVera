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

use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

pub struct ConstVec<const N: usize, T> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<const N: usize, T: Clone> Clone for ConstVec<N, T> {
    fn clone(&self) -> Self {
        let mut data = [const { MaybeUninit::uninit() }; N];

        // Clone each element
        data.iter_mut()
            .take(self.len)
            .zip(self.data.iter())
            .for_each(|(n, p)| *n = MaybeUninit::new(unsafe { p.assume_init_ref() }.clone()));

        Self {
            data,
            len: self.len,
        }
    }
}

impl<const N: usize, T: Copy> Copy for ConstVec<N, T> {}

impl<const N: usize, T> ConstVec<N, T> {
    pub const fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }

    pub const fn as_slice(&self) -> &[T] {
        // Until #96097 fixes this issue
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        // Until #96097 fixes this issue
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    pub const fn as_ptr(&self) -> *const T {
        self.data.as_ptr().cast()
    }

    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr().cast()
    }

    pub const fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len);

        if index + 1 == self.len {
            self.len -= 1;
            unsafe { self.data[self.len + 1].assume_init_read() }
        } else {
            let removed_item = unsafe { self.data[index].assume_init_read() };

            // Take the last element and replace the removed element with it
            let swapping_item = unsafe { self.data[self.len - 1].assume_init_read() };
            self.data[index] = MaybeUninit::new(swapping_item);
            self.len -= 1;

            removed_item
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn insert(&mut self, index: usize, element: T) {
        // Cannot use self.try_insert().unwrap() due to Result<(), T>'s drop being non-const
        if index > self.len {
            panic!("insertion index is beyond end of array");
        }

        if index + 1 > N {
            panic!("insertion index is beyond end of array's length");
        }

        unsafe {
            let splice = self.as_mut_ptr().add(index);
            if index < self.len {
                core::ptr::copy(splice, splice.add(1), self.len - index);
            }

            self.data[index] = MaybeUninit::new(element);
            self.len += 1;
        }
    }

    pub const fn try_insert(&mut self, index: usize, element: T) -> Result<(), T> {
        if index > self.len || index + 1 > N {
            return Err(element);
        }

        unsafe {
            let splice = self.as_mut_ptr().add(index);
            if index < self.len {
                core::ptr::copy(splice, splice.add(1), self.len - index);
            }

            self.data[index] = MaybeUninit::new(element);
            self.len += 1;
        }

        Ok(())
    }

    pub const fn remove(&mut self, index: usize) -> T {
        if index > self.len {
            panic!("removal index is beyond end of array");
        }

        unsafe {
            let read_value = self.data[index].assume_init_read();
            let splice = self.as_mut_ptr().add(index);

            core::ptr::copy(splice.add(1), splice, self.len - index - 1);
            self.len -= 1;

            read_value
        }
    }

    pub const fn push(&mut self, value: T) {
        if self.len + 1 > N {
            panic!("push index is beyond end of array's length");
        }

        self.data[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    pub const fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.len + 1 > N {
            return Err(value);
        }

        self.data[self.len] = MaybeUninit::new(value);
        self.len += 1;

        Ok(())
    }
}

impl<const N: usize, T> Deref for ConstVec<N, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize, T> DerefMut for ConstVec<N, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_array_push() {
        let mut array: ConstVec<8, i32> = ConstVec::new();

        for i in 0..8 {
            array.push(i);
        }

        assert_eq!(array.try_push(9), Err(9));

        for i in 0..8 {
            assert_eq!(array[i], i as i32);
        }
    }

    #[test]
    fn test_array_remove() {
        let mut array: ConstVec<8, i32> = ConstVec::new();

        for i in 0..8 {
            array.push(i);
        }

        assert_eq!(array.len(), 8);

        for i in 0..8 {
            assert_eq!(array.remove(0), i as i32);
        }

        assert_eq!(array.len(), 0);
    }

    #[test]
    #[should_panic]
    fn test_push_past_len() {
        let mut array: ConstVec<8, i32> = ConstVec::new();

        for i in 0..9 {
            array.push(i);
        }
    }
}
