use axum::ServiceExt;
use iptv_peru_api::{config, database};

#[tokio::main]
async fn main() {
    config::load_dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let url = config::database_url();
    let pool = database::connect(&url).await;
    database::run_migrations(&pool)
        .await
        .expect("no se pudo preparar las migraciones");
    database::seed_if_empty(&pool)
        .await
        .expect("no se pudo sembrar la base de datos");
    database::update_streams(&pool)
        .await
        .expect("no se pudo actualizar streams");

    let app = iptv_peru_api::build_app(pool);
    let host = config::host();
    let port = config::port();
    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .expect("no se pudo abrir el puerto");
    println!("iptv-peru-api escuchando en http://{host}:{port}");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("servidor");
}
