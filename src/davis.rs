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

use crate::db::{secs_f64_since_epoch, Measurement, DB};

#[derive(Debug)]
pub struct Davis {
    counting_handle: std::thread::JoinHandle<()>,
    update_handle: tokio::task::JoinHandle<()>,
    period: f64,
    db: Arc<DB>,
}

impl Davis {
    pub async fn connect(db_path: String, simulation: bool) -> Result<Self> {
        tracing::info!("start connect to Davis sensor");
        let db = Arc::new(DB::connect(db_path).await?);
        let period = 30.0;
        let counter = Arc::new(AtomicU64::new(0));
        let counter_ptr = counter.clone();
        let counting_handle = if simulation {
            std::thread::spawn(move || fake_counting_sync_loop(counter_ptr, period))
        } else {
            std::thread::spawn(|| counting_sync_loop(counter_ptr))
        };
        let db_clone = db.clone();
        let update_handle =
            tokio::task::spawn(async move { fetch_data_loop(period, counter, db_clone).await });
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
        let mut agg: Vec<Vec<Measurement>> = vec![vec![]];
        let mut idx = 0;
        let interval = interval.as_secs_f64();
        let mut now = secs_f64_since_epoch();
        for m in measures.iter().rev() {
            if m.ts > now - interval {
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

pub fn fake_counting_sync_loop(counter: Arc<AtomicU64>, period: f64) {
    let mut sleep_time = period / 10.0;
    let mut start_ts = Instant::now();
    loop {
        counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_secs_f64(sleep_time));
        let duration_since_start = (Instant::now() - start_ts).as_secs_f64();
        if duration_since_start > 2.5 * period {
            sleep_time = period / 10.0;
            start_ts = Instant::now();
        } else if duration_since_start > period * 1.5 {
            sleep_time = period / 100.0;
        }
    }
}

pub fn counting_sync_loop(counter: Arc<AtomicU64>) {
    if let Err(err) = counting_sync_loop_inner(counter) {
        tracing::error!("HW loop has died, {:?} ", err);
        panic!("If we fail here, no need to continue");
    }
}

pub fn counting_sync_loop_inner(counter: Arc<AtomicU64>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut wind_io = gpio.get(5)?.into_input_pullup();
    wind_io.set_interrupt(Trigger::RisingEdge)?;
    let mut ts = Instant::now();
    loop {
        match wind_io.poll_interrupt(true, Some(Duration::from_secs(10))) {
            Err(_e) => (),
            Ok(_info) => {
                if Instant::now() - ts < Duration::from_secs_f64(0.002) {
                    continue;
                };
                ts = Instant::now();
                counter.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

pub async fn fetch_data_loop(period: f64, counter: Arc<AtomicU64>, db: Arc<DB>) {
    let mut last_call = Instant::now();
    loop {
        sleep(Duration::from_secs_f64(period)).await;
        let now = Instant::now();
        let elapsed = now - last_call;
        last_call = now;
        let count = counter.swap(0, Ordering::SeqCst);
        let wind_speed_mph = count as f64 * (2.25 / elapsed.as_secs_f64());

        tracing::debug!("Number of IO edges: {:?}", &count);
        let vel = wind_speed_mph * 0.44704;
        tracing::info!("Read vel: {:?}", &vel);
        if let Err(err) = db.insert_measurement(vel, 0).await {
            tracing::error!("Failed to write measurement in DB!, {:?}", err)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::SystemTime;

    use anyhow::Result;
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
