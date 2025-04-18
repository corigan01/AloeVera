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

use self::bpb::Bpb;
use crate::{
    error::{FsError, Result},
    io::SeekFrom,
};
use crate::{
    fatfs::inode::{DirectoryEntry, Inode},
    io::{Read, Seek},
};
use core::{cell::SyncUnsafeCell, fmt::Debug, mem::size_of};

mod bpb;
mod inode;

#[derive(Debug)]
pub enum FatKind {
    Fat12,
    Fat16,
    Fat32,
}

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct Fat<Part: ReadSeek> {
    disk: Part,
    bpb: Bpb,
}

type ClusterId = u32;

#[derive(Debug, Clone, Copy)]
enum FatEntry {
    Free,
    Next(ClusterId),
    EOF,
    Reserved,
    Defective,
}

// FIXME: Bug: Rust thinks some of these constants are not being used, yet they are
//        used in `from_fat16` and `from_fat32` which are being used. Maybe a bug with
//        Rust?
#[allow(dead_code)]
impl FatEntry {
    const FREE_CLUSTER: u32 = 0;
    const ALLOCATED_CLUSTER_BEGIN: u32 = 2;
    const FAT16_MAX: u32 = 0xfff4;
    const FAT16_RESERVED_END: u32 = 0xfff6;
    const FAT16_DEFECTIVE: u32 = Self::FAT16_RESERVED_END + 1;
    const FAT16_EOF: u32 = u16::MAX as u32;
    const FAT32_MAX: u32 = 0xffffff4;
    const FAT32_RESERVED_END: u32 = 0xffffff6;
    const FAT32_DEFECTIVE: u32 = Self::FAT32_RESERVED_END + 1;
    const FAT32_EOF: u32 = u32::MAX;

    fn from_fat16(id: ClusterId) -> FatEntry {
        match id {
            Self::FREE_CLUSTER => FatEntry::Free,
            Self::ALLOCATED_CLUSTER_BEGIN..=Self::FAT16_MAX => FatEntry::Next(id),
            ..=Self::FAT16_RESERVED_END => FatEntry::Reserved,
            Self::FAT16_DEFECTIVE => FatEntry::Defective,
            Self::FAT16_EOF => FatEntry::EOF,
            _ => unreachable!("ClusterID Unknown"),
        }
    }

    fn from_fat32(id: ClusterId) -> FatEntry {
        match id {
            Self::FREE_CLUSTER => FatEntry::Free,
            Self::ALLOCATED_CLUSTER_BEGIN..=Self::FAT32_MAX => FatEntry::Next(id),
            ..=Self::FAT32_RESERVED_END => FatEntry::Reserved,
            Self::FAT32_DEFECTIVE => FatEntry::Defective,
            Self::FAT32_EOF => FatEntry::EOF,
            _ => unreachable!("ClusterID Unknown"),
        }
    }
}

pub struct FatFile<'a, Part: ReadSeek> {
    filesize: usize,
    start_cluster: ClusterId,
    last_cluster: Option<(ClusterId, u64)>,
    fatfs: &'a mut Fat<Part>,
    seek: u64,
}

impl<'a, Part: ReadSeek> FatFile<'a, Part> {
    pub const fn filesize(&self) -> usize {
        self.filesize
    }
}

impl<'a, Part> FatFile<'a, Part> where Part: ReadSeek {}
impl<'a, Part> Seek for FatFile<'a, Part>
where
    Part: ReadSeek,
{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(pos) => self.seek = pos,
            _ => todo!("SeekFrom is not fully implemented, only Start(x) is implemented"),
        }
        Ok(self.seek)
    }

    fn stream_position(&mut self) -> u64 {
        self.seek
    }
}

impl<'a, Part> Read for FatFile<'a, Part>
where
    Part: ReadSeek,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let cluster_bytes =
            (self.fatfs.bpb.cluster_sectors() * self.fatfs.bpb.sector_size()) as u64;
        let mut bytes_read = 0;

        loop {
            let (cluster_id, offset) = match self.last_cluster {
                Some((last_cluster, last_seek)) if last_seek <= self.seek => {
                    (last_cluster, self.seek - last_seek)
                }
                _ => (self.start_cluster, self.seek),
            };

            let cluster_info = self.fatfs.cluster_of_offset(cluster_id, offset)?;
            self.last_cluster = Some((cluster_info.0, self.seek));

            let disk_loc = self.fatfs.bpb.cluster_physical_loc(cluster_info.0) + cluster_info.1;

            self.fatfs.disk.seek(SeekFrom::Start(disk_loc))?;
            let bytes_until_cluster_end = cluster_bytes - cluster_info.1;
            let bytes_until_read_end = bytes_until_cluster_end.min((buf.len() - bytes_read) as u64);

            self.fatfs
                .disk
                .read(&mut buf[bytes_read..bytes_read + bytes_until_read_end as usize])?;

            bytes_read += bytes_until_read_end as usize;
            self.seek += bytes_until_read_end;

            assert!(
                bytes_read <= buf.len(),
                "Attemped to more bytes ({}) than buffer's capacity ({})!",
                bytes_read,
                buf.len()
            );

            if bytes_read == buf.len() {
                return Ok(bytes_read);
            }
        }
    }
}

static FAT_BLOCK_RESERVE: SyncUnsafeCell<(u64, [u8; 512])> = SyncUnsafeCell::new((0, [0; 512]));

impl<Part: ReadSeek> Fat<Part> {
    pub fn new(mut disk: Part) -> Result<Self> {
        let bpb = Bpb::new(&mut disk)?;

        Ok(Self { disk, bpb })
    }

    fn read_fat(&mut self, id: ClusterId) -> Result<FatEntry> {
        let fat_region = self.bpb.fat_range();
        let entries_per_sector = (self.bpb.sector_size()) / self.bpb.fat_entry_bytes();

        let entry_sector = (id / entries_per_sector as u32) as u64 + *fat_region.start();
        let entry_offset = (id % entries_per_sector as u32) as usize;

        if entry_sector > *fat_region.end() {
            return Err(FsError::InvalidInput);
        }

        if entry_sector != unsafe { (&*FAT_BLOCK_RESERVE.get()).0 } {
            self.disk.seek(SeekFrom::Start(
                entry_sector * self.bpb.sector_size() as u64,
            ))?;
            unsafe {
                self.disk
                    .read(&mut (&mut *FAT_BLOCK_RESERVE.get()).1.as_mut())?;
                (&mut *FAT_BLOCK_RESERVE.get()).0 = entry_sector;
            }
        }

        Ok(match self.bpb.kind() {
            FatKind::Fat16 => unsafe {
                let arr = core::slice::from_raw_parts(
                    (&*FAT_BLOCK_RESERVE.get()).1.as_ptr() as *const u16,
                    256,
                );
                FatEntry::from_fat16(arr[entry_offset] as u32)
            },
            FatKind::Fat32 => unsafe {
                let arr = core::slice::from_raw_parts(
                    (&*FAT_BLOCK_RESERVE.get()).1.as_ptr() as *const u32,
                    128,
                );
                FatEntry::from_fat32(arr[entry_offset])
            },
            FatKind::Fat12 => todo!("Support reading FAT12"),
        })
    }

    fn cluster_of_offset(
        &mut self,
        cluster_start: ClusterId,
        offset: u64,
    ) -> Result<(ClusterId, u64)> {
        let mut search_cluster = cluster_start;
        let mut total_offset = 0;
        let cluster_size_bytes = self.bpb.cluster_sectors() as u64 * self.bpb.sector_size() as u64;

        loop {
            if offset - total_offset < cluster_size_bytes {
                return Ok((search_cluster, offset % cluster_size_bytes));
            }

            match self.read_fat(search_cluster)? {
                FatEntry::Next(next) => {
                    search_cluster = next;
                    total_offset += cluster_size_bytes;
                }
                FatEntry::EOF => return Err(FsError::EndOfFile),
                _ => return Err(FsError::ReadError),
            }
        }
    }

    pub fn volume_label<'a>(&'a self) -> &'a str {
        self.bpb.volume_label()
    }

    pub fn open<'a>(&'a mut self, name: &str) -> Result<FatFile<'a, Part>> {
        let entry_info = self.entry_of(name)?;

        Ok(FatFile {
            filesize: entry_info.file_size as usize,
            start_cluster: entry_info.cluster_id(),
            fatfs: self,
            seek: 0,
            last_cluster: None,
        })
    }

    pub fn entry_of(&mut self, name: &str) -> Result<DirectoryEntry> {
        assert_eq!(
            self.bpb.cluster_sectors(),
            2,
            "TODO: Expecting cluster size to be 2 sectors"
        );

        let mut path = name.split('/').filter(|str| !str.is_empty()).peekable();
        let mut inode_cluster = self.bpb.root_cluster();
        let mut data = [0u8; 1024];

        'outer: loop {
            let Some(path_part) = path.next() else {
                unreachable!("path_part is somehow none");
            };

            // Max string size for FAT is 256-chars
            let mut filename_str = [0u8; 256];
            let mut filename_len = 0;

            self.disk.seek(SeekFrom::Start(
                self.bpb.cluster_physical_loc(inode_cluster),
            ))?;
            self.disk.read(&mut data)?;

            for inode in data
                .chunks(size_of::<DirectoryEntry>())
                .map(|slice| slice.try_into())
                .filter_map(|entry: Result<Inode>| entry.ok())
            {
                let filename = core::str::from_utf8(&filename_str[..filename_len])
                    .unwrap_or("")
                    .trim();

                match inode {
                    Inode::LongFileName(lfn) => {
                        let ordering_number = (lfn.ordering - 1) & (u8::MAX ^ 0x40);
                        let offset = (ordering_number * 13) as usize;

                        filename_str[offset..(offset + 13)]
                            .iter_mut()
                            .zip(
                                inode
                                    .name_iter()
                                    .filter(|lfn_c| lfn_c.is_ascii() && *lfn_c != '\0'),
                            )
                            .for_each(|(filename_c, inode_c)| {
                                *filename_c = inode_c as u8;
                                filename_len += 1;
                            });
                    }
                    Inode::Dir(entry) => {
                        if path_part.trim().eq_ignore_ascii_case(filename) {
                            // more todo
                            if path.peek().is_some() {
                                inode_cluster = entry.cluster_id();
                                continue 'outer;
                            }

                            return Ok(entry);
                        }

                        filename_str = [0u8; 256];
                        filename_len = 0;
                        continue;
                    }
                    Inode::File(file) => {
                        // Files cannot have other files after it in the path:
                        // So, we must not be the one.
                        if path.peek().is_some() {
                            filename_str = [0u8; 256];
                            filename_len = 0;
                            continue;
                        }

                        if path_part.trim().eq_ignore_ascii_case(filename) {
                            return Ok(file);
                        }

                        filename_str = [0u8; 256];
                        filename_len = 0;
                    }
                }
            }

            return Err(FsError::NotFound);
        }
    }
}

impl<Part: ReadSeek> Debug for Fat<Part> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Fat")
            .field("kind", &self.bpb.kind())
            .field("bytes", &(self.bpb.total_sectors() * 512))
            .field("name", &self.volume_label())
            .finish()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        assert!(true, "True Should Be True!");
    }
}
