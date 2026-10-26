//! Serveur FabSpeed en bibliothèque : utilisé par le binaire `fabspeed`
//! (navigateur, réseau du lab) et par l'application de bureau, qui l'embarque.

pub mod api;
pub mod auth;
pub mod backup;
pub mod canonical;
pub mod catalogue;
pub mod identity;
pub mod install;
mod lab;
mod lab_api;
mod library_transfer;
pub mod loader;
pub mod publication;
mod publication_api;
mod publication_fs;
pub mod source;
pub mod state;
mod ui;

pub use state::AppState;

use axum::Router;
use tower_http::cors::CorsLayer;

/// Application complète : API `/api/v1` et interface web intégrée.
pub fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1", api::router())
        .fallback(ui::serve)
        // FabCAM (Tauri) et les scripts locaux appellent l'API depuis une autre origine.
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Sert l'application sur un port déjà ouvert, jusqu'à l'arrêt du processus.
pub async fn serve(listener: tokio::net::TcpListener, state: AppState) -> std::io::Result<()> {
    let worker = state.start_publisher();
    let result = axum::serve(listener, app(state)).await;
    worker.abort();
    result
}

#[cfg(test)]
mod identity_tests;

#[cfg(test)]
mod catalogue_tests;

#[cfg(all(test, unix))]
mod publication_tests;
