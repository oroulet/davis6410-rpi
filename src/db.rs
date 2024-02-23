use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug)]
struct DB {
    conn: Connection,
}

#[derive(Debug)]
struct Measurement {
    ts: SystemTime,
    vel: f64,
    direction: u16,
}

impl DB {
    pub fn connect(path: &Path) -> Result<Self> {
        tracing::warn!("Opening sqlite DB at {:?}", &path);
        let conn = Connection::open(path)?;
        tracing::warn!("DB opened");
        Ok(Self { conn })
    }
    pub fn create_tables(&self, force_delete: bool) -> Result<()> {
        if force_delete {
            tracing::warn!("Force deleting tables");
            self.conn.execute("DROP TABLE IF EXISTS wind", ())?;
        }
        tracing::warn!("Creating tables");
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS wind (
            ts    REAL PRIMARY KEY,
            vel  REAL,
            direction  INTEGER
        )",
            (), // empty list of parameters.
        )?;
        tracing::warn!("Tables created");
        Ok(())
    }

    pub fn insert_measurement(&self, vel: f64, direction: u16) -> Result<()> {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?;
        self.insert_measurement_at_t(ts, vel, direction)
    }

    pub fn insert_measurement_at_t(
        &self,
        ts_since_epoch: Duration,
        vel: f64,
        direction: u16,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO wind (ts, vel, direction) VALUES (?1, ?2, ?3)",
            (&ts_since_epoch.as_secs_f64(), &vel, &direction),
        )?;
        Ok(())
    }

    pub fn data_since(&self, duration: Duration) -> Result<Vec<Measurement>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let threshold = now.as_secs_f64() - duration.as_secs_f64();
        let mut stmt = self
            .conn
            .prepare("SELECT ts, vel, direction FROM wind WHERE ts > ?1")?;
        let rows = stmt.query_map([threshold], |row| {
            Ok(Measurement {
                ts: UNIX_EPOCH + Duration::from_secs_f64(row.get(0)?),
                vel: row.get(1)?,
                direction: row.get(2)?,
            })
        })?;
        let mut measurements = Vec::new();
        for row in rows {
            measurements.push(row?)
        }
        Ok(measurements)
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Once;

    use anyhow::Result;

    use super::*;

    static START: Once = Once::new();

    fn init_tracing() {
        START.call_once(|| {
            tracing_subscriber::fmt::init();
        });
    }

    #[test]
    fn test_db() -> Result<()> {
        init_tracing();
        let db = DB::connect(Path::new("./test_db.sqlite"))?;
        db.create_tables(true)?;
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)? - Duration::from_secs(10);
        db.insert_measurement_at_t(ts, 50000.0, 2)?;
        for i in 1..10 {
            db.insert_measurement(i as f64, 2)?;
        }
        let data = db.data_since(Duration::from_secs(5))?;
        let mean: f64 = data.iter().map(|m| m.vel).sum::<f64>() / data.len() as f64;
        dbg!(&data, mean);
        assert_eq!(mean, 5.0);
        let data = db.data_since(Duration::from_secs(15))?;
        let mean: f64 = data.iter().map(|m| m.vel).sum::<f64>() / data.len() as f64;
        dbg!(&data, mean);
        assert!(mean > 10.0);
        Ok(())
    }
}
