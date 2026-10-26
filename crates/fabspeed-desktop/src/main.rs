//! Application de bureau FabSpeed (macOS, Windows, Linux).
//!
//! Une seule fenêtre, qui affiche l'interface web servie par le serveur
//! FabSpeed embarqué sur un port local. Chaque poste a sa propre base, dans
//! le dossier de données de l'utilisateur (`…/fr.fablab.fabspeed/data/`) :
//! les données livrées y sont installées au premier lancement et mises à
//! jour quand une nouvelle version les change, sans toucher à la base du lab
//! ni au mot de passe (voir `fabspeed_server::install`).

// Pas de console sous Windows pour l'application distribuée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fabspeed_server::install::{install_base, InstallOutcome};
use fabspeed_server::AppState;
use std::net::{Ipv4Addr, TcpListener};
use tauri::path::BaseDirectory;
use tauri::webview::DownloadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Port local préféré : fixe pour que l'origine de la page (et donc la langue,
/// le thème et la configuration mémorisés par le navigateur intégré) reste la
/// même d'un lancement à l'autre. Un autre port est pris s'il est occupé.
const PREFERRED_PORT: u16 = 47830;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            let data_dir = app_dir.join("data");
            let bundled = app.path().resolve("base-data", BaseDirectory::Resource)?;
            match install_base(&bundled, &data_dir)? {
                InstallOutcome::Fresh => {
                    tracing::info!(
                        "premier lancement : données installées dans {}",
                        data_dir.display()
                    )
                }
                InstallOutcome::UpToDate => {}
                InstallOutcome::Updated {
                    from,
                    to,
                    safety_backup,
                } => tracing::info!(
                    "données de base mises à jour ({from} → {to}) ; copie de sécurité : {}",
                    safety_backup.display()
                ),
            }
            let state = AppState::from_dir(data_dir.clone())
                .map_err(|e| format!("Données illisibles dans {} : {e}", data_dir.display()))?;

            state.set_exchange_hint(app_dir.join("exchange").join("fabapps-catalogue-v1.json"));

            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, PREFERRED_PORT))
                .or_else(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)))?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(listener).expect("port local");
                if let Err(e) = fabspeed_server::serve(listener, state).await {
                    tracing::error!("serveur FabSpeed arrêté : {e}");
                }
            });
            tracing::info!(
                "FabSpeed {} sur http://127.0.0.1:{port}",
                fabspeed_core::ENGINE_VERSION
            );

            let downloads = app.path().download_dir().ok();
            let url = format!("http://127.0.0.1:{port}/").parse()?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("FabSpeed")
                .inner_size(1440.0, 900.0)
                .min_inner_size(1024.0, 680.0)
                // Sauvegardes et exports CSV : enregistrés dans Téléchargements.
                .on_download(move |_webview, event| {
                    if let DownloadEvent::Requested { destination, .. } = event {
                        if let (Some(dir), Some(name)) = (&downloads, destination.file_name()) {
                            *destination = dir.join(name);
                        }
                    }
                    true
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("FabSpeed n'a pas pu démarrer");
}
