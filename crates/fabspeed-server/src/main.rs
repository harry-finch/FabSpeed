//! Serveur FabSpeed : API REST pour FabCAM / scripts et interface web.
//!
//! ```text
//! fabspeed                         # http://127.0.0.1:3030, données ./data
//! fabspeed --host 0.0.0.0          # accessible depuis le réseau du lab
//! fabspeed --print-default-policy  # politique par défaut (base de data/policy.toml)
//! fabspeed --reset-admin-password  # mot de passe administrateur oublié
//! ```

use clap::Parser;
use fabspeed_server::{auth, AppState};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "fabspeed",
    version,
    about = "Calculateur Feeds & Speeds pour fraiseuses CNC"
)]
struct Args {
    /// Dossier des données (machines, matières, fraises, préréglages).
    #[arg(long, env = "FABSPEED_DATA", default_value = "data")]
    data: PathBuf,
    /// Destination proposée du catalogue FabApps (publication désactivée par défaut).
    #[arg(long, env = "FABSPEED_EXCHANGE_FILE")]
    exchange_file: Option<PathBuf>,
    /// Adresse d'écoute. 0.0.0.0 pour ouvrir au réseau local.
    #[arg(long, env = "FABSPEED_HOST", default_value = "127.0.0.1")]
    host: String,
    #[arg(long, env = "FABSPEED_PORT", default_value_t = 3030)]
    port: u16,
    /// Affiche la politique par défaut du moteur au format TOML et quitte.
    #[arg(long)]
    print_default_policy: bool,
    /// Supprime le mot de passe administrateur de ce poste (oubli) et quitte ;
    /// un nouveau se définit ensuite depuis l'interface.
    #[arg(long)]
    reset_admin_password: bool,
    /// Diagnostic d'identité en lecture seule (aucun contenu privé affiché).
    #[arg(long)]
    check_identities: bool,
    /// Nouvelle identité de source pour une copie indépendante ; application arrêtée.
    #[arg(long, conflicts_with_all = ["check_identities", "reset_admin_password", "print_default_policy"])]
    initialize_independent_catalogue_source: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    if args.print_default_policy {
        print!("{}", fabspeed_core::toml_data::default_policy_toml());
        return;
    }
    if args.check_identities {
        match fabspeed_server::identity::diagnostics(&args.data) {
            Ok(messages) => {
                for message in messages {
                    println!("{message}");
                }
                println!("Identity inventory complete (not proof of historical equivalence)");
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.initialize_independent_catalogue_source {
        match fabspeed_server::source::SourceIdentity::initialize_independent_clone(&args.data) {
            Ok(_) => println!("Independent catalogue source initialized; no publication enabled"),
            Err(_) => {
                eprintln!("Catalogue source initialization refused");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.reset_admin_password {
        let _lease =
            fabspeed_server::identity::DataLease::acquire(&args.data).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1)
            });
        let file = args.data.join(auth::AUTH_FILE);
        match std::fs::remove_file(&file) {
            Ok(()) => println!(
                "Mot de passe administrateur supprimé ({}). Redémarrez FabSpeed et définissez-en un nouveau.",
                file.display()
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("Aucun mot de passe administrateur défini ({}).", file.display())
            }
            Err(e) => {
                eprintln!("Impossible de supprimer {} : {e}", file.display());
                std::process::exit(1);
            }
        }
        return;
    }

    let state = match AppState::from_dir(args.data.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Impossible de charger les données depuis « {} » :\n{e}",
                args.data.display()
            );
            std::process::exit(1);
        }
    };
    for issue in state.issues().await {
        tracing::warn!("données : {}", issue.message);
    }
    let catalog = state.catalog(fabspeed_core::Lang::Fr).await;
    tracing::info!(
        "données « {} » : {} machines, {} matières, {} fraises, {} préréglages",
        catalog.meta.revision,
        catalog.machines.len(),
        catalog.materials.len(),
        catalog.tools.len(),
        catalog.presets.len()
    );

    if let Some(path) = args.exchange_file {
        state.set_exchange_hint(path);
    }
    let worker = state.start_publisher();
    let app = fabspeed_server::app(state);

    let addr: SocketAddr = match format!("{}:{}", args.host, args.port).parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Adresse invalide : {e}");
            std::process::exit(2);
        }
    };
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Impossible d'écouter sur {addr} : {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(
        "FabSpeed {} prêt sur http://{addr}",
        fabspeed_core::ENGINE_VERSION
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("serveur HTTP");
    worker.abort();
}
