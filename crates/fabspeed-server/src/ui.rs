//! Interface web embarquée (build Vite de `ui/`).
//!
//! En mode debug, rust-embed lit `ui/dist` sur le disque à chaque requête :
//! `npm run build` suffit, sans recompiler le serveur. En release, les
//! fichiers sont intégrés au binaire.

use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../ui/dist"]
#[allow_missing = true]
struct Assets;

const MISSING_UI: &str = r#"<!doctype html><html lang="fr"><meta charset="utf-8"><title>FabSpeed</title>
<body style="font:14px system-ui;background:#1d1d1d;color:#ddd;padding:32px">
<h1 style="font-size:18px">FabSpeed — interface non construite</h1>
<p>L'API fonctionne (<a style="color:#6ea8ff" href="/api/v1/health">/api/v1/health</a>).
Pour l'interface : <code>cd ui &amp;&amp; npm install &amp;&amp; npm run build</code>.</p></body></html>"#;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "Route d'API inconnue").into_response();
    }
    let candidate = if path.is_empty() { "index.html" } else { path };
    // Un fichier absent (favicon.ico, image…) donne 404 : seule une adresse
    // sans extension retombe sur l'interface. Sinon le navigateur reçoit du
    // HTML à la place d'une icône et garde l'ancienne.
    let is_file = path
        .rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'));
    if is_file && Assets::get(candidate).is_none() {
        return (StatusCode::NOT_FOUND, "Fichier introuvable").into_response();
    }
    match Assets::get(candidate).or_else(|| Assets::get("index.html")) {
        Some(file) => {
            let served = if Assets::get(candidate).is_some() {
                candidate
            } else {
                "index.html"
            };
            let mime = mime_guess::from_path(served).first_or_octet_stream();
            let cache = if served.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (header::CONTENT_TYPE, mime.as_ref()),
                    (header::CACHE_CONTROL, cache),
                ],
                file.data,
            )
                .into_response()
        }
        None => Html(MISSING_UI).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ui_01_missing_files_are_404_routes_fall_back_to_the_app() {
        let status = |p: &'static str| async move { serve(Uri::from_static(p)).await.status() };
        // Une icône absente ne doit pas recevoir la page HTML (le navigateur garderait l'ancienne).
        assert_eq!(status("/absent.ico").await, StatusCode::NOT_FOUND);
        assert_eq!(status("/assets/absent.js").await, StatusCode::NOT_FOUND);
        // Une adresse d'écran (sans extension) retombe sur l'interface.
        assert_eq!(status("/bibliotheque").await, StatusCode::OK);
        assert_eq!(status("/").await, StatusCode::OK);
    }
}
