use std::{sync::Arc, time::Instant};

use anyhow::Result;
use rppal::gpio::Gpio;
use rust_wind::davis::{counting_sync_loop, WindSpeedData};

fn main() -> Result<()> {
    let data = Arc::new(WindSpeedData::new());
    let gpio = Gpio::new()?;
    let wind_speed = gpio.get(5)?.into_input_pullup();
    let new_data = data.clone();
    std::thread::spawn(|| counting_sync_loop(new_data, wind_speed));
    //let wind_dir = gpio.get(6)?.into_input();
    let mut last = Instant::now();
    loop {
        let elapsed = Instant::now() - last;
        if elapsed.as_secs_f64() > 2.0 {
            last = Instant::now();
            dbg!(data.get_speed());
        }

        //let speed = wind_speed.read();
        //let dir = wind_dir.read();
        //dbg!(speed, dir);
        //sleep(Duration::from_secs_f64(0.01));
    }
}
