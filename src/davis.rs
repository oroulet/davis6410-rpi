use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use rppal::gpio::{Gpio, InputPin, Trigger};
use tokio::time::sleep;

#[derive(Debug)]
pub struct Davis {
    gpio: Gpio,
    counting_handle: std::thread::JoinHandle<Result<()>>,
    update_handle: tokio::task::JoinHandle<Result<()>>,
}

impl Davis {
    pub async fn connect() -> Result<Self> {
        dbg!("start connect to Davis sensor");
        let data = Arc::new(SensorData::new());
        let gpio = Gpio::new()?;
        let wind_speed = gpio.get(5)?.into_input_pullup();
        let new_data = data.clone();
        let counting_handle = std::thread::spawn(|| counting_sync_loop(new_data, wind_speed));
        let update_handle = tokio::task::spawn(async move { fetch_data_loop(data).await });
        dbg!("end connect");
        Ok(Davis {
            gpio,
            counting_handle,
            update_handle,
        })
    }
    pub fn get_current_wind(&self) -> f64 {
        1.23
    }
}

pub struct SensorData {
    count: AtomicU64,
    last_call: Mutex<Instant>,
    rising_edge_ts: Mutex<Instant>,
}

impl SensorData {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            last_call: Mutex::new(Instant::now()),
            rising_edge_ts: Mutex::new(Instant::now()),
        }
    }
    pub fn get_speed(&self) -> f64 {
        let now = Instant::now();
        let elapsed = {
            let mut last_call = self.last_call.lock().unwrap();

            let elasped = now - *last_call;
            *last_call = now;
            elasped
        };
        let count = self.count.swap(0, Ordering::SeqCst);
        let wind_speed_mph = count as f64 * (2.25 / elapsed.as_secs_f64());

        dbg!(count, elapsed, self.count.load(Ordering::SeqCst));
        wind_speed_mph * 0.44704
    }
}

impl Default for SensorData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn counting_sync_loop(data: Arc<SensorData>, mut io: InputPin) -> Result<()> {
    io.set_interrupt(Trigger::RisingEdge)?;
    loop {
        match io.poll_interrupt(true, Some(Duration::from_secs(10))) {
            Err(_e) => (),
            Ok(_info) => {
                dbg!("count");
                let mut ts = data.rising_edge_ts.lock().unwrap();
                if Instant::now() - *ts < Duration::from_secs_f64(0.002) {
                    continue;
                };
                *ts = Instant::now();
                data.count.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

pub async fn fetch_data_loop(data: Arc<SensorData>) -> Result<()> {
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
        sleep(Duration::from_secs_f64(0.01)).await;
    }
}