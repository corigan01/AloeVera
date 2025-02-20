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

use alloc::alloc::{alloc_zeroed, dealloc};
use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
};
use mem::addr::{AlignedTo, VirtAddr};
use util::consts::PAGE_4K;

type ArchStackPtr = usize;

/// A task's flags
#[bits::bits(field(RW, 0, running), field(RW, 1, dead))]
#[derive(Debug, Clone, Copy)]
pub struct TaskFlags(u64);

/// The managed stack area for a task
#[derive(Debug)]
pub struct TaskStack {
    stack_bottom: VirtAddr<AlignedTo<{ size_of::<ArchStackPtr>() }>>,
    rsp: ArchStackPtr,
    len: usize,
}

impl TaskStack {
    pub fn new(len: usize) -> Self {
        // FIXME: This should allocate kernel pages instead of using the default alloc
        let allocation = unsafe {
            alloc_zeroed(Layout::from_size_align(len, size_of::<ArchStackPtr>()).unwrap())
        };

        Self {
            stack_bottom: VirtAddr::try_from(allocation).unwrap(),
            rsp: (allocation as ArchStackPtr) + len,
            len,
        }
    }
}

impl Drop for TaskStack {
    fn drop(&mut self) {
        unsafe {
            dealloc(
                self.stack_bottom.as_mut_ptr(),
                Layout::from_size_align(self.len, size_of::<ArchStackPtr>()).unwrap(),
            );
        }
    }
}

/// A unit of execution (kernel only) that is allowed to switch execution to other Tasks though
/// the stack.
#[derive(Debug)]
pub struct Task {
    stack: TaskStack,
    task_flags: TaskFlags,
}

impl Task {
    pub const TASK_DEFAULT_STACK_LEN: usize = PAGE_4K * 8;

    /// Check if this Task is the currently executing task
    pub fn is_current(&self) -> bool {
        let stack_bottom = self.stack.stack_bottom.addr();
        let stack_top = stack_bottom + self.stack.len;
        let current_rsp = unsafe { asm_get_rsp() };

        stack_bottom <= current_rsp && stack_top >= current_rsp
    }

    /// Called immediately before switching
    #[inline(always)]
    fn switch_prelude(&mut self) {
        self.task_flags.set_running_flag(false);
    }

    /// Called immediately after switching
    #[inline(always)]
    fn switch_epilogue(&mut self) {
        self.task_flags.set_running_flag(true);
    }

    /// Get the tasks inner stack ptr
    #[inline]
    fn get_task_stack_ptr(&mut self) -> *mut ArchStackPtr {
        &raw mut self.stack.rsp
    }

    /// Switch from task `from` into task `to`.
    pub unsafe fn switch_task(from: *mut Self, to: *mut Self) {
        unsafe {
            (&mut *from).switch_prelude();
            let from_stack_ptr = (&mut *from).get_task_stack_ptr();
            let to_stack_ptr = (&mut *to).get_task_stack_ptr();

            assert!(
                (&*from).is_current(),
                "Switching, yet current is not correct!"
            );
            asm_switch(from_stack_ptr, to_stack_ptr);
            assert!(
                (&*from).is_current(),
                "Switched back to current, yet current is not correct!"
            );

            (&mut *from).switch_epilogue()
        };
    }

    /// Switch init
    ///
    /// This function switchs to a task for the first time
    pub unsafe fn switch_first(to: *mut Self) -> ! {
        unsafe {
            let to_stack_ptr = (&mut *to).get_task_stack_ptr();
            let mut from_stack_ptr = asm_get_rsp();

            asm_switch(&raw mut from_stack_ptr, to_stack_ptr);

            unreachable!("Cannot return from `switch_first`");
        }
    }

    /// Create a new task that calls `start`
    pub fn new(start: fn()) -> Self {
        let mut new_task = Self {
            stack: TaskStack::new(Self::TASK_DEFAULT_STACK_LEN),
            task_flags: TaskFlags(0),
        };

        let stack_ptr = new_task.get_task_stack_ptr();
        unsafe { asm_init(stack_ptr, start, ret_call_panic) };

        new_task
    }
}

/// This is the function placed at the top of the call stack for new tasks. Since its not
/// called directly, or even part of the execution loop for most functions its just to serve
/// to prevent functions who return (which isn't valid behavior) from going unnoticed and causing
/// werid and hard to debug errors.
///
/// Its just the function Tasks return to when they finish.
pub fn ret_call_panic() -> ! {
    panic!("Should never return from caller");
}

#[inline(always)]
pub unsafe fn asm_get_rsp() -> ArchStackPtr {
    let rsp;
    unsafe { asm!("mov {rsp_ptr}, rsp", rsp_ptr = out(reg) rsp) };
    rsp
}

/// Init a given tasks state
///
/// # Safety
/// This function will switch to the `task` stack during init. The caller must ensure that the
/// `task`'s stack is valid at the time of calling, and can properly be switched to.
///
/// This function also takes a function that will be called first (`init_fn`) when the task is
/// first started. There is an additional argument `ret_call` for saftety when returning from
/// the `init_fn`. It should be impossible for `ret_call` to return, as returning is undefined.
pub unsafe fn asm_init(task: *mut ArchStackPtr, init_fn: fn(), ret_call: fn() -> !) {
    unsafe {
        asm!(
                r#"

            # -- Switch to task's stack

            mov {rsp_save}, rsp
            mov rsp, [{task_rsp}]

            # -- Setup inital frame

            push {ret}    # ret call
            push {init}   # ret init
            push 0        # r15
            push 0        # r14
            push 0        # r13
            push 0        # r12
            push 0        # rbp
            push 0        # rbx

            # -- Restore caller's stack

            mov [{task_rsp}], rsp
            mov rsp, {rsp_save}
        "#,
            task_rsp = in(reg) task,
            init = in(reg) init_fn,
            ret = in(reg) ret_call,
            rsp_save = out(reg) _,
        );
    }
}

/// Switch between two tasks given ptrs to their stack ptrs
#[naked]
pub unsafe extern "C" fn asm_switch(from: *mut ArchStackPtr, to: *const ArchStackPtr) {
    unsafe {
        naked_asm!(
            r#"
            .align 16
            # asm_switch(rdi, rsi, rdx) -> ();
            # struct TaskState {{ rbx, rbp, r12, r13, r14, r15 }}

            # -- Save old task's state

            push r15
            push r14
            push r13
            push r12
            push rbp
            push rbx

            # -- Switch to new task stack

            mov [rdi], rsp
            mov rsp, [rsi]

            # -- Restore old task's state

            pop rbx
            pop rbp
            pop r12
            pop r13
            pop r14
            pop r15

            ret
        "#
        )
    };
}
