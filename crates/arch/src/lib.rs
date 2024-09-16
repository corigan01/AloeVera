/*
  ____                 __               __   _ __
 / __ \__ _____ ____  / /___ ____ _    / /  (_) /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / /__/ / _ \
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /____/_/_.__/
    Part of the Quantum OS Project

Copyright 2024 Gavin Kellam

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

pub mod io;
pub mod registers;

pub mod interrupts {
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        core::arch::asm!("sti");
    }

    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        core::arch::asm!("cli");
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CpuPrivilege {
    Ring0,
    Ring1,
    Ring2,
    Ring3,
}

impl Into<u16> for CpuPrivilege {
    fn into(self) -> u16 {
        match self {
            CpuPrivilege::Ring0 => 0,
            CpuPrivilege::Ring1 => 1,
            CpuPrivilege::Ring2 => 2,
            CpuPrivilege::Ring3 => 3,
        }
    }
}

pub mod stack {
    #[inline(always)]
    #[cfg(target_pointer_width = "64")]
    pub fn stack_ptr() -> usize {
        let value: u64;
        unsafe {
            core::arch::asm!("mov {0}, rsp", out(reg) value);
        }

        value as usize
    }

    #[inline(always)]
    #[cfg(target_pointer_width = "32")]
    pub fn stack_ptr() -> usize {
        let value: u32;
        unsafe {
            core::arch::asm!("mov {0}, esp", out(reg) value);
        }

        value as usize
    }

    #[inline(always)]
    pub unsafe fn align_stack() {
        #[cfg(target_pointer_width = "32")]
        core::arch::asm!("and esp, 0xffffff00");
        #[cfg(target_pointer_width = "64")]
        core::arch::asm!("and rsp, 0xffffffffffffff00");
    }

    #[inline(always)]
    pub unsafe fn push_stack(value: usize) {
        #[cfg(target_pointer_width = "32")]
        core::arch::asm!("push {}", in(reg) (value as u32));
        #[cfg(target_pointer_width = "64")]
        core::arch::asm!("push {}", in(reg) (value as u64));
    }
}
