use iptv_peru_api::{config, database};

#[tokio::main]
async fn main() {
    config::load_dotenv();
    let url = config::database_url();
    let pool = database::connect(&url).await;
    database::run_migrations(&pool)
        .await
        .expect("run_migrations");
    let inserted = database::seed_if_empty(&pool).await.expect("seed_if_empty");
    let updated = database::update_streams(&pool)
        .await
        .expect("update_streams");
    let channels: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
        .fetch_one(&pool)
        .await
        .expect("count channels");
    let iptv: i64 = sqlx::query_scalar("SELECT count(*) FROM iptv_channels")
        .fetch_one(&pool)
        .await
        .expect("count iptv");
    println!(
        "seed ok: channels={channels} (insertados={inserted}, actualizados={updated}) iptv={iptv}"
    );
}
