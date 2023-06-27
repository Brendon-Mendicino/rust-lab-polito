use clap::Parser;
use std::ffi::CStr;
use std::fs::File;
use std::io::{Read, self};
use std::mem::{size_of, self};
use std::path::PathBuf;
use std::str::FromStr;


#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {

    /// Input file
    #[arg(short, long)]
    input: PathBuf,
}


#[derive(Clone, Copy)]
#[repr(C)]
struct Value {
    data_type: i32,
    val: f32,
    timestamp: i64,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct MValue {
    data_type: i32,
    val: [f32; 10],
    timestamp: i64,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Message {
    data_type: i32,
    message: [u8; 21],
}

#[derive(Clone, Copy)]
#[repr(C)]
union DataUnion {
    value: Value,
    m_value: MValue,
    message: Message,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct CData {
    data_type: i32,
    data_union: DataUnion,
}

#[allow(dead_code)]
#[derive(Debug)]
enum RustData {
    Value {
        //data_type: i32,
        val: f32,
        timestamp: i64,
    },
    MValue {
        //data_type: i32,
        val: [f32; 10],
        timestamp: i64,
    },
    Message {
        //data_type: i32,
        message: String,
    }
}

impl CData {
    fn from_file(file: &mut File) -> io::Result<Vec<RustData>> {
        let mut data = Vec::<CData>::with_capacity(100);
        let mut buffer = [0u8; size_of::<CData>()];

        for _ in 0..100 {
            file.read_exact(&mut buffer)?;
            let c_data: CData = unsafe { mem::transmute(buffer) };
            data.push(c_data);
        }

        Ok(data.into_iter().map(|d| d.to_rust()).collect())
    }

    fn to_rust(self) -> RustData {
        unsafe {
            match self.data_type {
                1 => RustData::Value {
                    //data_type: self.data_union.value.data_type,
                    val: self.data_union.value.val,
                    timestamp: self.data_union.value.timestamp
                },
                2 => RustData::MValue { 
                    //data_type: self.data_union.m_value.data_type,
                    val: self.data_union.m_value.val, 
                    timestamp: self.data_union.m_value.timestamp 
                },
                3 => {
                    let c_message = self.data_union.message.message;
                    let first_null = c_message.iter().position(|c| *c == b'\0').unwrap();

                    // Generate CStr from raw bytes and then convert it to String
                    let c_str = CStr::from_bytes_with_nul(&c_message[..=first_null]).expect("Cannot read string!");
                    let message = String::from_str(c_str.to_str().unwrap()).unwrap();

                    RustData::Message {
                        //data_type: self.data_union.message.data_type, 
                        message 
                    }
                }
                _ => panic!("Unexpected value: {}!", self.data_type)
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut file = File::open(args.input)?;

    let data = CData::from_file(&mut file)?;

    data.iter()
        .for_each(|d| println!("{:?}", d));

    Ok(())
}
