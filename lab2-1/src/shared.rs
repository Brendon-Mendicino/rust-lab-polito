use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::time::Duration;
use std::{mem, thread};
use std::os::unix::prelude::FileExt;
use std::path::{Path, PathBuf};

use fcntl::FcntlLockType;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SensorData {
    pub seq: u32, // sequenza letture
    pub values: [f32; 10],
    pub timestamp: u32,
}

#[repr(C)]
struct CircularBuffer {
    len: u32,
    index: u32,
    capacity: u32,
}

pub struct FileReader {
    file: PathBuf,
}

impl SensorData {
    pub fn default() -> Self {
        Self {
            seq: 0,
            values: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
            timestamp: 0,
        }
    }

    fn serialize(self) -> [u8; mem::size_of::<Self>()] {
        unsafe { mem::transmute::<Self, [u8; mem::size_of::<Self>()]>(self) }
    }

    fn deserialize(bytes: [u8; mem::size_of::<Self>()]) -> Self {
        unsafe { mem::transmute::<[u8; mem::size_of::<Self>()], Self>(bytes) }
    }
}

impl CircularBuffer {
    fn default() -> Self {
        Self {
            len: 0,
            index: 0,
            capacity: 10,
        }
    }

    fn serialize(self) -> [u8; mem::size_of::<Self>()] {
        unsafe { mem::transmute::<Self, [u8; mem::size_of::<Self>()]>(self) }
    }

    fn deserialize(bytes: [u8; mem::size_of::<Self>()]) -> Self {
        unsafe { mem::transmute::<[u8; mem::size_of::<Self>()], Self>(bytes) }
    }
}

impl FileReader {
    pub fn new() -> Self {
        Self {
            file: "cicular".into(),
        }
    }

    fn init_file(file: &Path) -> Result<(), Box<dyn Error>> {
        let mut output = File::create(file)?;

        let head = CircularBuffer::default().serialize();
        output.write_all(&head)?;

        // wirte capcity * size byte of SensorData
        for _ in 0..CircularBuffer::default().capacity {
            output.write_all(&[0u8; mem::size_of::<SensorData>()])?;
        }

        Ok(())
    }

    pub fn write_data(&mut self, data: SensorData) -> Result<(), Box<dyn Error>> {
        let file_exists = Path::new(&self.file).try_exists()?;
        if !file_exists {
            println!("write_data: file created");
            FileReader::init_file(&self.file)?;
        }

        let mut output = OpenOptions::new().read(true).write(true).open(&self.file)?;
        while !fcntl::lock_file(&output, None, Some(FcntlLockType::Write))? {
            thread::sleep(Duration::from_millis(100));
        }

        let mut head_bytes = [0u8; mem::size_of::<CircularBuffer>()];
        output.read_exact(&mut head_bytes)?;

        let mut head = CircularBuffer::deserialize(head_bytes);

        // if buffer is full don't write anything.
        if head.len != head.capacity {
            let head_size = mem::size_of::<CircularBuffer>();
            let write_position = ((head.index + head.len) % head.capacity) as usize
                * mem::size_of::<SensorData>()
                + head_size;

            output.write_at(&data.serialize(), write_position as u64)?;

            // update head
            head.len = head.len + 1;
            output.write_at(&head.serialize(), 0)?;
        }

        if !fcntl::unlock_file(&output, None)? {
            return Err("Could not unlock file!".into());
        }

        Ok(())
    }

    pub fn read_data(&mut self) -> Result<Vec<SensorData>, Box<dyn Error>> {
        let file_exists = Path::new(&self.file).try_exists()?;
        if !file_exists {
            FileReader::init_file(&self.file)?;
        }

        let mut data = Vec::new();

        let mut input = OpenOptions::new().read(true).write(true).open(&self.file)?;

        while !fcntl::lock_file(&input, None, Some(FcntlLockType::Write))? {
            thread::sleep(Duration::from_millis(100));
        }


        let mut head_bytes = [0u8; mem::size_of::<CircularBuffer>()];
        input.read_exact(&mut head_bytes)?;

        let mut head = CircularBuffer::deserialize(head_bytes);

        let mut data_bytes = [0u8; mem::size_of::<SensorData>()];
        for _ in 0..head.len {
            let head_size = mem::size_of::<CircularBuffer>();
            let read_position = (head.index % head.capacity) as usize
                * mem::size_of::<SensorData>()
                + head_size;

            input.read_at(&mut data_bytes, read_position as u64)?;
            data.push(SensorData::deserialize(data_bytes));

            head.index = (head.index + 1) % head.capacity;
            head.len -= 1;
        }

        // update header
        input.write_at(&CircularBuffer::default().serialize(), 0)?;

        if !fcntl::unlock_file(&input, None)? {
            return Err("Could not unlock file!".into());
        }

        Ok(data)
    }


}
