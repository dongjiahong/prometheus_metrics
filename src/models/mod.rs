use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

pub async fn connect_mysql(
    database_url: &str,
    show_log: bool,
    sqlx_level: tracing::log::LevelFilter,
) -> DatabaseConnection {
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(10))
        .max_lifetime(Duration::from_secs(10))
        .sqlx_logging(show_log)
        .sqlx_logging_level(sqlx_level);

    Database::connect(opt).await.unwrap()
}
