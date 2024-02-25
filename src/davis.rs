use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use rppal::gpio::{Gpio, Trigger};
use tokio::time::sleep;

use crate::db::{Measurement, DB};

#[derive(Debug)]
pub struct Davis {
    counting_handle: std::thread::JoinHandle<Result<()>>,
    update_handle: tokio::task::JoinHandle<Result<()>>,
    period: f64,
    db: Arc<DB>,
}

impl Davis {
    pub async fn connect(db_path: String) -> Result<Self> {
        dbg!("start connect to Davis sensor");
        let db = Arc::new(DB::connect(db_path).await?);
        let period = 5.0;
        let counter = Arc::new(AtomicU64::new(0));
        let counter_ptr = counter.clone();
        let counting_handle = std::thread::spawn(|| counting_sync_loop(counter_ptr));
        let db_clone = db.clone();
        let update_handle =
            tokio::task::spawn(async move { fetch_data_loop(period, counter, db_clone).await });
        dbg!("end connect");
        Ok(Davis {
            counting_handle,
            update_handle,
            period,
            db,
        })
    }
    pub async fn last_data(&self) -> Result<Measurement> {
        self.db.last_data().await
    }
}

pub fn counting_sync_loop(counter: Arc<AtomicU64>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut wind_io = gpio.get(5)?.into_input_pullup();
    wind_io.set_interrupt(Trigger::RisingEdge)?;
    let mut ts = Instant::now();
    loop {
        match wind_io.poll_interrupt(true, Some(Duration::from_secs(10))) {
            Err(_e) => (),
            Ok(_info) => {
                dbg!("count");
                if Instant::now() - ts < Duration::from_secs_f64(0.002) {
                    continue;
                };
                ts = Instant::now();
                counter.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

pub async fn fetch_data_loop(period: f64, counter: Arc<AtomicU64>, db: Arc<DB>) -> Result<()> {
    let mut last_call = Instant::now();
    loop {
        let now = Instant::now();
        let elapsed = now - last_call;
        last_call = now;
        let count = counter.swap(0, Ordering::SeqCst);
        let wind_speed_mph = count as f64 * (2.25 / elapsed.as_secs_f64());

        dbg!(count, elapsed, counter.load(Ordering::SeqCst));
        let vel = wind_speed_mph * 0.44704;
        dbg!(&vel);
        db.insert_measurement(vel, 0).await?;
        sleep(Duration::from_secs_f64(period)).await;
    }
}
