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

use hw::make_hw;

/// The max 'bits' of physical memory the system supports.
pub const MAX_PHY_MEMORY_WIDTH: usize = 48;

#[make_hw(
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_accessed),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    field(RW, 6, pub dirty),
    field(RW, 7, pub page_attribute_table),
    field(RW, 8, pub global),
    field(RW, 12..48, pub virt_address),
    field(RW, 59..62, pub protection_key),
    field(RW, 63, pub execute_disable)
)]
#[derive(Clone, Copy)]
pub struct PageEntry4K(u64);

impl PageEntry4K {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero()
    }
}

#[make_hw(
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_accessed),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    field(RW, 6, pub dirty),
    /// For this entry, `page_size` needs to be set to true! 
    field(RW, 7, pub page_size),
    field(RW, 8, pub global),
    field(RW, 12, pub page_attribute_table),
    field(RW, 21..48, pub virt_address),
    field(RW, 59..62, pub protection_key),
    field(RW, 63, pub execute_disable)
)]
#[derive(Clone, Copy)]
pub struct PageEntry2M(u64);

impl PageEntry2M {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero().set_page_size_flag(true)
    }
}

#[make_hw(
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_accessed),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    field(RW, 6, pub dirty),
    /// For this entry, `page_size` needs to be set to true! 
    field(RW, 7, pub page_size),
    field(RW, 8, pub global),
    field(RW, 12, pub page_attribute_table),
    field(RW, 21..48, pub virt_address),
    field(RW, 59..62, pub protection_key),
    field(RW, 63, pub execute_disable)
)]
#[derive(Clone, Copy)]
pub struct PageEntry1G(u64);

impl PageEntry1G {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero().set_page_size_flag(true)
    }
}

/// A Page Directory Table Entry
///
/// # How to use?
///
/// Here we are building a `PageDirectryEntry` with the `P`, `R/W`, and `U/S` bits set. The
/// bit-field in `entry` will correspond to this change (should be compiled in).
///
/// # Safety
/// This is not 'unsafe' however, its not fully 'safe' either. When loading the page
/// tables themselves, one must understand and verify that all page tables are
/// loaded correctly. Each entry in the page table isn't unsafe by itself,
/// however, when loaded into the system it becomes unsafe.
///
/// It would be a good idea to verify that all 'bit' or options set in this entry  does exactly
/// what you intend it to do before loading it. Page tables can cause the entire system to become
/// unstable if mapped wrong -- **this is very important.**
#[make_hw( 
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_access),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    /// In this mode `page_size` needs to be set to false!
    field(RW, 7, pub page_size),
    field(RW, 12..48, pub next_entry_phy_address),
    field(RW, 63, pub execute_disable),
)]
#[derive(Clone, Copy)]
pub struct PageEntryLvl2(u64);

impl PageEntryLvl2 {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero()
    }
}

/// A Page Directory Pointer Table Entry
///
/// # How to use?
///
/// Here we are building a `PageDirectryEntry` with the `P`, `R/W`, and `U/S` bits set. The
/// bit-field in `entry` will correspond to this change (should be compiled in).
///
/// # Safety
/// This is not 'unsafe' however, its not fully 'safe' either. When loading the page
/// tables themselves, one must understand and verify that all page tables are
/// loaded correctly. Each entry in the page table isn't unsafe by itself,
/// however, when loaded into the system it becomes unsafe.
///
/// It would be a good idea to verify that all 'bit' or options set in this entry  does exactly
/// what you intend it to do before loading it. Page tables can cause the entire system to become
/// unstable if mapped wrong -- **this is very important.**
#[make_hw( 
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_access),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    /// In this mode `page_size` needs to be set to false!
    field(RW, 7, pub page_size),
    field(RW, 12..48, pub next_entry_phy_address),
    field(RW, 63, pub execute_disable),
)]
#[derive(Clone, Copy)]
pub struct PageEntryLvl3(u64);

impl PageEntryLvl3 {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero()
    }
}

/// A Page Level 4 Table Entry
///
/// # How to use?
///
/// Here we are building a `PageDirectryEntry` with the `P`, `R/W`, and `U/S` bits set. The
/// bit-field in `entry` will correspond to this change (should be compiled in).
///
/// # Safety
/// This is not 'unsafe' however, its not fully 'safe' either. When loading the page
/// tables themselves, one must understand and verify that all page tables are
/// loaded correctly. Each entry in the page table isn't unsafe by itself,
/// however, when loaded into the system it becomes unsafe.
///
/// It would be a good idea to verify that all 'bit' or options set in this entry  does exactly
/// what you intend it to do before loading it. Page tables can cause the entire system to become
/// unstable if mapped wrong -- **this is very important.**
#[make_hw( 
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_access),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    /// In this mode `page_size` needs to be set to false!
    field(RW, 7, pub page_size),
    field(RW, 12..48, pub next_entry_phy_address),
    field(RW, 63, pub execute_disable),
)]
#[derive(Clone, Copy)]
pub struct PageEntryLvl4(u64);

impl PageEntryLvl4 {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero()
    }
}

/// A Page Level 4 Table Entry
///
/// # How to use?
///
/// Here we are building a `PageDirectryEntry` with the `P`, `R/W`, and `U/S` bits set. The
/// bit-field in `entry` will correspond to this change (should be compiled in).
///
/// # Safety
/// This is not 'unsafe' however, its not fully 'safe' either. When loading the page
/// tables themselves, one must understand and verify that all page tables are
/// loaded correctly. Each entry in the page table isn't unsafe by itself,
/// however, when loaded into the system it becomes unsafe.
///
/// It would be a good idea to verify that all 'bit' or options set in this entry  does exactly
/// what you intend it to do before loading it. Page tables can cause the entire system to become
/// unstable if mapped wrong -- **this is very important.**
#[make_hw( 
    field(RW, 0, pub present),
    field(RW, 1, pub read_write),
    field(RW, 2, pub user_access),
    field(RW, 3, pub write_though),
    field(RW, 4, pub cache_disable),
    field(RW, 5, pub accessed),
    /// In this mode `page_size` needs to be set to false!
    field(RW, 7, pub page_size),
    field(RW, 12..48, pub next_entry_phy_address),
    field(RW, 63, pub execute_disable),
)]
#[derive(Clone, Copy)]
pub struct PageEntryLvl5(u64);

impl PageEntryLvl5 {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new() -> Self {
        Self::zero()
    }
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct PageMapLvl5([u64; 512]);

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct PageMapLvl4([u64; 512]);

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct PageMapLvl3([u64; 512]);

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct PageMapLvl2([u64; 512]);
