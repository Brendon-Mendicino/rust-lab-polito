mod shared;
use std::time::Duration;

use shared::{CircularBuffer, SensorData, BWriter, BReader};

fn print_sensor(data: &Vec<shared::SensorData>) {
    for i in 0..10 {
        println!("sensor {:2}: max {}; min {}; avg {};",
            i,
            data.into_iter().map(|d| d.values[i]).reduce(f32::max).unwrap(),
            data.into_iter().map(|d| d.values[i]).reduce(f32::min).unwrap(),
            data.into_iter().map(|d| d.values[i]).sum::<f32>() / data.len() as f32
        );
    }
}

fn consumer(reader: &mut CircularBuffer<SensorData, BReader>) {
    loop {
        std::thread::sleep(Duration::from_secs(10));

        let data = reader.read_data();
        
        if let Some(data) = data {
            println!("{:#?}", data);
            print_sensor(&data);
        }
    }
}

fn producer(writer: &mut CircularBuffer<SensorData, BWriter>) {
    let mut seq = 1..;
    let mut values =  [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let data = SensorData {
            seq: seq.next().unwrap(),
            values,
            timestamp: 0,
        };

        if let Err(_) = writer.write_data(data) {
            continue;
        }
        for n in values.iter_mut() { *n += 10.0; }

        println!("wrote: {:?}", data);

        if data.seq == 0 { break; }
    }
}



fn main() {
    let (mut r,mut w) = shared::new_buffer();
    std::thread::scope(|s| {
        s.spawn(|| consumer(&mut r));
        s.spawn(|| producer(&mut w));
    });
}
