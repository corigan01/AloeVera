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

use qk_alloc::vec::Vec;
use quantum_lib::{debug_print, debug_println};
use crate::ata::registers::{CommandRegister, Commands, DataRegister, DiskID, DriveHeadRegister, ErrorRegister, SectorRegisters, StatusFlags, StatusRegister};
use owo_colors::OwoColorize;

mod registers;

pub struct ATADisk {
    device: DiskID,
    identify: Vec<u16>
}

impl ATADisk {
    fn new(device: DiskID, identify: Vec<u16>) -> Self {
        Self {
            device,
            identify
        }
    }


}

pub fn scan_for_disks() -> Vec<ATADisk> {
    let mut disks = Vec::new();

    for disk in DiskID::iter() {
        let disk = *disk;
        debug_print!("Scanning disk '{:?}' \t... ", disk);

        // Select the drive that we are using. Since we are preforming a disk change,
        // we must wait >400ns for the controller to push its status on the IO lines.
        DriveHeadRegister::select_drive(disk);
        StatusRegister::perform_400ns_delay(disk);

        // Spec suggests we need to zero all the sector registers before sending the identify command
        SectorRegisters::zero_registers(disk);

        CommandRegister::send_command(disk, Commands::Identify);
        StatusRegister::perform_400ns_delay(disk);

        // If the bus is floating, we know we don't have a disk
        if StatusRegister::is_floating(disk) {
            debug_println!("{}", "N/A".yellow());
            continue;
        }

        // If some bit got set for the sector registers, its not a ATA device.
        // Some ATAPI drives to not follow spec! At this point we *must* stop pulling.
        if !SectorRegisters::are_all_zero(disk) {
            debug_println!("{}", "Skip".yellow());
            continue;
        }

        // Loop while busy
        while StatusRegister::is_busy(disk) &&
            !StatusRegister::is_err_or_fault(disk) &&
            !StatusRegister::is_status(disk, StatusFlags::DRQ) {}

        if StatusRegister::is_err_or_fault(disk) {
            let errors = ErrorRegister::all_flags(disk);

            debug_println!("{}\nError Details: {:#?}\n", "ERR".red().bold(), errors);
            continue;
        }

        if !StatusRegister::is_status(disk, StatusFlags::DRQ) {
            unreachable!("The 'DRQ' should be set at this point");
        }

        // Finally: Read the Identify Response
        let mut read_identify = Vec::new();
        for _ in 0..256 {
            let value = DataRegister::read_u16(disk);
            read_identify.push(value);
        }

        debug_println!("{}", "OK".green().bold());

        disks.push(ATADisk::new(disk, read_identify));
    }

    disks
}