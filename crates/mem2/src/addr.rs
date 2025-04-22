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

use core::{
    fmt::Debug,
    marker::PhantomData,
    sync::atomic::{AtomicU8, Ordering},
};

pub(crate) mod _priv {
    pub trait Sealed {}
}

pub trait AddrAlign: _priv::Sealed {
    const ALIGN: usize;
}

pub trait AddrKind: _priv::Sealed + Debug {
    const ADDR_NAME_HUMAN: &str;
    const ADDR_NAME_MACHINE: &str;

    fn addr_width_bits() -> u8;

    #[inline]
    fn check_address(addr: usize) -> bool {
        let shift = Self::addr_width_bits() - 1;
        (addr >> shift) == 0 || (addr >> shift) == (usize::MAX >> shift)
    }
}

#[derive(Debug)]
pub struct Unaligned;
#[derive(Debug)]
pub struct Aligned<const ALIGN: usize>;
#[derive(Debug)]
pub struct PhysicalAddr;
#[derive(Debug)]
pub struct LogicalAddr;

impl _priv::Sealed for PhysicalAddr {}
impl _priv::Sealed for LogicalAddr {}
impl _priv::Sealed for Unaligned {}
impl<const ALIGN: usize> _priv::Sealed for Aligned<ALIGN> {}

impl AddrAlign for Unaligned {
    const ALIGN: usize = 1;
}

impl<const TYPE_ALIGN: usize> AddrAlign for Aligned<TYPE_ALIGN> {
    const ALIGN: usize = TYPE_ALIGN;
}

impl AddrKind for PhysicalAddr {
    const ADDR_NAME_HUMAN: &str = "Physical Address";
    const ADDR_NAME_MACHINE: &str = stringify!(PhysicalAddr);

    fn addr_width_bits() -> u8 {
        physical_addr_width_bits()
    }
}

impl AddrKind for LogicalAddr {
    const ADDR_NAME_HUMAN: &str = "Logical/Virtual Address";
    const ADDR_NAME_MACHINE: &str = stringify!(LogicalAddr);

    fn addr_width_bits() -> u8 {
        logical_addr_width_bits()
    }
}

pub type PhysAddrWidthBits = u8;
pub type LogicalAddrWidthBits = u8;

pub(crate) static PHYSICAL_ADDR_WIDTH_BITS: AtomicU8 = AtomicU8::new(0);
pub(crate) static LOGICAL_ADDR_WIDTH_BITS: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn populate_address_width() {
    // If its already populated, return!
    if PHYSICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed) != 0
        && LOGICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed) != 0
    {
        return;
    }

    // read the cpu's addr width values
    let cpuid = unsafe { core::arch::x86_64::__cpuid(0x80000008) };

    let physical_bits = (cpuid.eax & 0xFF) as u8;
    let logical_bits = ((cpuid.eax >> 8) & 0xFF) as u8;

    if PHYSICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed) == 0 {
        PHYSICAL_ADDR_WIDTH_BITS.store(physical_bits, Ordering::SeqCst);
    }

    if LOGICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed) == 0 {
        LOGICAL_ADDR_WIDTH_BITS.store(logical_bits, Ordering::SeqCst);
    }
}

#[inline]
pub fn logical_addr_width_bits() -> LogicalAddrWidthBits {
    populate_address_width();
    LOGICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed)
}

#[inline]
pub fn physical_addr_width_bits() -> PhysAddrWidthBits {
    populate_address_width();
    PHYSICAL_ADDR_WIDTH_BITS.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddrError<Kind: AddrKind> {
    InvalidAlignment {
        expected_alignmnet: usize,
        given: usize,
    },
    InvalidAddr {
        given: usize,
        _ph: PhantomData<Kind>,
    },
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Addr<Kind: AddrKind, Align: AddrAlign = Unaligned> {
    addr: usize,
    _ph: PhantomData<(Kind, Align)>,
}

pub type PhysAddr<Align = Unaligned> = Addr<PhysicalAddr, Align>;
pub type VirtAddr<Align = Unaligned> = Addr<LogicalAddr, Align>;

impl<Kind: AddrKind> Addr<Kind, Unaligned> {
    #[inline]
    pub const unsafe fn unaligned_unchecked(addr: usize) -> Self {
        Self {
            addr,
            _ph: PhantomData,
        }
    }

    #[inline]
    pub fn unaligned(addr: usize) -> Self {
        Self::try_unaligned(addr).expect("Unable to verify provided address")
    }

    #[inline]
    pub fn try_unaligned(addr: usize) -> Result<Self, AddrError<Kind>> {
        Self::verify_addr(addr)?;
        Ok(unsafe { Self::unaligned_unchecked(addr) })
    }
}

impl<Kind: AddrKind, const ALIGN: usize> Addr<Kind, Aligned<ALIGN>> {
    #[inline]
    pub const unsafe fn aligned_unchecked(addr: usize) -> Self {
        Self {
            addr,
            _ph: PhantomData,
        }
    }

    pub fn aligned(addr: usize) -> Self {
        Self::try_aligned(addr).expect("Unable to verify provided address")
    }

    pub fn try_aligned(addr: usize) -> Result<Self, AddrError<Kind>> {
        Self::verify_addr(addr)?;
        Ok(unsafe { Self::aligned_unchecked(addr) })
    }
}

impl<Kind: AddrKind, Align: AddrAlign> Addr<Kind, Align> {
    const ADDR_ALIGNMENT: usize = Align::ALIGN;

    fn verify_addr(addr: usize) -> Result<(), AddrError<Kind>> {
        // Check that the address is aligned
        if addr & (Self::ADDR_ALIGNMENT - 1) != 0 {
            return Err(AddrError::InvalidAlignment {
                expected_alignmnet: Self::ADDR_ALIGNMENT,
                given: addr,
            });
        }

        // Check that the address is actually addressable on this machine
        if !Kind::check_address(addr) {
            return Err(AddrError::InvalidAddr {
                given: addr,
                _ph: PhantomData,
            });
        }

        Ok(())
    }

    pub const fn get(&self) -> usize {
        self.addr
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_addr_verify() {
        #[derive(Debug)]
        struct FakeAddr;

        impl _priv::Sealed for FakeAddr {}
        impl AddrKind for FakeAddr {
            const ADDR_NAME_HUMAN: &str = "";
            const ADDR_NAME_MACHINE: &str = "";

            fn addr_width_bits() -> u8 {
                8
            }
        }

        assert!(FakeAddr::check_address(127));
        assert!(!FakeAddr::check_address(128));
        assert!(!FakeAddr::check_address(0b1_0000_0000));
        assert!(FakeAddr::check_address(usize::MAX ^ 0x7A));
        assert!(!FakeAddr::check_address(usize::MAX ^ 0x8A));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn machine_check_addr_length() {
        PHYSICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);
        LOGICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);

        _ = PhysAddr::unaligned(100);
        _ = PhysAddr::<Aligned<8>>::try_aligned(100).expect_err("Expected this to fail!");
        _ = PhysAddr::<Aligned<8>>::aligned(16);
    }

    #[test]
    #[should_panic]
    #[cfg(target_pointer_width = "64")]
    fn should_fail_addr_length() {
        PHYSICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);
        LOGICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);

        _ = PhysAddr::unaligned(usize::MAX ^ (1 << 63));
    }
}
