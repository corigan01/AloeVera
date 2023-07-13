/*
  ____                 __               __ __                 __
 / __ \__ _____ ____  / /___ ____ _    / //_/__ _______  ___ / /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / ,< / -_) __/ _ \/ -_) /
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /_/|_|\__/_/ /_//_/\__/_/
  Part of the Quantum OS Kernel

Copyright 2022 Gavin Kellam

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

use quantum_lib::x86_64::io::port::IOPort;
use quantum_utils::bitset::BitSet;

const PRIMARY_BUS_IO_BASE: usize = 0x1F0;
const SECONDARY_BUS_IO_BASE: usize = 0x170;

const PRIMARY_BUS_CONTROL_BASE: usize = 0x3F6;
const SECONDARY_BUS_CONTROL_BASE: usize = 0x376;

/// # R/W: Data Register (16-bit / 16-bit)
/// Read/Write PIO data bytes
const DATA_REGISTER_OFFSET_FROM_IO_BASE: usize = 0;

/// # R: Error Register (8-bit / 16-bit)
/// Used to retrieve any error generated by the last ATA command executed.
const ERROR_REGISTER_OFFSET_FROM_IO_BASE: usize = 1;

/// # W: Features Register (8-bit / 16-bit)
/// Used to control command specific interface features.
const FEATURES_REGISTER_OFFSET_FROM_IO_BASE: usize = 1;

/// # R/W: Sector Count Register (8-bit / 16-bit)
/// Number of sectors to read/write (0 is a special value).
const SECTOR_COUNT_OFFSET_FROM_IO_BASE: usize = 2;

/// # R/W: Sector Number Register (LBAlo) (8-bit / 16-bit)
/// This is CHS / LBA28 / LBA48 specific.
const SECTOR_NUM_LOW_OFFSET_FROM_IO_BASE: usize = 3;

/// # R/W: Cylinder Low Register / (LBAmid) (8-bit / 16-bit)
/// Partial Disk Sector address.
const SECTOR_NUM_MID_OFFSET_FROM_IO_BASE: usize = 4;

/// # R/W: Cylinder High Register / (LBAhi)	(8-bit / 16-bit)
/// Partial Disk Sector address.
const SECTOR_NUM_HIGH_OFFSET_FROM_IO_BASE: usize = 5;

/// # R/W: Drive / Head Register (8-bit / 8-bit)
/// Used to select a drive and/or head. Supports extra address/flag bits.
const DRIVE_HEAD_OFFSET_FROM_IO_BASE: usize = 6;

/// # R: Status Register (8-bit / 8-bit)
/// Used to read the current status.
const STATUS_REGISTER_OFFSET_FROM_IO_BASE: usize = 7;

/// # W: Command Register (8-bit / 8-bit)
/// Used to send ATA commands to the device.
const COMMAND_OFFSET_FROM_IO_BASE: usize = 7;

/// # R: Alternate Status Register (8-bit / 8-bit)
/// A duplicate of the Status Register which does not affect interrupts.
const ALTERNATE_STATUS_REGISTER_OFFSET_FROM_CONTROL_BASE: usize = 0;

/// # W: Device Control Register (8-bit / 8-bit)
/// Used to reset the bus or enable/disable interrupts.
const DEVICE_CONTROL_REGISTER_OFFSET_FROM_CONTROL_BASE: usize = 0;

/// # R: Drive Address Register (8-bit / 8-bit)
/// Provides drive select and head select information.
const DRIVE_ADDRESS_REGISTER_OFFSET_FROM_CONTROL_BASE: usize = 1;

#[derive(Clone, Copy)]
pub enum Device {
    PrimaryFirst,
    PrimarySecond,
    SecondaryFirst,
    SecondarySecond
}

impl Device {
    pub const fn bus_base(&self) -> usize {
        match self {
            Device::PrimaryFirst => PRIMARY_BUS_IO_BASE,
            Device::PrimarySecond => PRIMARY_BUS_IO_BASE,
            Device::SecondaryFirst => SECONDARY_BUS_IO_BASE,
            Device::SecondarySecond => SECONDARY_BUS_IO_BASE,
        }
    }

    pub const fn control_base(&self) -> usize {
        match self {
            Device::PrimaryFirst => PRIMARY_BUS_CONTROL_BASE,
            Device::PrimarySecond => PRIMARY_BUS_CONTROL_BASE,
            Device::SecondaryFirst => SECONDARY_BUS_CONTROL_BASE,
            Device::SecondarySecond => SECONDARY_BUS_CONTROL_BASE,
        }
    }

    pub fn is_first(&self) -> bool {
        match self {
            Device::PrimaryFirst => true,
            Device::PrimarySecond => false,
            Device::SecondaryFirst => true,
            Device::SecondarySecond => false
        }
    }

    pub fn is_second(&self) -> bool {
        !self.is_first()
    }
}

pub enum StatusFlags {
    /// Indicates an error occurred. Send a new command to clear it (or nuke it with a Software Reset).
    Err,
    /// Index. Always set to zero.
    Idx,
    /// Corrected data. Always set to zero.
    CorrectedData,
    /// Set when the drive has PIO data to transfer, or is ready to accept PIO data.
    DRQ,
    /// Overlapped Mode Service Request.
    SRV,
    /// Drive Fault Error (**does not set ERR**).
    DriveFault,
    /// Bit is clear when drive is spun down, or after an error. Set otherwise.
    SpinDown,
    /// Indicates the drive is preparing to send/receive data (wait for it to clear).
    /// In case of 'hang' (it never clears), do a software reset.
    Busy
}

pub struct StatusRegister {}
impl StatusRegister {
    const ATA_SR_BSY_BIT: u8 = 7;
    const ATA_SR_DRDY_BIT: u8 = 6;
    const ATA_SR_DF_BIT: u8 = 5;
    const ATA_SR_DSC_BIT: u8 = 4;
    const ATA_SR_DRQ_BIT: u8 = 3;
    const ATA_SR_CORR_BIT: u8 = 2;
    const ATA_SR_IDX_BIT: u8 = 1;
    const ATA_SR_ERR_BIT: u8 = 0;

    fn my_port(device: Device) -> IOPort {
        let device_io = device.bus_base() + STATUS_REGISTER_OFFSET_FROM_IO_BASE;

        IOPort::new(device_io as u16)
    }

    pub fn read(device: Device) -> u8 {
        let my_port = Self::my_port(device);

        unsafe { my_port.read_u8() }
    }

    pub fn perform_400ns_delay(device: Device) {
        let my_port = Self::my_port(device);

        for _ in 0..15 {
            unsafe { my_port.read_u8() };
        }
    }

    pub fn is_status(device: Device, status: StatusFlags) -> bool {
        let read_value = Self::read(device);

        let bit = match status {
            StatusFlags::Err => Self::ATA_SR_ERR_BIT,
            StatusFlags::Idx => Self::ATA_SR_IDX_BIT,
            StatusFlags::CorrectedData => Self::ATA_SR_CORR_BIT,
            StatusFlags::DRQ => Self::ATA_SR_DRQ_BIT,
            StatusFlags::SRV => Self::ATA_SR_DRDY_BIT,
            StatusFlags::DriveFault => Self::ATA_SR_DF_BIT,
            StatusFlags::SpinDown => Self::ATA_SR_DSC_BIT,
            StatusFlags::Busy => Self::ATA_SR_BSY_BIT
        };

        read_value.get_bit(bit)
    }

}

pub enum ErrorFlags {
    /// 0:	(AMNF)    Address mark not found.
    AddressMarkNotFound,
    /// 1:	(TKZNF)   Track zero not found.
    TrackZeroNotFound,
    /// 2:	(ABRT)    Aborted command.
    AbortedCommand,
    /// 3:	(MCR)	  Media change request.
    MediaChangeRequest,
    /// 4:	(IDNF)    ID not found.
    IDNotFound,
    /// 5:	(MC)      Media changed.
    MediaChanged,
    /// 6:	(UNC)     Uncorrectable data error
    UncorrectableDataError,
    /// 7:	(BBK)     Bad Block detected.
    BadBlockDetected,
}

pub struct ErrorRegister {}
impl ErrorRegister {
    const ATA_ER_BBK_BIT: u8 = 7;
    const ATA_ER_UNC_BIT: u8 = 6;
    const ATA_ER_MC_BIT: u8 = 5;
    const ATA_ER_IDNF_BIT: u8 = 4;
    const ATA_ER_MCR_BIT: u8 = 3;
    const ATA_ER_ABRT_BIT: u8 = 2;
    const ATA_ER_TKONF_BIT: u8 = 1;
    const ATA_ER_AMNF_BIT: u8 = 0;

    fn my_port(device: Device) -> IOPort {
        let io_port = device.bus_base() + ERROR_REGISTER_OFFSET_FROM_IO_BASE;

        IOPort::new(io_port as u16)
    }

    pub fn read(device: Device) -> u8 {
        let port = Self::my_port(device);

        unsafe { port.read_u8() }
    }

    pub fn any_error(device: Device) -> bool {
        Self::read(device) > 0
    }

    pub fn is_error(device: Device, error: ErrorFlags) -> bool {
        let value = Self::read(device);

        if value == 0 {
            return false;
        }

        match error {
            ErrorFlags::AddressMarkNotFound => {
                value & (1 << Self::ATA_ER_AMNF_BIT) != 0
            }
            ErrorFlags::TrackZeroNotFound => {
                value & (1 << Self::ATA_ER_TKONF_BIT) != 0
            }
            ErrorFlags::AbortedCommand => {
                value & (1 << Self::ATA_ER_ABRT_BIT) != 0
            }
            ErrorFlags::MediaChangeRequest => {
                value & (1 << Self::ATA_ER_MCR_BIT) != 0
            }
            ErrorFlags::IDNotFound => {
                value & (1 << Self::ATA_ER_IDNF_BIT) != 0
            }
            ErrorFlags::MediaChanged => {
                value & (1 << Self::ATA_ER_MC_BIT) != 0
            }
            ErrorFlags::UncorrectableDataError => {
                value & (1 << Self::ATA_ER_UNC_BIT) != 0
            }
            ErrorFlags::BadBlockDetected => {
                value & (1 << Self::ATA_ER_BBK_BIT) != 0
            }
        }
    }
}

pub struct DriveHeadRegister {}
impl DriveHeadRegister {
    const ATA_DH_DRV: u8 = 4;
    const ATA_DH_RESERVED_1: u8 = 5;
    const ATA_DH_LBA: u8 = 6;
    const ATA_DH_RESERVED_2: u8 = 7;

    fn my_port(device: Device) -> IOPort {
        let io_port = device.bus_base() + DRIVE_HEAD_OFFSET_FROM_IO_BASE;

        IOPort::new(io_port as u16)
    }

    pub fn read(device: Device) -> u8 {
        let port = Self::my_port(device);

        unsafe { port.read_u8() }
    }

    pub fn write(device: Device, value: u8) {
        let port = Self::my_port(device);

        unsafe { port.write_u8(value) };
    }

    pub fn is_using_chs(device: Device) -> bool {
        let value = Self::read(device);

        value & (1 << Self::ATA_DH_LBA) == 0
    }

    pub fn is_using_lba(device: Device) -> bool {
        !Self::is_using_chs(device)
    }

    pub fn select_drive(device: Device) {
        let mut read = Self::read(device);
        let write_value = read.set_bit(Self::ATA_DH_DRV, device.is_second());
        Self::write(device, write_value);
    }


}

pub struct CommandRegister {}
impl CommandRegister {
    const ATA_CMD_READ_PIO: u8 = 0x20;
    const ATA_CMD_READ_PIO_EXT: u8 = 0x24;
    const ATA_CMD_READ_DMA: u8 = 0xC8;
    const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
    const ATA_CMD_WRITE_PIO: u8 = 0x30;
    const ATA_CMD_WRITE_PIO_EXT: u8 = 0x34;
    const ATA_CMD_WRITE_DMA: u8 = 0xCA;
    const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
    const ATA_CMD_CACHE_FLUSH: u8 = 0xE7;
    const ATA_CMD_CACHE_FLUSH_EXT: u8 = 0xEA;
    const ATA_CMD_PACKET: u8 = 0xA0;
    const ATA_IDENTIFY_PACKET: u8 = 0xA1;
    const ATA_IDENTIFY: u8 = 0xEC;

    pub fn preform_identify(device: Device) {

    }
}