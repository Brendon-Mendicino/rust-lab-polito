use std::error::Error;
use std::{thread, mem};
use std::time::Duration;

use crate::shared::SensorData;

mod shared;

fn main() -> Result<(), Box<dyn Error>> {
    let mut file = shared::FileReader::new();

    let mut seq = 1..;
    let mut values =  [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    loop {
        thread::sleep(Duration::from_secs(1));
        let data = SensorData {
            seq: seq.next().unwrap(),
            values,
            timestamp: 0,
        };
        file.write_data(data)?;
        for n in values.iter_mut() { *n += 10.0; }

        println!("wrote: {:?}", data);

        if data.seq == 0 { break; }
    }

    Ok(())
}
