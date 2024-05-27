use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use rppal::gpio::{Gpio, Trigger};
use tokio::time::interval;

use crate::db::{secs_f64_since_epoch, Measurement, DB};

#[derive(Debug)]
pub struct Davis {
    counting_handle: std::thread::JoinHandle<()>,
    db_update_handle: tokio::task::JoinHandle<()>,
    db: Arc<DB>,
}

impl Davis {
    pub async fn connect(db_path: String, simulation: bool) -> Result<Self> {
        tracing::info!("start connect to Davis sensor");
        let db = Arc::new(DB::connect(db_path).await?);
        // using a short period to remove some noise
        let period = Duration::from_secs_f64(2.25);
        let counter = Arc::new(AtomicU64::new(0));
        let counter_ptr = counter.clone();
        // Start a task incrementing a counter at every pulse from sensor
        let counting_handle = if simulation {
            std::thread::spawn(move || fake_counting_sync_loop(counter_ptr, period))
        } else {
            std::thread::spawn(|| counting_sync_loop(counter_ptr))
        };
        // Start a task storing wind speed at every period into a DB
        let db_clone = db.clone();
        let db_update_handle =
            tokio::task::spawn(async move { fetch_data_loop(period, counter, db_clone).await });
        Ok(Davis {
            counting_handle,
            db_update_handle,
            db,
        })
    }

    pub async fn current_wind(&self) -> Result<Measurement> {
        self.db.current_wind().await
    }

    pub async fn last_data(&self) -> Result<Measurement> {
        self.db.last_data().await
    }

    pub async fn oldest_data(&self) -> Result<Measurement> {
        self.db.oldest_data().await
    }

    pub async fn all_data_since(&self, t: Duration) -> Result<Vec<Measurement>> {
        self.db.data_since(t).await
    }

    pub async fn aggregated_data_since(
        &self,
        t: Duration,
        interval: Duration,
    ) -> Result<Vec<Measurement>> {
        let measures = self.db.data_since(t).await?;
        if measures.is_empty() {
            return Err(anyhow::anyhow!("No data returned"));
        }
        let mut agg: Vec<Vec<Measurement>> = vec![];
        let mut idx = 0;
        let interval = interval.as_secs_f64();
        let mut now = secs_f64_since_epoch();
        for m in measures.iter().rev() {
            if m.ts > now - interval {
                if agg.len() <= idx {
                    agg.push(vec![]);
                }
                agg[idx].push((*m).clone());
            } else {
                now -= interval;
                idx += 1;
                agg.push(vec![(*m).clone()]);
            }
        }
        Ok(agg
            .iter()
            .map(|v| Measurement {
                ts: v[0].ts,
                vel: v.iter().map(|m| m.vel).sum::<f64>() / v.len() as f64,
                direction: v.iter().map(|m| m.direction).sum::<u16>() / v.len() as u16,
            })
            .rev()
            .collect())
    }
}

/// That method simply increment a counter at almost random pace to simulate data from sensor
pub fn fake_counting_sync_loop(counter: Arc<AtomicU64>, period: Duration) {
    let mut sleep_time = period / 10;
    let mut start_ts = Instant::now();
    loop {
        counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(sleep_time);
        let duration_since_start = start_ts.elapsed();
        if duration_since_start > 3 * period {
            sleep_time = period / 10;
            start_ts = Instant::now();
        } else if duration_since_start > period * 2 {
            sleep_time = period / 100;
        }
    }
}

/// That method simply increment a counter at every pulse from sensor
pub fn counting_sync_loop(counter: Arc<AtomicU64>) {
    if let Err(err) = counting_sync_loop_inner(counter) {
        tracing::error!("HW loop has died, {:?} ", err);
        panic!("If we fail here, no need to continue");
    }
}

pub fn counting_sync_loop_inner(counter: Arc<AtomicU64>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut wind_io = gpio.get(5)?.into_input_pullup();
    wind_io.set_interrupt(Trigger::FallingEdge)?;
    let mut ts = Instant::now();
    loop {
        match wind_io.poll_interrupt(true, None) {
            Err(_e) => (),
            Ok(_info) => {
                // debounce stuff
                if ts.elapsed() > Duration::from_millis(18) {
                    ts = Instant::now();
                    counter.fetch_add(1, Ordering::SeqCst);
                };
            }
        }
    }
}

/// at every period, convert counter value to wind speed using formula from manufacturer
pub async fn fetch_data_loop(period: Duration, counter: Arc<AtomicU64>, db: Arc<DB>) {
    let mut interval = interval(period);
    interval.tick().await; // we skip first tick to get some data from sensor first
    loop {
        interval.tick().await;
        let count = counter.swap(0, Ordering::SeqCst);
        let wind_speed_mph = count as f64 * (2.25 / period.as_secs_f64());
        tracing::debug!("Number of IO edges: {:?}", &count);
        let vel = wind_speed_mph * 0.44704;
        if vel > 30.0 {
            // filter out too high values, something is wrong
            continue;
        }
        tracing::info!("Read vel: {:?}", &vel);
        if let Err(err) = db.insert_measurement(vel, 0).await {
            tracing::error!("Failed to write measurement in DB!, {:?}", err);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::SystemTime;

    use anyhow::Result;
    use tokio::time::sleep;
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn test_davis() -> Result<()> {
        let start = SystemTime::now();
        let davis = Davis::connect(String::from("./db-test2.sqlite"), true).await?;
        sleep(Duration::from_secs(6)).await;
        let data = davis.last_data().await?;
        assert!(data.vel > 0.0);
        assert!(data.ts > start);
        Ok(())
    }
}
