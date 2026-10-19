use iptv_peru_api::{config, database};

#[tokio::main]
async fn main() {
    config::load_dotenv();
    let url = config::database_url();
    let pool = database::connect(&url).await;
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&pool)
        .await
        .expect("version");
    let short_ver = if version.len() > 70 {
        &version[..70]
    } else {
        &version
    };
    println!("CONECTADO: {short_ver}");
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name"
    )
    .fetch_all(&pool)
    .await
    .expect("tables");
    println!("tablas: {:?}", tables);
}
