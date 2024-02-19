use std::{thread::sleep, time::Duration};

use anyhow::Result;
use rppal::gpio::Gpio;

fn main() -> Result<()> {
    let gpio = Gpio::new()?;
    let wind_speed = gpio.get(5)?.into_input();
    let wind_dir = gpio.get(6)?.into_input();
    loop {
        let speed = wind_speed.read();
        let dir = wind_dir.read();
        dbg!(speed, dir);
        sleep(Duration::from_secs_f64(0.01));
    }
}
