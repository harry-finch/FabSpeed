//! Import d'une bibliothèque d'outils FabCAM (`*.ivac-toolset.json`).
//!
//! Le format FabCAM ne décrit ni le sens d'hélice ni la denture : ils sont
//! déduits du nom (`upcut`, `downcut`, `diamant`…) et signalés dans les notes
//! comme « déduits à l'import ». Les vitesses FabCAM (`speed`, `feedRate`,
//! `plungeRate`) ne deviennent **pas** des préréglages : ce sont des valeurs
//! initiales d'outil, conservées pour mémoire.

use crate::i18n::Lang;
use crate::model::{Helix, Identity, ToolFamily, ToolMaterial, ToolSpec};
use crate::tr;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("JSON invalide : {0}")]
    Json(#[from] serde_json::Error),
    #[error("Ce fichier n'est pas une bibliothèque d'outils FabCAM (kind = {0})")]
    NotAToolset(String),
}

impl ImportError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            ImportError::Json(e) => tr!(lang, "JSON invalide : {e}", "Invalid JSON: {e}"),
            ImportError::NotAToolset(k) => tr!(
                lang,
                "Ce fichier n'est pas une bibliothèque d'outils FabCAM (kind = {k})",
                "This file is not a FabCAM tool library (kind = {k})"
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IvacToolset {
    kind: String,
    #[serde(default)]
    payload: Vec<IvacTool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IvacTool {
    name: String,
    kind: String,
    diameter: f64,
    #[serde(default)]
    flutes: u32,
    #[serde(default)]
    speed: Option<f64>,
    #[serde(default)]
    feed_rate: Option<f64>,
    #[serde(default)]
    plunge_rate: Option<f64>,
    #[serde(default)]
    flute_length_mm: Option<f64>,
    #[serde(default)]
    shank_diameter_mm: Option<f64>,
    #[serde(default)]
    length_mm: Option<f64>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    fs_identity: Option<Identity>,
    #[serde(default)]
    fs_local_identity: Option<Identity>,
    #[serde(default)]
    angle: Option<f64>,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Convertit une bibliothèque FabCAM en fraises FabSpeed.
/// Les notes ajoutées à l'import sont rédigées dans `lang`.
pub fn import_ivac_toolset(json: &str, lang: Lang) -> Result<Vec<ToolSpec>, ImportError> {
    let set: IvacToolset = serde_json::from_str(json)?;
    if set.kind != "toolset" {
        return Err(ImportError::NotAToolset(set.kind));
    }
    Ok(set.payload.into_iter().map(|t| convert(t, lang)).collect())
}

fn convert(t: IvacTool, lang: Lang) -> ToolSpec {
    let name_lc = t.name.to_lowercase();
    let mut notes = vec![tr!(
        lang,
        "Importée depuis FabCAM : sens d'hélice et famille déduits du nom, à vérifier.",
        "Imported from FabCAM: helix and family inferred from the name, to be checked."
    )];
    let family = match t.kind.as_str() {
        "ball_nose" | "ballnose" => ToolFamily::BallNose,
        "v_bit" | "vbit" | "engraver" | "chamfer" => ToolFamily::VBit,
        _ if name_lc.contains("diamant") || name_lc.contains("diamond") => ToolFamily::DiamondCut,
        _ if name_lc.contains("compression") => ToolFamily::Compression,
        _ => ToolFamily::FlatEndmill,
    };
    let helix = if name_lc.contains("downcut") {
        Helix::Downcut
    } else if name_lc.contains("compression") {
        Helix::Compression
    } else if name_lc.contains("upcut") {
        Helix::Upcut
    } else {
        Helix::Unknown
    };
    let flutes = u8::try_from(t.flutes).ok().filter(|z| *z > 0);
    if let (Some(s), Some(f)) = (t.speed, t.feed_rate) {
        let p = t.plunge_rate.unwrap_or(0.0);
        notes.push(tr!(
            lang,
            "Valeurs initiales FabCAM (non utilisées comme préréglage) : {s:.0} tr/min, {f:.0} mm/min, plongée {p:.0} mm/min.",
            "FabCAM initial values (not used as a preset): {s:.0} rpm, {f:.0} mm/min, plunge {p:.0} mm/min."
        ));
    }
    if let Some(c) = t.comment.filter(|c| !c.trim().is_empty()) {
        notes.push(c);
    }
    let reference = t
        .name
        .split('·')
        .next()
        .unwrap_or(&t.name)
        .trim()
        .to_string();
    let id = t
        .fs_identity
        .as_ref()
        .map(|i| i.id.clone())
        .unwrap_or_else(|| format!("import/{}", slug(&reference)));
    ToolSpec {
        id,
        name: t.name.clone(),
        reference,
        supplier: String::new(),
        url: None,
        family,
        helix,
        tool_material: ToolMaterial::Carbide,
        diameter_mm: t.diameter,
        flutes,
        flute_length_mm: t.flute_length_mm.unwrap_or(t.diameter * 3.0),
        shank_mm: t.shank_diameter_mm,
        overall_length_mm: t.length_mm,
        tip_angle_deg: t.angle,
        upcut_length_mm: None,
        max_rpm: None,
        supplier_rpm_min: None,
        supplier_rpm_recommended: None,
        recommended_materials: vec![],
        excluded_materials: vec![],
        notes,
        fabcam_identity: t.fs_identity.or(t.fs_local_identity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_TOOLSET: &str =
        include_str!("../../../docs/examples/fabcam-toolset.example.json");

    #[test]
    fn imports_a_fabcam_toolset() {
        let tools = import_ivac_toolset(EXAMPLE_TOOLSET, Lang::Fr).unwrap();
        assert_eq!(tools.len(), 4);
        let up = tools.iter().find(|t| t.id == "example/up600").unwrap();
        assert_eq!(up.helix, Helix::Upcut);
        assert_eq!(up.flutes, Some(1));
        assert_eq!(up.flute_length_mm, 25.0);
        let down = tools.iter().find(|t| t.id == "example/down600").unwrap();
        assert_eq!(down.helix, Helix::Downcut);
        let diamond = tools.iter().find(|t| t.name.contains("DIA300")).unwrap();
        assert_eq!(diamond.family, ToolFamily::DiamondCut);
        assert_eq!(diamond.flutes, None, "0 dent = non renseigné");
        let ball = tools.iter().find(|t| t.name.contains("BALL150")).unwrap();
        assert_eq!(ball.family, ToolFamily::BallNose);
        assert!(ball.id.starts_with("import/"));
    }

    #[test]
    fn rejects_other_kinds() {
        let err = import_ivac_toolset(r#"{"kind":"machine","payload":[]}"#, Lang::Fr).unwrap_err();
        assert!(matches!(err, ImportError::NotAToolset(_)));
    }
}
