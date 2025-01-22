mod errors;
pub use errors::*;

use by_types::DatabaseConfig;
use slog::o;
use sqlx::{postgres::PgPoolOptions, sqlite::SqlitePoolOptions, Pool, Postgres, Sqlite};

pub async fn init(log: &slog::Logger, conf: &DatabaseConfig) -> Result<(), DatabaseError> {
    match conf {
        DatabaseConfig::DynamoDb { aws, table_name } => {
            easy_dynamodb::init(
                log.new(o!("module" => "by-db")),
                aws.access_key_id.to_string(),
                aws.secret_access_key.to_string(),
                aws.region.to_string(),
                table_name.to_string(),
                "id".to_string(),
                None,
                None,
            );
        }
        DatabaseConfig::Postgres { url } => {
            let pool: Pool<Postgres> = PgPoolOptions::new().max_connections(5).connect(url).await?;
        }
        DatabaseConfig::Sqlite { url } => {
            let pool: Pool<Sqlite> = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(url)
                .await?;
        }
    }

    Ok(())
}
