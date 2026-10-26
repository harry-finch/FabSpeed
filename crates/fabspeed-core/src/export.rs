//! Export des essais en CSV, pour un tableur ou le wiki du lab.
//!
//! Français : séparateur `;` et virgule décimale (ce qu'attend un tableur
//! réglé en français) ; anglais : séparateur `,` et point décimal. Une
//! marque d'ordre des octets (BOM) en tête fait lire l'UTF-8 correctement
//! par les tableurs. Les noms (machine, matière, symptômes) sont ceux du
//! catalogue dans la langue demandée ; les identifiants sont aussi exportés
//! pour retrouver chaque fiche.

use crate::catalog::Catalog;
use crate::engine::mode_label;
use crate::i18n::Lang;
use crate::model::{OperationKind, Workholding};
use crate::tr;
use crate::trials::{ToolCondition, Trial, TrialOutcome};

pub fn operation_label(l: Lang, op: OperationKind) -> String {
    match op {
        OperationKind::Profile => tr!(l, "Détourage", "Profile"),
        OperationKind::Pocket => tr!(l, "Poche", "Pocket"),
        OperationKind::FullSlot => tr!(l, "Rainure", "Slot"),
        OperationKind::Finish => tr!(l, "Finition", "Finishing"),
        OperationKind::Surfacing => tr!(l, "Surfaçage", "Surfacing"),
        OperationKind::Engraving => tr!(l, "Gravure", "Engraving"),
    }
}

fn workholding_label(l: Lang, w: Workholding) -> String {
    match w {
        Workholding::ScrewsClamps => tr!(l, "Vissée ou bridée", "Screwed or clamped"),
        Workholding::Vacuum => tr!(l, "Table aspirante", "Vacuum table"),
        Workholding::Tape => tr!(l, "Adhésif double face", "Double-sided tape"),
        Workholding::Unknown => tr!(l, "Inconnu", "Unknown"),
    }
}

fn outcome_label(l: Lang, o: TrialOutcome) -> String {
    match o {
        TrialOutcome::Good => tr!(l, "Bon", "Good"),
        TrialOutcome::Adjust => tr!(l, "À ajuster", "Needs adjusting"),
        TrialOutcome::Failed => tr!(l, "Échec", "Failed"),
    }
}

/// Champ CSV : entre guillemets si nécessaire, guillemets doublés.
fn field(s: &str, sep: char) -> String {
    if s.contains(sep) || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Tous les essais du catalogue, du plus ancien au plus récent.
pub fn trials_csv(catalog: &Catalog) -> String {
    let l = catalog.lang;
    let sep = match l {
        Lang::Fr => ';',
        Lang::En => ',',
    };
    // Nombres sans séparateur de milliers (lisibles par un tableur).
    let n = |v: f64, dec: usize| {
        let s = format!("{v:.dec$}");
        match l {
            Lang::Fr => s.replace('.', ","),
            Lang::En => s,
        }
    };
    let opt = |v: Option<f64>, dec: usize| v.map(|v| n(v, dec)).unwrap_or_default();
    let header = [
        tr!(l, "Identifiant", "Id"),
        tr!(l, "Date (UTC)", "Date (UTC)"),
        tr!(l, "Opérateur", "Operator"),
        tr!(l, "Machine", "Machine"),
        tr!(l, "Matière", "Material"),
        tr!(l, "Épaisseur (mm)", "Thickness (mm)"),
        tr!(l, "Fraise", "Tool"),
        tr!(l, "Diamètre (mm)", "Diameter (mm)"),
        tr!(l, "Dents", "Flutes"),
        tr!(l, "État de la fraise", "Tool condition"),
        tr!(l, "Opération", "Operation"),
        tr!(l, "Mode", "Mode"),
        tr!(l, "Maintien", "Workholding"),
        tr!(l, "Profondeur (mm)", "Depth (mm)"),
        tr!(l, "Pas latéral (%)", "Stepover (%)"),
        tr!(l, "Rotation (tr/min)", "Spindle speed (rpm)"),
        tr!(l, "Avance (mm/min)", "Feed (mm/min)"),
        tr!(l, "Plongée (mm/min)", "Plunge (mm/min)"),
        tr!(l, "Passe (mm)", "Depth of cut (mm)"),
        tr!(l, "Résultat", "Result"),
        tr!(l, "Constats", "Observations"),
        tr!(l, "Remarques", "Notes"),
        tr!(l, "Validé le", "Validated on"),
        tr!(l, "Validé par", "Validated by"),
        tr!(l, "Critères vérifiés", "Criteria checked"),
        tr!(l, "Identifiant machine", "Machine id"),
        tr!(l, "Identifiant matière", "Material id"),
        tr!(l, "Identifiant fraise", "Tool id"),
        tr!(l, "Moteur", "Engine"),
        tr!(l, "Données", "Data"),
    ];
    let mut trials: Vec<&Trial> = catalog.trials.iter().collect();
    trials.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at).then(a.id.cmp(&b.id)));

    let mut out = String::from('\u{feff}');
    let line = |cells: Vec<String>| {
        cells
            .iter()
            .map(|c| field(c, sep))
            .collect::<Vec<_>>()
            .join(&sep.to_string())
    };
    out.push_str(&line(header.to_vec()));
    out.push_str("\r\n");
    for t in trials {
        let machine = catalog
            .machine(&t.machine_id)
            .map_or(t.machine_id.clone(), |m| m.name.clone());
        let material = catalog
            .material(&t.material_id)
            .map_or(t.material_id.clone(), |m| m.name.clone());
        let symptoms: Vec<String> = t
            .symptoms
            .iter()
            .map(|id| catalog.symptom(id).map_or(id.clone(), |s| s.label.clone()))
            .collect();
        let v = t.validation.as_ref();
        out.push_str(&line(vec![
            t.id.clone(),
            t.recorded_at.clone(),
            t.operator.clone(),
            machine,
            material,
            opt(t.thickness_mm, 2),
            t.tool_name.clone(),
            n(t.tool_diameter_mm, 2),
            t.tool_flutes.map(|f| f.to_string()).unwrap_or_default(),
            t.tool_condition
                .map(|c| match c {
                    ToolCondition::New => tr!(l, "Neuve", "New"),
                    ToolCondition::Used => tr!(l, "Déjà utilisée", "Used"),
                })
                .unwrap_or_default(),
            operation_label(l, t.operation),
            mode_label(l, t.aggressiveness),
            workholding_label(l, t.workholding),
            opt(t.depth_mm, 2),
            n(t.stepover_ratio * 100.0, 0),
            n(t.rpm, 0),
            n(t.feed_mm_min, 0),
            n(t.plunge_mm_min, 0),
            n(t.doc_mm, 2),
            outcome_label(l, t.outcome),
            symptoms.join(", "),
            t.notes.clone(),
            v.map(|v| v.validated_at.clone()).unwrap_or_default(),
            v.map(|v| v.validated_by.clone()).unwrap_or_default(),
            v.map(|v| v.note.clone()).unwrap_or_default(),
            t.machine_id.clone(),
            t.material_id.clone(),
            t.tool_id.clone(),
            t.engine_version.clone(),
            t.dataset_revision.clone(),
        ]));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::field;

    #[test]
    fn exp_01_fields_are_quoted_only_when_needed() {
        assert_eq!(field("Bord net", ';'), "Bord net");
        assert_eq!(field("a;b", ';'), "\"a;b\"");
        assert_eq!(field("a,b", ';'), "a,b");
        assert_eq!(field("dit \"net\"", ','), "\"dit \"\"net\"\"\"");
        assert_eq!(field("ligne 1\nligne 2", ','), "\"ligne 1\nligne 2\"");
    }
}
