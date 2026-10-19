use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .test_before_acquire(true)
        .connect(url)
        .await
        .expect("no se pudo conectar a la base de datos")
}
