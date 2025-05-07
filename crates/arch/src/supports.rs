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

/// Cpuid feature flags
#[derive(Clone, Copy, Debug)]
pub enum CpuFeature {
    SupportsSse3,
    SupportsPclmul,
    SupportsDtes64,
    SupportsMonitor,
    SupportsDscpl,
    SupportsVmx,
    SupportsSmx,
    SupportsEst,
    SupportsTm2,
    SupportsSsse3,
    SupportsCid,
    SupportsSdbg,
    SupportsFma,
    SupportsCx16,
    SupportsXtpr,
    SupportsPdcm,
    SupportsPcid,
    SupportsDca,
    SupportsSse4_1,
    SupportsSse4_2,
    SupportsX2apic,
    SupportsMovbe,
    SupportsPopcnt,
    SupportsTsc0,
    SupportsAes,
    SupportsXsave,
    SupportsOsxsave,
    SupportsAvx,
    SupportsF16c,
    SupportsRdrand,
    SupportsHypervisor,
    SupportsFpu,
    SupportsVme,
    SupportsDe,
    SupportsPse,
    SupportsTsc1,
    SupportsMsr,
    SupportsPae,
    SupportsMce,
    SupportsCx8,
    SupportsApic,
    SupportsSep,
    SupportsMtrr,
    SupportsPge,
    SupportsMca,
    SupportsCmov,
    SupportsPat,
    SupportsPse36,
    SupportsPsn,
    SupportsClflush,
    SupportsDs,
    SupportsAcpi,
    SupportsMmx,
    SupportsFxsr,
    SupportsSse,
    SupportsSse2,
    SupportsSs,
    SupportsHtt,
    SupportsTm,
    SupportsIa64,
    SupportsPbe,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum CpuidRequest {
    VenderString,
    AddressSize,
    Feature,
    None,
}

#[derive(Clone, Copy, Debug)]
pub enum CpuVender {
    Amd,
    OldAmd,
    Intel,
    Qemu,
    Kvm,
    VMware,
    VirtualBox,
    Xen,
    HyperV,
    Parallels,
    Bhyve,
    Qnx,
    UnknownVender([u8; 12]),
}

impl CpuVender {
    fn from_vender_string(ebx: u32, ecx: u32, edx: u32) -> CpuVender {
        let vender_string_u32 = [ebx, edx, ecx];
        let vender_string = unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(
                vender_string_u32.as_ptr().cast(),
                12,
            ))
            .expect("CPUID should always return valid UTF8 chars!")
        };

        match vender_string {
            "AuthenticAMD" => CpuVender::Amd,
            "AMDisbetter!" => CpuVender::OldAmd,
            "GenuineIntel" => CpuVender::Intel,
            "TCGTCGTCGTCG" => CpuVender::Qemu,
            " KVMKVMKVM  " => CpuVender::Kvm,
            "VMwareVMware" => CpuVender::VMware,
            "VBoxVBoxVBox" => CpuVender::VirtualBox,
            "XenVMMXenVMM" => CpuVender::Xen,
            "Microsoft Hv" => CpuVender::HyperV,
            " prl hyperv " | " lrpepyh vr " => CpuVender::Parallels,
            "bhyve bhyve " => CpuVender::Bhyve,
            " QNXQVMBSQG " => CpuVender::Qnx,
            _ => CpuVender::UnknownVender(unsafe { core::mem::transmute(vender_string_u32) }),
        }
    }
}

impl CpuidRequest {
    /// Convert this request into registers that cpuid uses.
    #[inline]
    pub const fn into_registers(self) -> (u32, u32, u32, u32) {
        match self {
            Self::VenderString => (0, 0, 0, 0),
            Self::Feature => (1, 0, 0, 0),
            Self::AddressSize => (0x80000008, 0, 0, 0),
            _ => panic!("todo"),
        }
    }
}

/// The raw cpuid command rapper.
#[inline]
pub fn cpuid(request: CpuidRequest) -> (u32, u32, u32, u32) {
    let (mut eax, mut ebx, mut ecx, mut edx) = request.into_registers();

    #[cfg(target_pointer_width = "32")]
    unsafe {
        // This is dumb, but LLVM won't let me use 'ebx' so I need to use a sub for it
        core::arch::asm!("
            push ebx
            mov ebx, {ebx_sub:e}
            cpuid
            mov {ebx_sub:e}, ebx
            pop ebx",
            ebx_sub = inout(reg) ebx,
            inout("eax") eax,
            inout("ecx") ecx,
            inout("edx") edx
        );
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        // This is dumb, but LLVM won't let me use 'ebx' so I need to use a sub for it
        core::arch::asm!("
            push rbx
            mov ebx, {ebx_sub:e}
            cpuid
            mov {ebx_sub:e}, ebx
            pop rbx",
            ebx_sub = inout(reg) ebx,
            inout("eax") eax,
            inout("ecx") ecx,
            inout("edx") edx
        );
    }

    (eax, ebx, ecx, edx)
}

/// Gets the cpu vender info
#[inline]
pub fn cpu_vender() -> CpuVender {
    let (_, ebx, ecx, edx) = cpuid(CpuidRequest::VenderString);
    CpuVender::from_vender_string(ebx, ecx, edx)
}

/// Using `cpuid` check if this cpu supports `feature`.
#[inline]
pub fn does_cpu_support(feature: CpuFeature) -> bool {
    let (_, _, ecx, edx) = cpuid(CpuidRequest::Feature);
    match feature {
        CpuFeature::SupportsSse3 => ecx & (1 << 0) != 0,
        CpuFeature::SupportsPclmul => ecx & (1 << 1) != 0,
        CpuFeature::SupportsDtes64 => ecx & (1 << 2) != 0,
        CpuFeature::SupportsMonitor => ecx & (1 << 3) != 0,
        CpuFeature::SupportsDscpl => ecx & (1 << 4) != 0,
        CpuFeature::SupportsVmx => ecx & (1 << 5) != 0,
        CpuFeature::SupportsSmx => ecx & (1 << 6) != 0,
        CpuFeature::SupportsEst => ecx & (1 << 7) != 0,
        CpuFeature::SupportsTm2 => ecx & (1 << 8) != 0,
        CpuFeature::SupportsSsse3 => ecx & (1 << 9) != 0,
        CpuFeature::SupportsCid => ecx & (1 << 10) != 0,
        CpuFeature::SupportsSdbg => ecx & (1 << 11) != 0,
        CpuFeature::SupportsFma => ecx & (1 << 12) != 0,
        CpuFeature::SupportsCx16 => ecx & (1 << 13) != 0,
        CpuFeature::SupportsXtpr => ecx & (1 << 14) != 0,
        CpuFeature::SupportsPdcm => ecx & (1 << 15) != 0,
        CpuFeature::SupportsPcid => ecx & (1 << 17) != 0,
        CpuFeature::SupportsDca => ecx & (1 << 18) != 0,
        CpuFeature::SupportsSse4_1 => ecx & (1 << 19) != 0,
        CpuFeature::SupportsSse4_2 => ecx & (1 << 20) != 0,
        CpuFeature::SupportsX2apic => ecx & (1 << 21) != 0,
        CpuFeature::SupportsMovbe => ecx & (1 << 22) != 0,
        CpuFeature::SupportsPopcnt => ecx & (1 << 23) != 0,
        CpuFeature::SupportsTsc0 => ecx & (1 << 24) != 0,
        CpuFeature::SupportsAes => ecx & (1 << 25) != 0,
        CpuFeature::SupportsXsave => ecx & (1 << 26) != 0,
        CpuFeature::SupportsOsxsave => ecx & (1 << 27) != 0,
        CpuFeature::SupportsAvx => ecx & (1 << 28) != 0,
        CpuFeature::SupportsF16c => ecx & (1 << 29) != 0,
        CpuFeature::SupportsRdrand => ecx & (1 << 30) != 0,
        CpuFeature::SupportsHypervisor => ecx & (1 << 31) != 0,

        CpuFeature::SupportsFpu => edx & (1 << 1) != 0,
        CpuFeature::SupportsVme => edx & (1 << 2) != 0,
        CpuFeature::SupportsDe => edx & (1 << 3) != 0,
        CpuFeature::SupportsPse => edx & (1 << 4) != 0,
        CpuFeature::SupportsTsc1 => edx & (1 << 5) != 0,
        CpuFeature::SupportsMsr => edx & (1 << 6) != 0,
        CpuFeature::SupportsPae => edx & (1 << 7) != 0,
        CpuFeature::SupportsMce => edx & (1 << 8) != 0,
        CpuFeature::SupportsCx8 => edx & (1 << 9) != 0,
        CpuFeature::SupportsApic => edx & (1 << 11) != 0,
        CpuFeature::SupportsSep => edx & (1 << 12) != 0,
        CpuFeature::SupportsMtrr => edx & (1 << 13) != 0,
        CpuFeature::SupportsPge => edx & (1 << 14) != 0,
        CpuFeature::SupportsMca => edx & (1 << 15) != 0,
        CpuFeature::SupportsCmov => edx & (1 << 16) != 0,
        CpuFeature::SupportsPat => edx & (1 << 17) != 0,
        CpuFeature::SupportsPse36 => edx & (1 << 18) != 0,
        CpuFeature::SupportsPsn => edx & (1 << 19) != 0,
        CpuFeature::SupportsClflush => edx & (1 << 20) != 0,
        CpuFeature::SupportsDs => edx & (1 << 21) != 0,
        CpuFeature::SupportsAcpi => edx & (1 << 22) != 0,
        CpuFeature::SupportsMmx => edx & (1 << 23) != 0,
        CpuFeature::SupportsFxsr => edx & (1 << 24) != 0,
        CpuFeature::SupportsSse => edx & (1 << 25) != 0,
        CpuFeature::SupportsSse2 => edx & (1 << 26) != 0,
        CpuFeature::SupportsSs => edx & (1 << 27) != 0,
        CpuFeature::SupportsHtt => edx & (1 << 28) != 0,
        CpuFeature::SupportsTm => edx & (1 << 29) != 0,
        CpuFeature::SupportsIa64 => edx & (1 << 30) != 0,
        CpuFeature::SupportsPbe => edx & (1 << 31) != 0,
    }
}

/// Get the number of bits for this processors physical address size
#[inline]
pub fn physical_address_size_bits() -> usize {
    let (eax, ..) = cpuid(CpuidRequest::AddressSize);

    (eax as usize) & 0xFF
}

/// Get the number of bits for this processors virtual address size
#[inline]
pub fn virtual_address_size_bits() -> usize {
    let (eax, ..) = cpuid(CpuidRequest::AddressSize);

    (eax as usize >> 8) & 0xFF
}

/// Get the number of bits for this processors guest address size
#[inline]
pub fn guest_address_size_bits() -> usize {
    let (eax, ..) = cpuid(CpuidRequest::AddressSize);

    (eax as usize >> 16) & 0xFF
}

/// A macro that ensures support for some cpu feature
#[macro_export]
macro_rules! ensure_support_for {
    ($id:expr) => {{
        assert!(
            ::arch::supports::does_cpu_support($id),
            "CPU feature '{:?}' is required by QuantumOS! However, this CPU does not support it.",
            $id
        );
    }};
}
