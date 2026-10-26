//! Allow-listed pure FabApps projection. No endpoint, publisher or filesystem IO.
use crate::{canonical::digest, source::SourceIdentity};
use fabspeed_core::model::{Identity, MachineProfile, MaterialProfile, ToolSpec};
use fabspeed_core::{Catalog, Trial, TrialOutcome};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

pub type Result<T> = std::result::Result<T, &'static str>;
/// Approval covers every documented shared field, including name/supplier/reference.
/// Stable IDs only. Defaults disclose nothing; preset dependencies never auto-select.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Consent {
    pub enabled: bool,
    pub machines: BTreeSet<String>,
    pub tools: BTreeSet<String>,
    pub materials: BTreeSet<String>,
    /// Local trial IDs, never names or labels.
    pub trials: BTreeSet<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct Resource(Value);
impl Resource {
    pub fn id(&self) -> &str {
        self.0["id"].as_str().expect("constructed ID")
    }
    pub fn revision(&self) -> &str {
        self.0["revision"].as_str().expect("constructed revision")
    }
    pub fn value(&self) -> &Value {
        &self.0
    }
    fn finish(domain: &str, mut value: Value) -> Result<Self> {
        if value["id"].as_str().is_none_or(str::is_empty)
            || value["name"].as_str().is_none_or(str::is_empty)
        {
            return Err("missing shared identity or name");
        }
        value["revision"] = json!(digest(domain, &value)?);
        Ok(Self(value))
    }
    fn identity(&self) -> Identity {
        Identity {
            id: self.id().into(),
            revision: self.revision().into(),
        }
    }
}
fn positive(v: f64) -> Result<Value> {
    if !v.is_finite() || v <= 0.0 {
        return Err("invalid positive shared number");
    }
    Ok(json!(v))
}
fn optional(m: &mut Map<String, Value>, key: &str, v: Option<f64>) -> Result<()> {
    if let Some(v) = v {
        m.insert(key.into(), positive(v)?);
    }
    Ok(())
}
pub fn machine(m: &MachineProfile) -> Result<Resource> {
    let mut limits = Map::new();
    for (k, v) in [
        ("rpm_min", Some(m.rpm_min)),
        ("rpm_max", Some(m.rpm_max)),
        ("spindle_power_w", Some(m.spindle_power_w)),
        ("spindle_rated_rpm", m.spindle_rated_rpm),
        ("max_feed_mm_min", Some(m.max_feed_mm_min)),
        ("max_plunge_mm_min", m.max_plunge_mm_min),
    ] {
        if let Some(v) = v {
            limits.insert(k.into(),json!({"value":positive(v)?,"status":m.sources.get(k).map(|s|json!(s.status)).unwrap_or(json!("unknown"))}));
        }
    }
    if m.rpm_min > m.rpm_max {
        return Err("inverted shared range");
    }
    if let Some(area) = m.work_area_mm {
        let values: Vec<Value> = area.into_iter().map(positive).collect::<Result<_>>()?;
        limits.insert("work_area_mm".into(),json!({"value":values,"status":m.sources.get("work_area_mm").map(|s|json!(s.status)).unwrap_or(json!("unknown"))}));
    }
    Resource::finish(
        "machine",
        json!({"id":m.id,"name":m.name,"kind":"machine_profile","limits":limits}),
    )
}
pub fn tool(t: &ToolSpec) -> Result<Resource> {
    let mut v = json!({"id":t.id,"name":t.name,"kind":"tool_spec","family":t.family,"helix":t.helix,"tool_material":t.tool_material,"diameter_mm":positive(t.diameter_mm)?,"flute_length_mm":positive(t.flute_length_mm)?});
    let m = v.as_object_mut().unwrap();
    for (k, n) in [
        ("shank_mm", t.shank_mm),
        ("overall_length_mm", t.overall_length_mm),
        ("tip_angle_deg", t.tip_angle_deg),
        ("upcut_length_mm", t.upcut_length_mm),
        ("max_rpm", t.max_rpm),
        ("supplier_rpm_min", t.supplier_rpm_min),
        ("supplier_rpm_recommended", t.supplier_rpm_recommended),
    ] {
        optional(m, k, n)?;
    }
    if let Some(f) = t.flutes {
        if f == 0 {
            return Err("invalid shared flute count");
        }
        m.insert("flutes".into(), json!(f));
    }
    if !t.supplier.is_empty() {
        m.insert("supplier".into(), json!(t.supplier));
    }
    if !t.reference.is_empty() {
        m.insert("reference".into(), json!(t.reference));
    }
    if t.tip_angle_deg.is_some_and(|v| v >= 180.0)
        || t.overall_length_mm
            .is_some_and(|v| v < t.flute_length_mm || t.upcut_length_mm.is_some_and(|u| u > v))
        || t.upcut_length_mm.is_some_and(|u| u > t.flute_length_mm)
        || t.max_rpm.is_some_and(|max| {
            t.supplier_rpm_min.is_some_and(|min| min > max)
                || t.supplier_rpm_recommended.is_some_and(|r| r > max)
        })
        || t.supplier_rpm_min
            .zip(t.supplier_rpm_recommended)
            .is_some_and(|(min, r)| min > r)
    {
        return Err("inconsistent shared tool geometry or range");
    }
    Resource::finish("tool", v)
}
pub fn material(m: &MaterialProfile) -> Result<Resource> {
    Resource::finish(
        "material",
        json!({"id":m.id,"name":m.name,"kind":"material_profile","category":m.category,"supervision_required":m.supervision_required}),
    )
}
fn timestamp(s: &str) -> Result<()> {
    if s.as_bytes().get(10) != Some(&b'T')
        || s.as_bytes().get(17).is_none_or(|b| *b > b'5')
        || !s.ends_with('Z')
        || time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).is_err()
    {
        return Err("invalid shared timestamp");
    }
    Ok(())
}
fn tested(t: &Trial) -> Result<Value> {
    timestamp(&t.recorded_at)?;
    let mut context = json!({"operation":t.operation,"aggressiveness":t.aggressiveness,"workholding":t.workholding,"stepover_ratio":positive(t.stepover_ratio)?});
    if t.stepover_ratio > 1.0 {
        return Err("invalid shared stepover");
    }
    let m = context.as_object_mut().unwrap();
    for (k, v) in [
        ("thickness_mm", t.thickness_mm),
        ("depth_mm", t.depth_mm),
        ("stickout_mm", t.stickout_mm),
    ] {
        optional(m, k, v)?;
    }
    if let Some(c) = t.tool_condition {
        m.insert("tool_condition".into(), json!(c));
    }
    Ok(
        json!({"context":context,"values":{"rpm":positive(t.rpm)?,"feed_mm_min":positive(t.feed_mm_min)?,"plunge_mm_min":positive(t.plunge_mm_min)?,"doc_mm":positive(t.doc_mm)?}}),
    )
}
/// Exact lookups deliberately do not use catalogue aliases.
pub fn capture(
    c: &Catalog,
    t: &Trial,
    source: &SourceIdentity,
) -> Result<fabspeed_core::trials::ValidationBinding> {
    source.check().map_err(|_| "invalid source")?;
    if t.outcome != TrialOutcome::Good {
        return Err("trial is not good");
    }
    let m = c
        .machines
        .iter()
        .find(|m| m.id == t.machine_id)
        .ok_or("missing binding dependency")?;
    let tool_spec = c
        .tools
        .iter()
        .find(|m| m.id == t.tool_id)
        .ok_or("missing binding dependency")?;
    let mat = c
        .materials
        .iter()
        .find(|m| m.id == t.material_id)
        .ok_or("missing binding dependency")?;
    Ok(fabspeed_core::trials::ValidationBinding {
        version: 1,
        catalogue_id: source.catalogue_id.clone(),
        trial_id: t.id.clone(),
        machine: machine(m)?.identity(),
        tool: tool(tool_spec)?.identity(),
        material: material(mat)?.identity(),
        context_values_revision: digest(
            "trial-context-values",
            &json!({"trial_id":t.id,"recorded_at":t.recorded_at,"tested":tested(t)?}),
        )?,
    })
}
#[derive(Debug, Clone, Serialize)]
pub struct Projection {
    #[serde(skip)]
    previous_generation: u64,
    format: &'static str,
    version: u32,
    catalogue_id: String,
    publisher: Value,
    complete: bool,
    language: &'static str,
    pub(crate) machines: Vec<Resource>,
    pub(crate) tools: Vec<Resource>,
    pub(crate) materials: Vec<Resource>,
    pub(crate) presets: Vec<Resource>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exclusion {
    NotValidated,
    Unbound,
    DependencyNotApproved,
    DependencyMissing,
    BindingChanged,
}
fn selected<T>(
    items: &[T],
    ids: &BTreeSet<String>,
    id: impl Fn(&T) -> &str,
    project: impl Fn(&T) -> Result<Resource>,
) -> Result<Vec<Resource>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if !seen.insert(id(item)) {
            return Err("duplicate resource identity");
        }
        if ids.contains(id(item)) {
            out.push(project(item)?);
        }
    }
    out.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(out)
}
/// Input is the authoritative unlocalized catalogue, never a UI language copy.
pub fn project(
    c: &Catalog,
    source: &SourceIdentity,
    consent: &Consent,
) -> Result<(Projection, Vec<Exclusion>)> {
    source.check().map_err(|_| "invalid source")?;
    if c.lang != fabspeed_core::Lang::Fr {
        return Err("projection requires authoritative language");
    }
    let mut p = Projection {
        previous_generation: source.generation,
        format: "fabapps-catalogue",
        version: 1,
        catalogue_id: source.catalogue_id.clone(),
        publisher: json!({"application":"fabspeed","application_version":fabspeed_core::ENGINE_VERSION}),
        complete: true,
        language: "fr",
        machines: vec![],
        tools: vec![],
        materials: vec![],
        presets: vec![],
    };
    let mut reasons = vec![];
    if !consent.enabled {
        return Ok((p, reasons));
    }
    p.machines = selected(&c.machines, &consent.machines, |m| &m.id, machine)?;
    p.tools = selected(&c.tools, &consent.tools, |m| &m.id, tool)?;
    p.materials = selected(&c.materials, &consent.materials, |m| &m.id, material)?;
    let mut seen = BTreeSet::new();
    for t in &c.trials {
        if !seen.insert(&t.id) {
            return Err("duplicate trial identity");
        }
        if !consent.trials.contains(&t.id) {
            continue;
        }
        let Some(v) = &t.validation else {
            reasons.push(Exclusion::NotValidated);
            continue;
        };
        if t.outcome != TrialOutcome::Good {
            reasons.push(Exclusion::NotValidated);
            continue;
        }
        let Some(binding) = &v.binding else {
            reasons.push(Exclusion::Unbound);
            continue;
        };
        if !consent.machines.contains(&t.machine_id)
            || !consent.tools.contains(&t.tool_id)
            || !consent.materials.contains(&t.material_id)
        {
            reasons.push(Exclusion::DependencyNotApproved);
            continue;
        }
        if !p.machines.iter().any(|r| r.id() == t.machine_id)
            || !p.tools.iter().any(|r| r.id() == t.tool_id)
            || !p.materials.iter().any(|r| r.id() == t.material_id)
        {
            reasons.push(Exclusion::DependencyMissing);
            continue;
        }
        if &capture(c, t, source)? != binding {
            reasons.push(Exclusion::BindingChanged);
            continue;
        }
        timestamp(&v.validated_at)?;
        let data = tested(t)?;
        let opaque = digest("trial-reference", &json!(t.id))?;
        let id = format!("trial:{}", opaque.trim_start_matches("sha256:"));
        p.presets.push(Resource::finish("preset",json!({"id":id,"name":format!("Validated preset {}",&opaque[7..19]),"status":"validated","machine":binding.machine,"tool":binding.tool,"material":binding.material,"context":data["context"],"values":data["values"],"validation":{"basis":"admin_confirmed_good_trial","trial_id":opaque,"tested_at":t.recorded_at,"validated_at":v.validated_at,"dependency_binding":"captured_at_validation"}}))?);
    }
    p.presets.sort_by(|a, b| a.id().cmp(b.id()));
    Ok((p, reasons))
}
impl Projection {
    pub fn revision(&self) -> Result<String> {
        digest(
            "catalogue",
            &serde_json::to_value(self).map_err(|_| "projection serialization failed")?,
        )
    }
    /// Synthetic/publication context only; does not reserve or persist a generation.
    pub fn assemble(&self, generation: u64, published_at: &str) -> Result<Value> {
        if generation <= self.previous_generation || generation > 9_007_199_254_740_991 {
            return Err("invalid publication generation");
        }
        timestamp(published_at)?;
        let mut value =
            serde_json::to_value(self).map_err(|_| "projection serialization failed")?;
        value["revision"] = json!(self.revision()?);
        value["generation"] = json!(generation);
        value["published_at"] = json!(published_at);
        Ok(value)
    }
}
