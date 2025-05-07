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

use crate::addr::PhysAddr;
use core::fmt::Debug;

pub mod map;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryComponent {
    KernelBin,
    KernelInitStack,
    KernelInitHeap,
    KernelElf,
    Initfs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysEntryKind {
    None,
    Free,
    Reserved,
    Special,
    AcpiReclaimable,
    Component(MemoryComponent),
    Broken,
}

pub trait PhysMemoryDescriptor: PartialOrd + Clone + Debug {
    fn phys_kind(&self) -> PhysEntryKind;
    fn phys_start(&self) -> PhysAddr;
    fn phys_end(&self) -> PhysAddr;

    fn phys_size(&self) -> usize {
        let start = self.phys_start();
        let end = self.phys_end();

        debug_assert!(
            end.get() > start.get(),
            "End address must come after start address!"
        );

        end.addr_distance(start)
    }
}
