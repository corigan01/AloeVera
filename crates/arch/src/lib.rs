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
#![feature(abi_x86_interrupt)]

pub mod gdt;
pub mod idt64;
pub mod io;
pub mod locks;
pub mod paging64;
pub mod pic8259;
pub mod pit825x;
pub mod registers;
pub mod supports;
pub mod tss64;

#[cfg(target_pointer_width = "64")]
pub mod processor;

pub mod interrupts {
    #[inline(always)]
    pub fn are_interrupts_enabled() -> bool {
        super::registers::eflags::is_interrupts_enable_set()
    }

    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        core::arch::asm!("sti");
    }

    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        core::arch::asm!("cli");
    }

    pub fn assert_interrupts(enabled: bool) {
        let int_state = crate::registers::eflags::is_interrupts_enable_set();
        assert_eq!(
            int_state,
            enabled,
            "Interrupts were {} when expected them to be {}",
            if int_state { "on" } else { "off" },
            if enabled { "on" } else { "off" }
        );
    }

    #[macro_export]
    macro_rules! critcal_section {
        ($($tt:tt)*) => {{
            let _priv_interrupt_before_state =
                ::arch::registers::eflags::is_interrupts_enable_set();

            if _priv_interrupt_before_state {
                unsafe { ::arch::interrupts::disable_interrupts() };
            }

            let r = {
                $($tt)*
            };

            if _priv_interrupt_before_state {
                unsafe { ::arch::interrupts::enable_interrupts() };
            }

            r
        }};
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
