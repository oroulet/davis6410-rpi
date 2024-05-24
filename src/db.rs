use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::Serialize;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};

#[derive(Debug)]
pub struct DB {
    pool: SqlitePool,
}

#[derive(Debug, Serialize, Clone)]
pub struct Measurement {
    pub ts: f64,
    pub vel: f64,
    pub direction: u16,
}

pub fn secs_f64_since_epoch() -> f64 {
    duration_since_epoch().as_secs_f64()
}

pub fn duration_since_epoch() -> Duration {
    //we know we are not at epoch so this will never fail
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

impl DB {
    pub async fn connect(path: String) -> Result<Self> {
        if Sqlite::database_exists(&path).await.unwrap_or(false) {
            tracing::info!("Database already exists");
        } else {
            tracing::warn!("Creating database {}", &path);
            match Sqlite::create_database(&path).await {
                Ok(()) => println!("Create db success"),
                Err(error) => panic!("error: {error}"),
            }
        }

        let pool = sqlx::SqlitePool::connect(path.as_str()).await?;
        let db = Self { pool };
        db.create_tables(false).await?;
        Ok(db)
    }

    pub async fn clean(&self) -> Result<()> {
        let now = secs_f64_since_epoch();
        let one_month = now - 30.0 * 24.0 * 60.0 * 60.0;
        sqlx::query!("DELETE FROM wind WHERE ts < ?1", one_month)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_tables(&self, force_delete: bool) -> Result<()> {
        if force_delete {
            tracing::warn!("Force deleting tables");
            sqlx::query!("DROP TABLE IF EXISTS wind")
                .execute(&self.pool)
                .await?;
        }
        tracing::info!("Creating tables if needed");
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS wind (
            ts    REAL PRIMARY KEY,
            vel  REAL,
            direction  INTEGER
        )"
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_measurement(&self, vel: f64, direction: u16) -> Result<()> {
        let ts = duration_since_epoch();
        self.insert_measurement_at_t(ts, vel, direction).await
    }

    pub async fn insert_measurement_at_t(
        &self,
        ts_since_epoch: Duration,
        vel: f64,
        direction: u16,
    ) -> Result<()> {
        let ts = ts_since_epoch.as_secs_f64();
        tracing::info!("DB: Inserting: ts: {:?}, vel: {:?}m/s", &ts, &vel);
        sqlx::query!(
            "INSERT INTO wind (ts, vel, direction) VALUES (?1, ?2, ?3)",
            ts,
            vel,
            direction,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn current_wind(&self) -> Result<Measurement> {
        let d = self.data_since(Duration::from_secs(60)).await?;
        match d.last() {
            Some(m) => Ok(m.clone()),
            None => Err(anyhow::anyhow!("No data avaialble in db yet")),
        }
    }

    pub async fn data_since(&self, duration: Duration) -> Result<Vec<Measurement>> {
        let now = secs_f64_since_epoch();
        let threshold = now - duration.as_secs_f64();
        let res = sqlx::query!(
            "SELECT ts, vel, direction FROM wind WHERE ts > ?1",
            threshold
        )
        .fetch_all(&self.pool)
        .await?;

        let mut measurements = Vec::new();
        for row in res {
            let mesurement = Measurement {
                ts: row.ts.ok_or_else(|| anyhow::anyhow!("not found"))?,
                vel: row.vel.ok_or_else(|| anyhow::anyhow!("not found"))?,
                direction: row
                    .direction
                    .ok_or_else(|| anyhow::anyhow!("Not found"))?
                    .try_into()?,
            };
            measurements.push(mesurement);
        }
        tracing::info!("Sending range: {:?}", &measurements);
        Ok(measurements)
    }

    pub async fn last_data(&self) -> Result<Measurement> {
        let row = sqlx::query!("SELECT ts, vel, direction  FROM wind ORDER BY ts DESC LIMIT 1",)
            .fetch_one(&self.pool)
            .await?;
        tracing::info!("Sending last data: {:?}", &row);
        Ok(Measurement {
            ts: row.ts.ok_or_else(|| anyhow::anyhow!("not found"))?,
            vel: row.vel.ok_or_else(|| anyhow::anyhow!("Not found"))?,
            direction: row
                .direction
                .ok_or_else(|| anyhow::anyhow!("Not found"))?
                .try_into()?,
        })
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn test_db() -> Result<()> {
        let db = DB::connect("./test_db1.sqlite".to_string()).await?;
        db.create_tables(true).await?;
        let ts = duration_since_epoch() - Duration::from_secs(10);
        db.insert_measurement_at_t(ts, 50000.0, 2).await?;
        for i in 1..10 {
            db.insert_measurement(i as f64, 2).await?;
        }
        let data = db.data_since(Duration::from_secs(5)).await?;
        let mean: f64 = data.iter().map(|m| m.vel).sum::<f64>() / data.len() as f64;
        dbg!(&data, mean);
        assert_eq!(mean, 5.0);
        let data = db.data_since(Duration::from_secs(15)).await?;
        let mean: f64 = data.iter().map(|m| m.vel).sum::<f64>() / data.len() as f64;
        dbg!(&data, mean);
        assert!(mean > 10.0);
        Ok(())
    }
}
