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

use ultraviolet::constvec::ConstVec;

use super::{PhysEntryKind, PhysMemoryDescriptor};
use crate::addr::PhysAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysMemoryEntry {
    kind: PhysEntryKind,
    start: PhysAddr,
    end: PhysAddr,
}

impl PhysMemoryDescriptor for PhysMemoryEntry {
    fn phys_kind(&self) -> PhysEntryKind {
        self.kind
    }

    fn phys_start(&self) -> PhysAddr {
        self.start
    }

    fn phys_end(&self) -> PhysAddr {
        self.end
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PhysMemoryBorder {
    lhs_kind: PhysEntryKind,
    splice_point: PhysAddr,
}

impl PhysMemoryBorder {
    pub const fn null() -> Self {
        Self {
            lhs_kind: PhysEntryKind::None,
            splice_point: PhysAddr::null_unaligned(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PhysMemoryMap<const N_BORDERS: usize> {
    array: ConstVec<N_BORDERS, PhysMemoryBorder>,
}

impl<const N_BORDERS: usize> PhysMemoryMap<N_BORDERS> {
    pub const fn new() -> Self {
        Self {
            array: ConstVec::new(),
        }
    }

    pub fn insert<D>(&mut self, desc: D)
    where
        D: PhysMemoryDescriptor,
    {
        todo!()
    }
}
