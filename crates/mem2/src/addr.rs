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

// This is to make sure no one else can impl the `AddrAlign` and `AddrKind` traits!
pub(crate) mod _priv {
    pub trait Sealed {}
}

/// Generic Alignment Specifier
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

pub trait AddrPage: AddrAlign {}

#[derive(Debug, Clone, Copy)]
pub struct Unaligned;
#[derive(Debug, Clone, Copy)]
pub struct Aligned<const ALIGN: usize>;
#[derive(Debug, Clone, Copy)]
pub struct PhysicalAddr;
#[derive(Debug, Clone, Copy)]
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

impl AddrPage for Aligned<PAGE_4K> {}
impl AddrPage for Aligned<PAGE_2M> {}
impl AddrPage for Aligned<PAGE_1G> {}

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
#[derive(Debug, Clone, Copy)]
pub struct Addr<Kind: AddrKind, Align: AddrAlign = Unaligned> {
    addr: usize,
    _ph: PhantomData<(Kind, Align)>,
}

pub type PhysAddr<Align = Unaligned> = Addr<PhysicalAddr, Align>;
pub type VirtAddr<Align = Unaligned> = Addr<LogicalAddr, Align>;

pub const PAGE_4K: usize = 4096;
pub const PAGE_2M: usize = PAGE_4K * 512;
pub const PAGE_1G: usize = PAGE_2M * 512;

pub type PhysPage<const ALIGN: usize> = Addr<PhysicalAddr, Aligned<ALIGN>>;
pub type VirtPage<const ALIGN: usize> = Addr<LogicalAddr, Aligned<ALIGN>>;

pub type PhysPage4K = Addr<PhysAddr, Aligned<PAGE_4K>>;
pub type PhysPage2M = Addr<PhysAddr, Aligned<PAGE_2M>>;
pub type PhysPage1G = Addr<PhysAddr, Aligned<PAGE_1G>>;

pub type VirtPage4K = Addr<LogicalAddr, Aligned<PAGE_4K>>;
pub type VirtPage2M = Addr<LogicalAddr, Aligned<PAGE_2M>>;
pub type VirtPage1G = Addr<LogicalAddr, Aligned<PAGE_1G>>;

impl<Kind: AddrKind> Addr<Kind, Unaligned> {
    #[inline]
    pub const unsafe fn unaligned_unchecked(addr: usize) -> Self {
        Self {
            addr,
            _ph: PhantomData,
        }
    }

    #[inline]
    pub const fn null_unaligned() -> Self {
        Self {
            addr: 0,
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

    #[inline]
    pub const fn null_aligned() -> Self {
        Self {
            addr: 0,
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
        assert!(
            Self::ADDR_ALIGNMENT.is_power_of_two(),
            "Addr Alignment must be a power of 2!"
        );

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

    #[inline]
    pub const fn is_aligned_to(&self, alignment: usize) -> bool {
        debug_assert!(
            Self::ADDR_ALIGNMENT.is_power_of_two(),
            "Addr Alignment must be a power of 2!"
        );
        debug_assert!(
            alignment.is_power_of_two(),
            "Addr Alignment must be a power of 2!"
        );

        Self::ADDR_ALIGNMENT >= alignment
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        assert!(self.is_aligned_to(align_of::<T>()));
        self.get() as *const T
    }

    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        assert!(self.is_aligned_to(align_of::<T>()));
        self.get() as *mut T
    }

    pub fn addr_distance(&self, rhs: Self) -> usize {
        let usized_lhs = self.get();
        let usized_rhs = rhs.get();

        usized_rhs.abs_diff(usized_lhs)
    }
}
impl<Kind: AddrKind> Addr<Kind, Aligned<PAGE_4K>> {
    pub const unsafe fn page_4k_unchecked(page_id: usize) -> Self {
        unsafe { Self::aligned_unchecked(page_id * Self::ADDR_ALIGNMENT) }
    }

    pub fn page_4k(page_id: usize) -> Self {
        Self::aligned(page_id * Self::ADDR_ALIGNMENT)
    }
}

impl<Kind: AddrKind> Addr<Kind, Aligned<PAGE_2M>> {
    pub const unsafe fn page_2m_unchecked(page_id: usize) -> Self {
        unsafe { Self::aligned_unchecked(page_id * Self::ADDR_ALIGNMENT) }
    }

    pub fn page_2m(page_id: usize) -> Self {
        Self::aligned(page_id * Self::ADDR_ALIGNMENT)
    }
}

impl<Kind: AddrKind> Addr<Kind, Aligned<PAGE_1G>> {
    pub const unsafe fn page_1g_unchecked(page_id: usize) -> Self {
        unsafe { Self::aligned_unchecked(page_id * Self::ADDR_ALIGNMENT) }
    }

    pub fn page_1g(page_id: usize) -> Self {
        Self::aligned(page_id * Self::ADDR_ALIGNMENT)
    }
}

impl<Kind: AddrKind, Page> Addr<Kind, Page>
where
    Page: AddrPage + AddrAlign,
{
    pub const fn page_id(&self) -> usize {
        self.addr / Self::ADDR_ALIGNMENT
    }
}

impl<KindO: AddrKind, AlignO: AddrAlign, KindI: AddrKind, AlignI: AddrAlign>
    PartialEq<Addr<KindI, AlignI>> for Addr<KindO, AlignO>
{
    fn eq(&self, other: &Addr<KindI, AlignI>) -> bool {
        self.addr == other.addr
    }
}

impl<KindO: AddrKind, AlignO: AddrAlign> Eq for Addr<KindO, AlignO> {}

impl<KindO: AddrKind, AlignO: AddrAlign, KindI: AddrKind, AlignI: AddrAlign>
    PartialOrd<Addr<KindI, AlignI>> for Addr<KindO, AlignO>
{
    fn partial_cmp(&self, other: &Addr<KindI, AlignI>) -> Option<core::cmp::Ordering> {
        self.addr.partial_cmp(&other.addr)
    }
}

impl<KindO: AddrKind, AlignO: AddrAlign> Ord for Addr<KindO, AlignO> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.addr.cmp(&other.addr)
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

        _ = VirtAddr::unaligned(100);
        _ = VirtAddr::<Aligned<8>>::try_aligned(100).expect_err("Expected this to fail!");
        _ = VirtAddr::<Aligned<8>>::aligned(16);
    }

    #[test]
    #[should_panic]
    #[cfg(target_pointer_width = "64")]
    fn should_fail_addr_length() {
        PHYSICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);
        LOGICAL_ADDR_WIDTH_BITS.store(48, Ordering::SeqCst);

        _ = PhysAddr::unaligned(usize::MAX ^ (1 << 63));
    }

    #[test]
    fn test_page_sameness() {
        let addr = VirtAddr::<Aligned<4096>>::aligned(40960 /* The 10th page  */);
        let page = VirtPage::page_4k(10 /* Also, the 10th page */);

        assert_eq!(addr, page);

        let addr_non_aligned =
            VirtAddr::unaligned(40960 /* Can even compare with non-aligned types! */);
        assert_eq!(addr_non_aligned, page);
    }

    #[test]
    fn check_page_sized_items() {
        for i in 0..1_000 {
            let page = VirtPage::page_4k(i);
            assert_eq!(page.page_id(), i);

            let page = VirtPage::page_2m(i);
            assert_eq!(page.page_id(), i);

            let page = VirtPage::page_1g(i);
            assert_eq!(page.page_id(), i);
        }
    }
}
