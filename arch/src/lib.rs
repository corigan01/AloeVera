#![no_std]

pub mod registers;

pub mod interrupts {
    pub unsafe fn enable_interrupts() {
        core::arch::asm!("cli");
    }

    pub unsafe fn disable_interrupts() {
        core::arch::asm!("sti");
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
}
