/*
  ____                 __               __ __                 __
 / __ \__ _____ ____  / /___ ____ _    / //_/__ _______  ___ / /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / ,< / -_) __/ _ \/ -_) /
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /_/|_|\__/_/ /_//_/\__/_/
  Part of the Quantum OS Kernel

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

use core::{arch::naked_asm, cell::SyncUnsafeCell};

pub static KERNEL_RSP_PTR: SyncUnsafeCell<u64> = SyncUnsafeCell::new(0x121212);
pub static TEMP_USERSPACE_RSP_PTR: SyncUnsafeCell<u64> = SyncUnsafeCell::new(0x121212);

#[unsafe(naked)]
pub unsafe extern "C" fn kernel_entry() {
    unsafe {
        naked_asm!(
            "
            #  -- Save User's stack ptr, and restore our own

            cli
            mov [{userspace_rsp_ptr}], rsp
            mov rsp, [{kernel_rsp_ptr}]
            push [{userspace_rsp_ptr}]
            sti

            #  -- Start building the `ProcessContext`

            push 0x23
            push [{userspace_rsp_ptr}]
            push r11                       # rFLAGS is saved in r11
            push 0x2b
            push rcx                       # rip is saved in rcx
            push 0                         # This isn't an ISR, so we can just store 0 into its 'exception_code'

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

            #  -- Call the 'syscall_handler' function

            mov r9, rax                    # We move rax (syscall number) into r9 (arg5 of callee)
            mov rcx, rsp
            call syscall_handler

            #  -- Start restoring the `ProcessContext`
        
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

            add rsp, 16
            pop rcx                        # rip
            add rsp,  8
            pop r11                        # rflags
            add rsp, 16

            #  -- Return back to Userspace

            pop rsp
            sysretq
        ",
            kernel_rsp_ptr = sym KERNEL_RSP_PTR,
            userspace_rsp_ptr = sym TEMP_USERSPACE_RSP_PTR,
        )
    };
}

/// Set the RSP of the syscall entry
pub unsafe fn set_syscall_rsp(new_rsp: u64) {
    let kernel_rsp_ptr = KERNEL_RSP_PTR.get();

    unsafe { *kernel_rsp_ptr = new_rsp };
}

/// get the RSP of the syscall entry
pub unsafe fn _get_syscall_rsp() -> u64 {
    let kernel_rsp_ptr = KERNEL_RSP_PTR.get();

    unsafe { *kernel_rsp_ptr }
}
