use super::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_FUNCTION, ERROR_INVALID_HANDLE, ERROR_PATH_NOT_FOUND,
    ERROR_TOO_MANY_FILES,
};
use crate::cpu::registers::{DI, SI};
use crate::machine::{Halt, Machine};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

const FIRST_FILE_HANDLE: usize = 5;
const MAX_HANDLES: usize = 40;

pub struct FileTable {
    slots: Vec<Option<File>>,
}

impl FileTable {
    pub fn new() -> Self {
        FileTable { slots: (0..MAX_HANDLES).map(|_| None).collect() }
    }

    fn insert(&mut self, file: File) -> Result<u16, u16> {
        let free = (FIRST_FILE_HANDLE..MAX_HANDLES).find(|index| self.slots[*index].is_none());
        let index = free.ok_or(ERROR_TOO_MANY_FILES)?;
        self.slots[index] = Some(file);
        Ok(index as u16)
    }

    fn get(&mut self, handle: u16) -> Result<&mut File, u16> {
        self.slots
            .get_mut(handle as usize)
            .and_then(|slot| slot.as_mut())
            .ok_or(ERROR_INVALID_HANDLE)
    }

    fn close(&mut self, handle: u16) -> Result<(), u16> {
        let slot = self.slots.get_mut(handle as usize).ok_or(ERROR_INVALID_HANDLE)?;
        slot.take().map(|_| ()).ok_or(ERROR_INVALID_HANDLE)
    }

    fn duplicate(&mut self, handle: u16) -> Result<u16, u16> {
        let copy = self.get(handle)?.try_clone().map_err(|_| ERROR_TOO_MANY_FILES)?;
        self.insert(copy)
    }

    pub fn close_all(&mut self) {
        self.slots.iter_mut().for_each(|slot| *slot = None);
    }
}

fn io_error(error: std::io::Error) -> u16 {
    match error.kind() {
        std::io::ErrorKind::NotFound => ERROR_FILE_NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => ERROR_ACCESS_DENIED,
        _ => ERROR_ACCESS_DENIED,
    }
}

impl Machine {
    pub(crate) fn dos_files(&mut self) -> Result<(), Halt> {
        if self.cpu.regs.ah() == 0x3F && self.cpu.regs.bx() == 0 {
            let count = self.read_stdin()?;
            self.cpu.regs.set_ax(count);
            self.dos_ok();
            return Ok(());
        }
        match self.file_operation() {
            Ok(Some(value)) => {
                self.cpu.regs.set_ax(value);
                self.dos_ok();
            }
            Ok(None) => self.dos_ok(),
            Err(code) => self.dos_error(code),
        }
        Ok(())
    }

    fn path_argument(&self) -> String {
        self.memory.read_asciiz(self.cpu.regs.ds(), self.cpu.regs.dx())
    }

    fn file_operation(&mut self) -> Result<Option<u16>, u16> {
        let regs = self.cpu.regs.clone();
        match regs.ah() {
            0x39 => {
                let path = self.drive.resolve(&self.path_argument())?;
                fs::create_dir(path).map_err(|_| ERROR_ACCESS_DENIED)?;
                Ok(None)
            }
            0x3A => {
                let path = self.drive.resolve(&self.path_argument())?;
                fs::remove_dir(path).map_err(|_| ERROR_ACCESS_DENIED)?;
                Ok(None)
            }
            0x3B => {
                let path = self.path_argument();
                self.drive.change_directory(&path)?;
                Ok(None)
            }
            0x3C => {
                let path = self.drive.resolve(&self.path_argument())?;
                let file = File::create(path).map_err(io_error)?;
                Ok(Some(self.dos.files.insert(file)?))
            }
            0x3D => {
                let path = self.drive.resolve(&self.path_argument())?;
                if !path.is_file() {
                    return Err(ERROR_FILE_NOT_FOUND);
                }
                let mode = regs.al() & 3;
                let file = OpenOptions::new()
                    .read(mode != 1)
                    .write(mode != 0)
                    .open(path)
                    .map_err(io_error)?;
                Ok(Some(self.dos.files.insert(file)?))
            }
            0x3E => {
                if regs.bx() < FIRST_FILE_HANDLE as u16 {
                    return Ok(None);
                }
                self.dos.files.close(regs.bx())?;
                Ok(None)
            }
            0x3F => self.read_handle().map(Some),
            0x40 => self.write_handle().map(Some),
            0x41 => {
                let path = self.drive.resolve(&self.path_argument())?;
                fs::remove_file(path).map_err(io_error)?;
                Ok(None)
            }
            0x42 => {
                let distance = (((regs.cx() as u32) << 16) | regs.dx() as u32) as i32 as i64;
                let target = match regs.al() {
                    0 => SeekFrom::Start(distance.max(0) as u64),
                    1 => SeekFrom::Current(distance),
                    _ => SeekFrom::End(distance),
                };
                let position = self.dos.files.get(regs.bx())?.seek(target).map_err(|_| ERROR_ACCESS_DENIED)?;
                self.cpu.regs.set_dx((position >> 16) as u16);
                Ok(Some(position as u16))
            }
            0x43 => {
                let path = self.drive.resolve(&self.path_argument())?;
                let metadata = fs::metadata(path).map_err(|_| ERROR_FILE_NOT_FOUND)?;
                self.cpu.regs.set_cx(if metadata.is_dir() { 0x10 } else { 0x20 });
                Ok(None)
            }
            0x44 => match regs.al() {
                0x00 => {
                    let info = if regs.bx() < FIRST_FILE_HANDLE as u16 {
                        0x80D3
                    } else {
                        self.dos.files.get(regs.bx())?;
                        0x0002
                    };
                    self.cpu.regs.set_dx(info);
                    Ok(Some(info))
                }
                0x01 => Ok(None),
                _ => Err(ERROR_INVALID_FUNCTION),
            },
            0x45 => {
                if regs.bx() < FIRST_FILE_HANDLE as u16 {
                    return Ok(Some(regs.bx()));
                }
                Ok(Some(self.dos.files.duplicate(regs.bx())?))
            }
            0x47 => {
                let text = self.drive.current_relative();
                let mut bytes = text.into_bytes();
                bytes.truncate(63);
                bytes.push(0);
                self.memory.write_bytes(regs.ds(), regs.general[SI], &bytes);
                Ok(Some(0x0100))
            }
            _ => {
                let from = self.drive.resolve(&self.path_argument())?;
                let target_name = self.memory.read_asciiz(regs.es(), regs.general[DI]);
                let to = self.drive.resolve(&target_name).map_err(|_| ERROR_PATH_NOT_FOUND)?;
                fs::rename(from, to).map_err(io_error)?;
                Ok(None)
            }
        }
    }

    fn read_stdin(&mut self) -> Result<u16, Halt> {
        let regs = self.cpu.regs.clone();
        let count = regs.cx() as usize;
        let line = self.read_line(count.saturating_sub(2).min(126), &[])?;
        self.video.print(&mut self.memory, "\n");
        let mut bytes: Vec<u8> = line.bytes().collect();
        bytes.extend_from_slice(b"\r\n");
        bytes.truncate(count);
        self.memory.write_bytes(regs.ds(), regs.dx(), &bytes);
        Ok(bytes.len() as u16)
    }

    fn read_handle(&mut self) -> Result<u16, u16> {
        let regs = self.cpu.regs.clone();
        let count = regs.cx() as usize;
        let bytes = if regs.bx() < FIRST_FILE_HANDLE as u16 {
            Vec::new()
        } else {
            let mut buffer = vec![0u8; count];
            let read = self.dos.files.get(regs.bx())?.read(&mut buffer).map_err(|_| ERROR_ACCESS_DENIED)?;
            buffer.truncate(read);
            buffer
        };
        self.memory.write_bytes(regs.ds(), regs.dx(), &bytes);
        Ok(bytes.len() as u16)
    }

    fn write_handle(&mut self) -> Result<u16, u16> {
        let regs = self.cpu.regs.clone();
        let bytes = self.memory.read_bytes(regs.ds(), regs.dx(), regs.cx() as usize);
        match regs.bx() {
            1 | 2 => {
                for byte in &bytes {
                    self.video.teletype(&mut self.memory, *byte, None);
                }
                Ok(bytes.len() as u16)
            }
            0 | 3 | 4 => Ok(bytes.len() as u16),
            handle => {
                let file = self.dos.files.get(handle)?;
                if bytes.is_empty() {
                    let position = file.stream_position().map_err(|_| ERROR_ACCESS_DENIED)?;
                    file.set_len(position).map_err(|_| ERROR_ACCESS_DENIED)?;
                    return Ok(0);
                }
                file.write_all(&bytes).map_err(|_| ERROR_ACCESS_DENIED)?;
                Ok(bytes.len() as u16)
            }
        }
    }
}
