//! Catalogue de test : les données publiques du dépôt (`data/`), plus des
//! réglages de référence fictifs (`tests/fixtures/`). Aucune donnée privée
//! d'un lab n'est nécessaire pour faire passer les tests.

#![allow(dead_code)]

use fabspeed_core::toml_data::DataKind;
use fabspeed_core::{Catalog, OperationKind, RecommendationRequest};

pub const SHOPBOT: &str = "shopbot-prs-standard-96x48-2016";

pub fn lab_catalog() -> Catalog {
    let mut c = Catalog::default();
    let files: [(DataKind, &str, &str); 5] = [
        (
            DataKind::Dataset,
            "dataset.toml",
            include_str!("../../../../data/dataset.toml"),
        ),
        (
            DataKind::Machines,
            "machines.toml",
            include_str!("../../../../data/machines.toml"),
        ),
        (
            DataKind::Materials,
            "materials.toml",
            include_str!("../../../../data/materials.toml"),
        ),
        (
            DataKind::Tools,
            "generic.toml",
            include_str!("../../../../data/tools/generic.toml"),
        ),
        (
            DataKind::Advisor,
            "advisor.toml",
            include_str!("../../../../data/advisor.toml"),
        ),
    ];
    for (kind, name, text) in files {
        c.merge_toml(kind, name, text)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    let fatal: Vec<_> = c.validate().into_iter().filter(|i| i.fatal).collect();
    assert!(fatal.is_empty(), "données invalides : {fatal:?}");
    c
}

/// Données du dépôt + réglages de référence fictifs : tests de précédence.
pub fn catalog_with_presets() -> Catalog {
    let mut c = lab_catalog();
    c.merge_toml(
        DataKind::Presets,
        "presets.toml",
        include_str!("../fixtures/presets.toml"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert!(c.validate().iter().all(|i| !i.fatal));
    c
}

/// Archive privée du lab (fiches fournisseur + réglages historiques du wiki),
/// lue à l'exécution si elle est présente sur le poste : sert au calibrage.
/// `None` dans un clone public.
pub fn private_lab_catalog() -> Option<Catalog> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tools = std::fs::read_to_string(root.join("data/tools/cncfraises-fablab.toml")).ok()?;
    let presets =
        std::fs::read_to_string(root.join("docs/research/fablab-wiki/presets-wiki.toml")).ok()?;
    let mut c = lab_catalog();
    c.merge_toml(DataKind::Tools, "cncfraises-fablab.toml", &tools)
        .unwrap_or_else(|e| panic!("{e}"));
    c.merge_toml(DataKind::Presets, "presets-wiki.toml", &presets)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(c.validate().iter().all(|i| !i.fatal));
    Some(c)
}

pub fn req(material: &str, tool: &str, op: OperationKind) -> RecommendationRequest {
    RecommendationRequest {
        machine_id: SHOPBOT.into(),
        material_id: material.into(),
        tool_id: Some(tool.into()),
        tool: None,
        operation: op,
        stepover_ratio: None,
        depth_mm: None,
        aggressiveness: Default::default(),
        workholding: Default::default(),
        stickout_mm: None,
        max_doc_mm: None,
        cut_length_mm: None,
        use_presets: true,
        supervised_speedups: false,
        overrides: Default::default(),
    }
}
