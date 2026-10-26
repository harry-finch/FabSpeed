//! # fabspeed-core
//!
//! Moteur **pur et déterministe** de recommandation Feeds & Speeds pour
//! fraiseuses CNC (routeurs à portique, fraiseuses de bureau…).
//!
//! - Aucune IO, aucune dépendance réseau ou UI : utilisable par le serveur
//!   FabSpeed, par FabCAM en import direct, ou compilé en WebAssembly.
//! - Les constantes empiriques vivent dans les données (`Catalog`) et la
//!   politique (`EnginePolicy`), jamais dans l'algorithme.
//! - Une recommandation expose sa provenance, ses limites actives et la trace
//!   de son calcul.
//!
//! ```no_run
//! use fabspeed_core::{recommend, Catalog, RecommendationRequest};
//! # fn demo(catalog: &Catalog, req: &RecommendationRequest) {
//! let reco = recommend(catalog, req).expect("contexte valide");
//! println!("{:?}", reco.values);
//! # }
//! ```

pub mod advisor;
pub mod catalog;
pub mod engine;
pub mod export;
pub mod i18n;
pub mod import;
mod levers;
pub mod localize;
pub mod model;
pub mod policy;
pub mod recommendation;
pub mod request;
#[cfg(feature = "toml")]
pub mod toml_data;
pub mod trials;

pub use advisor::{diagnose, Diagnosis, Symptom};
pub use catalog::{Catalog, CatalogIssue, Layers};
pub use engine::{recommend, EngineError, ENGINE_VERSION};
pub use i18n::Lang;
pub use model::*;
pub use policy::EnginePolicy;
pub use recommendation::*;
pub use request::{Overrides, RecommendationRequest, RequestPatch};
pub use trials::{ToolCondition, Trial, TrialInput, TrialOutcome, Validation};
