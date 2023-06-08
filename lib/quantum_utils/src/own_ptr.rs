/*
  ____                 __               __   _ __
 / __ \__ _____ ____  / /___ ____ _    / /  (_) /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / /__/ / _ \
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /____/_/_.__/
  Part of the Quantum OS Project

Copyright 2023 Gavin Kellam

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

use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

pub struct OwnPtr<Type: ?Sized> {
    ptr: NonNull<Type>,
    ph: PhantomData<Type>
}

impl<Type: Sized> OwnPtr<Type> {
    pub fn empty() -> Self {
        unsafe { Self::new_unchecked(mem::align_of::<Type>() as *mut Type) }
    }
}

impl<Type: ?Sized> OwnPtr<Type> {
    pub fn new(ptr: *mut Type) -> Option<Self> {
        Some(Self {
            ptr: NonNull::new(ptr)?,
            ph: Default::default(),
        })
    }

    pub unsafe fn new_unchecked(ptr: *mut Type) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            ph: PhantomData::default()
        }
    }

    pub fn as_ptr(&self) -> NonNull<Type> {
        self.ptr
    }

    pub unsafe fn as_ref(&self) -> &Type {
        self.ptr.as_ref()
    }

    pub unsafe fn as_mut(&mut self) -> &mut Type {
        self.ptr.as_mut()
    }
}