//! CAL-01 — Calibrage du modèle théorique contre les réglages historiques du
//! wiki du lab (archive privée : docs/research/fablab-wiki et
//! data/tools/cncfraises-fablab.toml, hors dépôt public). Sans cette archive
//! sur le poste, les tests de calibrage sont sautés (et le disent).
//!
//! Exigence produit : « paramètres conservateurs ». Pour chaque réglage testé
//! du wiki (mode Normal, ShopBot), le modèle seul (`use_presets = false`) doit :
//! - ne pas dépasser l'avance ni la profondeur du wiki ;
//! - rester utile : au moins `MIN_FEED_RATIO` de l'avance du wiki.
//!
//! `cargo test -p fabspeed-core --test calibration -- --nocapture` affiche le tableau.

mod common;

use common::*;
use fabspeed_core::{recommend, Aggressiveness, OperationKind};

const MIN_FEED_RATIO: f64 = 0.35;

#[test]
fn cal_01_model_stays_below_lab_wiki_settings() {
    let Some(catalog) = private_lab_catalog() else {
        eprintln!("archive privée du lab absente : calibrage sauté");
        return;
    };
    let mut failures = Vec::new();
    println!(
        "{:<22} {:<18} | {:>6} {:>6} {:>5} | {:>6} {:>6} {:>5} | {:>5} {:>5}",
        "matière", "fraise", "N wiki", "F wiki", "ap", "N mod", "F mod", "ap", "F %", "ap %"
    );
    for p in catalog
        .presets
        .iter()
        .filter(|p| p.aggressiveness == Aggressiveness::Normal)
    {
        let mut r = req(&p.material, &p.tool, OperationKind::Profile);
        r.use_presets = false;
        let reco = recommend(&catalog, &r).unwrap();
        let v = reco.values.expect("valeurs");
        let feed_ratio = v.feed_mm_min / p.feed_mm_min;
        let doc_ratio = v.doc_max_mm / p.doc_mm;
        println!(
            "{:<22} {:<18} | {:>6.0} {:>6.0} {:>5.2} | {:>6.0} {:>6.0} {:>5.2} | {:>4.0}% {:>4.0}%",
            reco.context.material_name,
            p.tool.trim_start_matches("cncfraises/"),
            p.rpm,
            p.feed_mm_min,
            p.doc_mm,
            v.rpm,
            v.feed_mm_min,
            v.doc_max_mm,
            feed_ratio * 100.0,
            doc_ratio * 100.0
        );
        if feed_ratio > 1.0 + 1e-9 || doc_ratio > 1.0 + 1e-9 {
            failures.push(format!(
                "{} : au-dessus du wiki (avance {feed_ratio:.2}, passe {doc_ratio:.2})",
                p.id
            ));
        }
        if feed_ratio < MIN_FEED_RATIO {
            failures.push(format!(
                "{} : avance trop faible ({feed_ratio:.2} du wiki)",
                p.id
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn cal_02_lab_presets_sit_in_the_healthy_chip_zone() {
    // Les réglages testés du lab ne doivent pas être signalés comme frottant
    // ou trop épais : sinon la zone de copeau est mal calée.
    let Some(catalog) = private_lab_catalog() else {
        eprintln!("archive privée du lab absente : calibrage sauté");
        return;
    };
    for p in &catalog.presets {
        let reco = recommend(&catalog, &req(&p.material, &p.tool, OperationKind::Profile)).unwrap();
        let m = reco.metrics.expect("métriques");
        assert_eq!(
            m.chip_zone.state,
            fabspeed_core::ChipState::Ok,
            "{} : copeau {:.3} hors zone [{:.3} ; {:.3}]",
            p.id,
            m.chip_thickness_mm,
            m.chip_zone.min_mm,
            m.chip_zone.max_mm
        );
        assert!(
            m.loads.max() <= 100.0,
            "{} : charge {:.0} %",
            p.id,
            m.loads.max()
        );
    }
}
