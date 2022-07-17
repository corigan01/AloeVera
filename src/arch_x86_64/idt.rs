/*
  ____                 __               __ __                 __
 / __ \__ _____ ____  / /___ ____ _    / //_/__ _______  ___ / /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / ,< / -_) __/ _ \/ -_) /
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /_/|_|\__/_/ /_//_/\__/_/
  Part of the Quantum OS Kernel

Copyright 2022 Gavin Kellam

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

use crate::arch_x86_64::CpuPrivilegeLevel;
use crate::memory::VirtualAddress;
use crate::{serial_print, serial_println};
use crate::bitset::BitSet;
use x86_64::instructions::segmentation;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::{PrivilegeLevel, VirtAddr};

pub type HandlerFunc = extern "x86-interrupt" fn();

pub struct Idt([Entry; 16]);

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

impl Entry {
    pub fn new(gdt_select: SegmentSelector, handler: HandlerFunc) -> Self {
        let pointer = handler as u64;
        Entry {
            gdt_selector: gdt_select,
            pointer_low: pointer as u16,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            options: EntryOptions::new(),
            reserved: 0,
        }
    }

    pub fn missing() -> Self {
        Entry {
            gdt_selector: SegmentSelector::new(0, PrivilegeLevel::Ring0),
            pointer_low: 0,
            pointer_middle: 0,
            pointer_high: 0,
            options: EntryOptions::new_minimal(),
            reserved: 0,
        }
    }
}

impl Idt {
    pub fn new() -> Idt {
        Idt([Entry::missing(); 16])
    }

    pub fn set_handler(&mut self, entry: u8, handler: HandlerFunc){
        self.0[entry as usize] = Entry::new(segmentation::cs(), handler);
    }

    pub fn load(&self) {
        use x86_64::instructions::tables::{DescriptorTablePointer, lidt};
        use core::mem::size_of;

        let ptr = DescriptorTablePointer {
            base: VirtAddr::new(self as *const _ as u64),
            limit: (size_of::<Self>() - 1) as u16,
        };

        unsafe { lidt(&ptr) };
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EntryOptions(u16);

impl EntryOptions {
    /// # Warning
    /// This has the "Must be 1-bits" **unset**! Meaning that you must set these bits before use or
    /// you risk having undefined behavior.
    pub unsafe fn new_zero() -> Self {
        EntryOptions(0)
    }

    pub fn new_minimal() -> Self {
        EntryOptions(0.set_bit_range(9..12, 0b111))
    }

    pub fn new() -> Self {
        let mut new_s = Self::new_minimal();

        // set the default options for the struct that the user might want
        new_s
            .set_cpu_prv(CpuPrivilegeLevel::RING0)
            .enable_int(false)
            .set_present_flag(true);

        new_s
    }

    pub fn set_present_flag(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    pub fn enable_int(&mut self, enable: bool) -> &mut Self {
        self.0.set_bit(8, enable);
        self
    }

    pub fn set_cpu_prv(&mut self, cpl: CpuPrivilegeLevel) -> &mut Self {
        self.0.set_bit_range(13..15, cpl as u64);
        self
    }

    pub fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0.set_bit_range(0..3, index as u64);
        self
    }
}

#[cfg(test)]
mod test_case {
    use core::arch::asm;
    use crate::arch_x86_64::idt::EntryOptions;
    use lazy_static::lazy_static;
    use crate::serial_println;

    extern "x86-interrupt" fn TEST_divide_by_zero_handler() {
        // We want this to be called and returned!
    }

    use crate::arch_x86_64::idt::Idt;

    lazy_static! {
        static ref IDT_TEST: Idt = {
            let mut idt = Idt::new();

            idt.set_handler(0, TEST_divide_by_zero_handler);

            idt
        };
    }

    fn divide_by_zero_TEST() {
        unsafe {
            asm!("int $0x0");
        }
    }

    #[test_case]
    fn test_handler_by_fault() {
        IDT_TEST.load();

        divide_by_zero_TEST();

        // [OK] We passed!
    }

    #[test_case]
    fn test_entry_options() {
        unsafe { assert_eq!(EntryOptions::new_zero().0, 0x00); }
        assert_eq!(EntryOptions::new_minimal().0, 0xE00);
        assert_eq!(EntryOptions::new().0, 0x8E00);
        assert_ne!(EntryOptions::new().set_present_flag(false).0, EntryOptions::new().0);
        assert_ne!(EntryOptions::new().0, EntryOptions::new_minimal().0);
    }
}

