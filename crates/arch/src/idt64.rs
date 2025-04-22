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

use crate::{
    registers::{cr2, ProcessContext, Segment},
    CpuPrivilege,
};
pub use arch_macro::interrupt;
use core::fmt::Debug;

#[derive(Clone, Copy, Debug)]
pub enum GateKind {
    TaskGate,
    InterruptGate16,
    TrapGate16,
    InterruptGate,
    TrapGate,
}

#[repr(C, align(4096))]
#[derive(Clone, Copy, Debug)]
pub struct InterruptDescTable([GateDescriptor; 256]);

impl InterruptDescTable {
    pub const fn new() -> Self {
        Self([GateDescriptor::zero(); 256])
    }

    pub const fn attach_raw(&mut self, irq: u8, gate: GateDescriptor) {
        self.0[irq as usize] = gate;
    }

    pub fn submit_table(&self) -> IdtPointer {
        IdtPointer {
            limit: 255 * 16 - 1,
            offset: self.0.as_ptr() as u64,
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct IdtPointer {
    limit: u16,
    offset: u64,
}

impl IdtPointer {
    pub unsafe fn load(self) {
        unsafe { core::arch::asm!("lidt [{}]", in(reg) &self) }
    }
}

#[bits::bits(
    field(RW, 0..=15, offset_1),
    field(RW, 16..=31, segment_selector),
    field(RW, 32..=34, pub ist),
    field(RW, 40..=43, gate_type),
    field(RW, 45..=46, dpl),
    field(RW, 47, pub present),
    field(RW, 48..=63, offset_2),
    field(RW, 64..=95, offset_3),
)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GateDescriptor(u128);

impl Debug for GateDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GateDescriptor")
            .field("offset", &self.get_offset())
            .field("segment", &self.get_segment_selector())
            .field("ist", &self.get_ist())
            .field("gate_type", &self.get_gate_kind())
            .field("dpl", &self.get_privilege())
            .field("present", &self.is_present_set())
            .finish()
    }
}

impl GateDescriptor {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn set_offset(&mut self, offset: u64) {
        self.set_offset_1((offset & (u16::MAX as u64)) as u16);
        self.set_offset_2(((offset >> 16) & (u16::MAX as u64)) as u16);
        self.set_offset_3(((offset >> 32) & (u32::MAX as u64)) as u32);
    }

    pub const fn get_offset(&self) -> u64 {
        (self.get_offset_1() as u64)
            | ((self.get_offset_2() as u64) << 16)
            | ((self.get_offset_3() as u64) << 32)
    }

    pub const fn set_gate_kind(&mut self, kind: GateKind) {
        let gate_num = match kind {
            GateKind::TaskGate => 0x5,
            GateKind::InterruptGate16 => 0x6,
            GateKind::TrapGate16 => 0x7,
            GateKind::InterruptGate => 0xE,
            GateKind::TrapGate => 0xF,
        };

        self.set_gate_type(gate_num);
    }

    pub const fn get_gate_kind(&self) -> GateKind {
        match self.get_gate_type() {
            0x5 => GateKind::TaskGate,
            0x6 => GateKind::InterruptGate16,
            0x7 => GateKind::TrapGate16,
            0xE => GateKind::InterruptGate,
            0xF => GateKind::TrapGate,
            _ => unreachable!(),
        }
    }

    pub const fn set_privilege(&mut self, dpl: CpuPrivilege) {
        let dpl_num = match dpl {
            CpuPrivilege::Ring0 => 0,
            CpuPrivilege::Ring1 => 1,
            CpuPrivilege::Ring2 => 2,
            CpuPrivilege::Ring3 => 3,
        };

        self.set_dpl(dpl_num);
    }

    pub const fn get_privilege(&self) -> CpuPrivilege {
        match self.get_dpl() {
            0 => CpuPrivilege::Ring0,
            1 => CpuPrivilege::Ring1,
            2 => CpuPrivilege::Ring2,
            3 => CpuPrivilege::Ring3,
            _ => unreachable!(),
        }
    }

    pub const fn set_code_segment(&mut self, segment: Segment) {
        self.set_segment_selector(segment.0);
    }

    pub const fn get_code_segment(&self) -> Segment {
        Segment(self.get_segment_selector())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptFrame {
    pub ip: u64,
    pub code_seg: u64,
    pub flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

pub type RawHandlerFuncNe = extern "x86-interrupt" fn(InterruptFrame);
pub type ExRawHandlerFunc = extern "C" fn();

#[derive(Debug, Clone, Copy)]
pub enum TableSelector {
    Gdt,
    Idt,
    Ldt,
}

impl TableSelector {
    pub fn from_cpu_err(err_code: u64) -> Self {
        match (err_code >> 1) & 0b11 {
            0 => Self::Gdt,
            1 => Self::Idt,
            _ => Self::Ldt,
        }
    }
}

// This is part of the `nice` user facing interrupt info
#[derive(Debug, Clone, Copy)]
pub enum InterruptFlags {
    DivisionError,
    Debug,
    NonMaskableInterrupt,
    Breakpoint,
    Overflow,
    BoundRangeExceeded,
    InvalidOpcode,
    DeviceNotAvailable,
    DoubleFault,
    CoprocessorSegmentOverrun,
    InvalidTSS {
        external: bool,
        table: TableSelector,
        table_index: u16,
    },
    SegmentNotPresent {
        external: bool,
        table: TableSelector,
        table_index: u16,
    },
    StackSegmentFault {
        external: bool,
        table: TableSelector,
        table_index: u16,
    },
    // TODO: This one has an error code, but its complex
    GeneralProtectionFault,
    PageFault {
        present: bool,
        write: bool,
        user: bool,
        reserved_write: bool,
        instruction_fetch: bool,
        protection_key: bool,
        shadow_stack: bool,
        software_guard: bool,
        // filled by cr2
        virt_addr: u64,
    },
    X87FloatingPointException,
    AlignmentCheck,
    MachineCheck,
    SIMDFloatingPointException,
    VirtualizationException,
    ControlProtectionException,
    HypervisorInjectionException,
    VMMCommunicationException,
    SecurityException,
    Reserved,
    TripleFault,
    Irq(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionKind {
    Interrupt,
    Fault,
    Trap,
    Abort,
}

impl InterruptFlags {
    pub const fn exception_kind(&self) -> ExceptionKind {
        match self {
            Self::DivisionError => ExceptionKind::Fault,
            Self::Debug => ExceptionKind::Fault,
            Self::NonMaskableInterrupt => ExceptionKind::Interrupt,
            Self::Breakpoint => ExceptionKind::Trap,
            Self::Overflow => ExceptionKind::Trap,
            Self::BoundRangeExceeded => ExceptionKind::Fault,
            Self::InvalidOpcode => ExceptionKind::Fault,
            Self::DeviceNotAvailable => ExceptionKind::Fault,
            Self::DoubleFault => ExceptionKind::Abort,
            Self::CoprocessorSegmentOverrun => ExceptionKind::Fault,
            Self::InvalidTSS { .. } => ExceptionKind::Fault,
            Self::SegmentNotPresent { .. } => ExceptionKind::Fault,
            Self::StackSegmentFault { .. } => ExceptionKind::Fault,
            Self::GeneralProtectionFault => ExceptionKind::Fault,
            Self::PageFault { .. } => ExceptionKind::Fault,
            Self::X87FloatingPointException => ExceptionKind::Fault,
            Self::AlignmentCheck => ExceptionKind::Fault,
            Self::MachineCheck => ExceptionKind::Abort,
            Self::SIMDFloatingPointException => ExceptionKind::Fault,
            Self::VirtualizationException => ExceptionKind::Fault,
            Self::ControlProtectionException => ExceptionKind::Fault,
            Self::HypervisorInjectionException => ExceptionKind::Fault,
            Self::VMMCommunicationException => ExceptionKind::Fault,
            Self::SecurityException => ExceptionKind::Fault,
            Self::Reserved => ExceptionKind::Abort,
            Self::TripleFault => ExceptionKind::Abort,
            Self::Irq(_) => ExceptionKind::Interrupt,
        }
    }

    fn convert_from_interrupt(irq_id: u8, error: u64) -> Self {
        match irq_id {
            0 => Self::DivisionError,
            1 => Self::Debug,
            2 => Self::NonMaskableInterrupt,
            3 => Self::Breakpoint,
            4 => Self::Overflow,
            5 => Self::BoundRangeExceeded,
            6 => Self::InvalidOpcode,
            7 => Self::DeviceNotAvailable,
            8 => Self::DoubleFault,
            9 => Self::CoprocessorSegmentOverrun,
            10 => Self::InvalidTSS {
                external: error & 1 != 0,
                table: TableSelector::from_cpu_err(error),
                table_index: ((error & (u16::MAX as u64)) >> 3) as u16,
            },
            11 => Self::SegmentNotPresent {
                external: error & 1 != 0,
                table: TableSelector::from_cpu_err(error),
                table_index: ((error & (u16::MAX as u64)) >> 3) as u16,
            },
            12 => Self::StackSegmentFault {
                external: error & 1 != 0,
                table: TableSelector::from_cpu_err(error),
                table_index: ((error & (u16::MAX as u64)) >> 3) as u16,
            },
            13 => Self::GeneralProtectionFault,
            14 => Self::PageFault {
                present: error & (1 << 0) != 0,
                write: error & (1 << 1) != 0,
                user: error & (1 << 2) != 0,
                reserved_write: error & (1 << 3) != 0,
                instruction_fetch: error & (1 << 4) != 0,
                protection_key: error & (1 << 5) != 0,
                shadow_stack: error & (1 << 6) != 0,
                software_guard: error & (1 << 15) != 0,
                virt_addr: cr2::read(),
            },
            16 => Self::X87FloatingPointException,
            17 => Self::AlignmentCheck,
            18 => Self::MachineCheck,
            19 => Self::SIMDFloatingPointException,
            20 => Self::VirtualizationException,
            21 => Self::ControlProtectionException,
            28 => Self::HypervisorInjectionException,
            29 => Self::VMMCommunicationException,
            30 => Self::SecurityException,
            15 | 22..=27 | 31 => Self::Reserved,
            _ => Self::Irq(irq_id),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InterruptInfo<'a> {
    pub context: &'a ProcessContext,
    pub flags: InterruptFlags,
}

impl<'a> InterruptInfo<'a> {
    pub fn convert_from_ne(irq_id: u8, context: *const ProcessContext) -> Self {
        Self {
            context: unsafe { &*context },
            flags: InterruptFlags::convert_from_interrupt(irq_id, 0),
        }
    }

    pub fn convert_from_e(irq_id: u8, context: *const ProcessContext) -> Self {
        let context = unsafe { &*context };
        Self {
            context,
            flags: InterruptFlags::convert_from_interrupt(irq_id, context.exception_code),
        }
    }
}

#[macro_export]
macro_rules! attach_irq {
    ($idt: ident, $irq: ident) => {{
        for (irq_num, handler_ptr) in $irq::IRQ_FUNCTION_E_PTRS {
            // FIXME: We need to allow the caller to define what these should be
            let mut gate = ::arch::idt64::GateDescriptor::zero();
            gate.set_offset(handler_ptr as u64);
            gate.set_code_segment(::arch::registers::Segment::new(
                1,
                ::arch::CpuPrivilege::Ring0,
            ));
            gate.set_privilege(::arch::CpuPrivilege::Ring0);
            gate.set_gate_kind(::arch::idt64::GateKind::InterruptGate);
            gate.set_present_flag(true);

            $idt.attach_raw(irq_num, gate);
        }

        for (irq_num, handler_ptr) in $irq::IRQ_FUNCTION_NE_PTRS {
            // FIXME: We need to allow the caller to define what these should be
            let mut gate = ::arch::idt64::GateDescriptor::zero();
            gate.set_offset(handler_ptr as u64);
            gate.set_code_segment(::arch::registers::Segment::new(
                1,
                ::arch::CpuPrivilege::Ring0,
            ));
            gate.set_privilege(::arch::CpuPrivilege::Ring0);
            gate.set_gate_kind(::arch::idt64::GateKind::InterruptGate);
            gate.set_present_flag(true);

            $idt.attach_raw(irq_num, gate);
        }

        for (irq_num, handler_ptr) in $irq::IRQ_FUNCTION_RESERVED_PTRS {
            let mut gate = ::arch::idt64::GateDescriptor::zero();
            gate.set_offset(handler_ptr as u64);
            gate.set_code_segment(::arch::registers::Segment::new(
                1,
                ::arch::CpuPrivilege::Ring0,
            ));
            gate.set_privilege(::arch::CpuPrivilege::Ring0);
            gate.set_gate_kind(::arch::idt64::GateKind::InterruptGate);
            gate.set_present_flag(true);

            $idt.attach_raw(irq_num, gate);
        }
    }};
}

pub fn fire_debug_int() {
    unsafe { core::arch::asm!("int 0x01") };
}

/// A raw interrupt wrapper macro that does the interrupt
/// stack unwrapping and creates a `ProcessContext`
#[macro_export]
macro_rules! raw_int_asm {
    (NO ERROR: $irq_id: literal, $ident: ident, $calling: ident) => {
        /// Interrupt wrapper (NO ERROR)
        #[unsafe(naked)]
        pub(crate) extern "C" fn $ident() {
            extern "C" fn handler(context: *const $crate::registers::ProcessContext) {
                let info = $crate::idt64::InterruptInfo::convert_from_ne($irq_id, context);
                $calling(&info);
            }

            unsafe {
                core::arch::naked_asm!(
                    r#"
                    .align 16
                    cld
                    push 0
                    push rax
                    push rbx
                    push rcx
                    push rdx
                    push rsi
                    push rdi
                    push rbp
                    push r8
                    push r9
                    push r10
                    push r11
                    push r12
                    push r13
                    push r14
                    push r15

                    mov rdi, rsp
                    call {handler}

                    pop r15
                    pop r14
                    pop r13
                    pop r12
                    pop r11
                    pop r10
                    pop r9
                    pop r8
                    pop rbp
                    pop rdi
                    pop rsi
                    pop rdx
                    pop rcx
                    pop rbx
                    pop rax
                    add rsp, 8                     # pop 0

                    iretq
                "#,
                    handler = sym handler
                );
            }
        }
    };
    (ERROR: $irq_id: literal, $ident: ident, $calling: ident) => {
        /// Interrupt wrapper (YES ERROR)
        #[unsafe(naked)]
        pub(crate) extern "C" fn $ident() {
            extern "C" fn handler(context: *const $crate::registers::ProcessContext) {
                let info = ::arch::idt64::InterruptInfo::convert_from_e($irq_id, context);
                $calling(&info);
            }

            unsafe {
                core::arch::naked_asm!(
                    r#"
                    .align 16
                    cld
                    push rax
                    push rbx
                    push rcx
                    push rdx
                    push rsi
                    push rdi
                    push rbp
                    push r8
                    push r9
                    push r10
                    push r11
                    push r12
                    push r13
                    push r14
                    push r15

                    mov rdi, rsp
                    call {handler}

                    pop r15
                    pop r14
                    pop r13
                    pop r12
                    pop r11
                    pop r10
                    pop r9
                    pop r8
                    pop rbp
                    pop rdi
                    pop rsi
                    pop rdx
                    pop rcx
                    pop rbx
                    pop rax
                    add rsp, 8                     # pop errno

                    iretq
                "#,
                    handler = sym handler
                );
            }
        }

    };
}
