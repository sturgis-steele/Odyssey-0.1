use odyssey_backend::{
    application,
    config::{self, Config},
};
use tokio::net::TcpListener;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load(config::root()?)?;
    tracing_subscriber::fmt()
        .json()
        .with_max_level(cfg.log_level)
        .with_target(false)
        .init();
    let listener = TcpListener::bind((cfg.host.as_str(), cfg.port)).await?;
    let addr = listener.local_addr()?;
    let app = application(cfg).await?;
    tracing::info!(%addr,"backend_ready");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! { _=term.recv()=>(), _=tokio::signal::ctrl_c()=>() }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
