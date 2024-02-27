use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::Serialize;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};

#[derive(Debug)]
pub struct DB {
    pool: SqlitePool,
}

#[derive(Debug, Serialize)]
pub struct Measurement {
    pub ts: SystemTime,
    pub vel: f64,
    pub direction: u16,
}

impl DB {
    pub async fn connect(path: String) -> Result<Self> {
        if !Sqlite::database_exists(&path).await.unwrap_or(false) {
            println!("Creating database {}", &path);
            match Sqlite::create_database(&path).await {
                Ok(_) => println!("Create db success"),
                Err(error) => panic!("error: {}", error),
            }
        } else {
            println!("Database already exists");
        }

        let pool = sqlx::SqlitePool::connect(path.as_str()).await?;
        tracing::warn!("DB opened");
        let db = Self { pool };
        db.create_tables(false).await?;
        Ok(db)
    }

    pub async fn create_tables(&self, force_delete: bool) -> Result<()> {
        if force_delete {
            tracing::warn!("Force deleting tables");
            sqlx::query!("DROP TABLE IF EXISTS wind")
                .execute(&self.pool)
                .await?;
        }
        tracing::warn!("Creating tables");
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS wind (
            ts    REAL PRIMARY KEY,
            vel  REAL,
            direction  INTEGER
        )"
        )
        .execute(&self.pool)
        .await?;

        tracing::warn!("Tables created");
        Ok(())
    }

    pub async fn insert_measurement(&self, vel: f64, direction: u16) -> Result<()> {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?;
        self.insert_measurement_at_t(ts, vel, direction).await
    }

    pub async fn insert_measurement_at_t(
        &self,
        ts_since_epoch: Duration,
        vel: f64,
        direction: u16,
    ) -> Result<()> {
        let ts = ts_since_epoch.as_secs_f64();
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

    pub async fn data_since(&self, duration: Duration) -> Result<Vec<Measurement>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let threshold = now.as_secs_f64() - duration.as_secs_f64();
        let res = sqlx::query!(
            "SELECT ts, vel, direction FROM wind WHERE ts > ?1",
            threshold
        )
        .fetch_all(&self.pool)
        .await?;

        let mut measurements = Vec::new();
        for row in res {
            let mesurement = Measurement {
                ts: UNIX_EPOCH
                    + Duration::from_secs_f64(row.ts.ok_or_else(|| anyhow::anyhow!("Not found"))?),
                vel: row.vel.ok_or_else(|| anyhow::anyhow!("Not found"))?,
                direction: row
                    .direction
                    .ok_or_else(|| anyhow::anyhow!("Not found"))?
                    .try_into()?,
            };
            measurements.push(mesurement);
        }
        dbg!(&measurements);
        Ok(measurements)
    }

    pub async fn last_data(&self) -> Result<Measurement> {
        let row = sqlx::query!("SELECT ts, vel, direction  FROM wind ORDER BY ts DESC LIMIT 1",)
            .fetch_one(&self.pool)
            .await?;
        dbg!(&row);
        Ok(Measurement {
            ts: UNIX_EPOCH
                + Duration::from_secs_f64(row.ts.ok_or_else(|| anyhow::anyhow!("Not found"))?),
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

    use std::sync::Once;

    use anyhow::Result;

    use super::*;

    static START: Once = Once::new();

    fn init_tracing() {
        START.call_once(|| {
            tracing_subscriber::fmt::init();
        });
    }

    #[tokio::test]
    async fn test_db() -> Result<()> {
        init_tracing();
        let db = DB::connect("./test_db.sqlite".to_string()).await?;
        db.create_tables(true).await?;
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)? - Duration::from_secs(10);
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
