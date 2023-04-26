use std::error::Error;
use std::thread;
use std::time::Duration;

mod shared;


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

fn main() -> Result<(), Box<dyn Error>> {
    let mut file = shared::FileReader::new();

    loop {
        thread::sleep(Duration::from_secs(10));

        let data = file.read_data()?;
        
        println!("{:#?}", data);
        print_sensor(&data);

        if data.len() == 11 { break; }
    }

    Ok(())
}
