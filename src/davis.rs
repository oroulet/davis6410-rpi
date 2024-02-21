use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use rppal::gpio::{InputPin, Trigger};

pub struct WindSpeedData {
    count: AtomicU64,
    last_call: Mutex<Instant>,
    state_ts: Mutex<Instant>,
}

impl WindSpeedData {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            last_call: Mutex::new(Instant::now()),
            state_ts: Mutex::new(Instant::now()),
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

    pub fn increase(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

impl Default for WindSpeedData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn count_loop(data: Arc<WindSpeedData>, mut io: InputPin) -> Result<()> {
    io.set_interrupt(Trigger::RisingEdge)?;
    loop {
        match io.poll_interrupt(true, Some(Duration::from_secs(10))) {
            Err(_e) => (),
            Ok(_info) => {
                let mut ts = data.state_ts.lock().unwrap();
                if Instant::now() - *ts < Duration::from_secs_f64(0.002) {
                    continue;
                };
                *ts = Instant::now();
                data.increase();
            }
        }
    }
}
