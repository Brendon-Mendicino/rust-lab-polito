use std::error::Error;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, Default)]
pub struct SensorData {
    pub seq: u32, // sequenza letture
    pub values: [f32; 10],
    pub timestamp: u32,
}

pub struct BReader {}
pub struct BWriter {}
pub trait BufferMode {}
impl BufferMode for BReader {}
impl BufferMode for BWriter {}

struct BufferHead<T>
where T: Copy + Default {
    len: usize,
    index: usize,
    capacity: usize,
    data: [T; 10],
}

pub struct CircularBuffer<T, Mode: BufferMode>
where T: Copy + Default {
    head: Arc<Mutex<BufferHead<T>>>,
    mode: PhantomData<Mode>
}

impl<T> BufferHead<T>
where T: Copy + Default {
    pub fn default() -> Self {
        Self { len: 0, index: 0, capacity: 10, data: [T::default(); 10] }
    }
}

pub fn new_buffer<T>() -> (CircularBuffer<T, BReader>, CircularBuffer<T, BWriter>)
where T: Copy + Default {
    let head = Arc::new(Mutex::new(BufferHead::default()));
    (CircularBuffer::<T, BReader>::new(head.clone()), CircularBuffer::<T, BWriter>::new(head))
}

impl<T> CircularBuffer<T, BReader>
where T: Copy + Default {
    fn new(head: Arc<Mutex<BufferHead<T>>>) -> Self {
        Self { head, mode: PhantomData::<BReader> }
    }

    pub fn read_data(&mut self) -> Option<Vec<T>> {
        let mut data = Vec::new();

        let mut head = self.head.lock().unwrap();

        for index in 0..head.len {
            let pos = (index + head.index) % head.capacity;

            data.push(head.data[pos].clone());
        }
        head.index = 0;
        head.len = 0;

        Some(data)
    }
}

impl<T> CircularBuffer<T, BWriter> 
where T: Copy + Default {
    fn new(head: Arc<Mutex<BufferHead<T>>>) -> Self {
        Self { head, mode: PhantomData::<BWriter> }
    }

    pub fn write_data(&mut self, data: T) -> Result<(), Box<dyn Error>> {
        let mut head = self.head.lock().unwrap();

        // if buffer is full don't write anything.
        if head.len != head.capacity {
            let pos = (head.index + head.len) % head.capacity;

            head.data[pos] = data;
        } else { 
            return Err("Buffer was full".into());
        }
        head.len += 1;

        Ok(())
    }
}
