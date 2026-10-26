//! Leviers « Aller plus vite », classés du plus sûr au plus agressif.
//!
//! 1. Moins de passes, plus profondes (le copeau ne change pas).
//! 2. Rotation et avance augmentées ensemble (même copeau, même effort par dent).
//! 3. Fraise rentrée dans la pince / meilleur maintien (lèvent une limite).
//! 4. Copeau plus épais (avance seule).
//! 5. Mode plus rapide.
//!
//! Chaque levier porte un `RequestPatch` : rien n'est appliqué sans action
//! explicite de l'utilisateur.

use crate::engine::{mode_label, Ctx};
use crate::model::{Aggressiveness, Workholding};
use crate::recommendation::{ChipState, CuttingValues, Lever, LimitingFactor, Metrics, SourceKind};
use crate::request::{Overrides, RequestPatch};
use crate::tr;

fn floor_step(v: f64, step: f64) -> f64 {
    ((v / step) + 1e-9).floor() * step
}

pub(crate) fn compute(
    ctx: &Ctx,
    v: &CuttingValues,
    m: &Metrics,
    limits: &[LimitingFactor],
    source: SourceKind,
) -> Vec<Lever> {
    let l = ctx.lang;
    let p = ctx.policy;
    let lp = &p.levers;
    let rnd = &p.rounding;
    let mut out = Vec::new();
    let max_load = m.loads.max().max(1.0);
    let load = l.num(max_load, 0);
    let beyond_lab = (source == SourceKind::LabPreset).then(|| {
        tr!(
            l,
            "Au-delà des réglages testés au lab : progressez par paliers et écoutez la machine.",
            "Beyond the settings tested in the lab: go step by step and listen to the machine."
        )
    });

    // Métaux : usinage supervisé, aucune accélération proposée, sauf demande
    // explicite du fabmanager (`supervised_speedups`). Les leviers qui
    // améliorent la sécurité (sortie de pince, maintien) restent toujours.
    let speedups = !ctx.material.supervision_required || ctx.req.supervised_speedups;

    // 1. Passes plus profondes.
    if speedups && max_load < lp.deeper_below_load_pct {
        let mut cap = ctx
            .tool
            .flute_length_mm
            .min(p.doc_max_ratio * ctx.d())
            // Un palier à la fois : on n'augmente jamais brutalement.
            .min(v.doc_max_mm * lp.deeper_max_step);
        if v.stepover_ratio >= p.rct_threshold_ratio {
            // Pleine largeur : au-delà, les copeaux s'évacuent mal.
            cap = cap.min(lp.deeper_full_width_max_ratio * ctx.d());
        }
        if let Some(c) = ctx.req.max_doc_mm {
            cap = cap.min(c);
        }
        let target = v.doc_max_mm * lp.deeper_target_load_pct / max_load;
        let new_doc = floor_step(target.min(cap), rnd.doc_step);
        let patch = RequestPatch {
            overrides: Some(Overrides {
                doc_mm: Some(new_doc),
                ..Default::default()
            }),
            ..Default::default()
        };
        match (v.depth_mm, v.passes) {
            (Some(depth), Some(passes_now)) if new_doc > v.doc_max_mm => {
                let passes_new = (depth / new_doc - 1e-9).ceil().max(1.0) as u32;
                if passes_new < passes_now {
                    let (from, to, per) = (
                        l.passes(passes_now),
                        l.passes(passes_new),
                        l.num_trim(depth / f64::from(passes_new), 2),
                    );
                    out.push(Lever {
                        id: "deeper_passes".into(),
                        title: tr!(l, "Moins de passes, plus profondes", "Fewer, deeper passes"),
                        detail: tr!(
                            l,
                            "La machine n'utilise que {load} % de sa marge. Passez de {from} à {to} ({per} mm par passe) : l'épaisseur du copeau ne change pas, c'est le levier le plus sûr.",
                            "The machine uses only {load}% of its margin. Go from {from} to {to} ({per} mm per pass): the chip thickness does not change, this is the safest lever."
                        ),
                        time_gain: Some(1.0 - f64::from(passes_new) / f64::from(passes_now)),
                        caution: beyond_lab.clone().or_else(|| {
                            Some(tr!(
                                l,
                                "Un palier à la fois : observez la coupe avant d'aller plus loin.",
                                "One step at a time: watch the cut before going further."
                            ))
                        }),
                        patch,
                    });
                }
            }
            (None, _) if new_doc > v.doc_max_mm * 1.1 => {
                let to = l.num_trim(new_doc, 2);
                out.push(Lever {
                    id: "deeper_passes".into(),
                    title: tr!(l, "Passes plus profondes", "Deeper passes"),
                    detail: tr!(
                        l,
                        "Marge disponible ({load} % de charge) : la profondeur de passe peut monter jusqu'à {to} mm sans changer le copeau.",
                        "Margin available ({load}% load): the depth of cut can go up to {to} mm without changing the chip."
                    ),
                    time_gain: Some(1.0 - v.doc_max_mm / new_doc),
                    caution: beyond_lab.clone(),
                    patch,
                });
            }
            _ => {}
        }
    }

    // 2. Hausse homothétique rotation + avance.
    let rpm_room = ctx.rpm_ceiling() / v.rpm;
    let feed_room = ctx.machine.max_feed_mm_min / v.feed_mm_min;
    let k = rpm_room.min(feed_room).min(1.0 + lp.homothetic_max_gain);
    let homothetic =
        speedups && k - 1.0 >= lp.homothetic_min_gain && m.loads.spindle_pct * k < 90.0;
    if homothetic {
        let rpm = floor_step(v.rpm * k, rnd.rpm_step);
        let feed = floor_step(v.feed_mm_min * (rpm / v.rpm), rnd.feed_step);
        if rpm > v.rpm && feed > v.feed_mm_min {
            let (r, f, gain) = (
                l.num(rpm, 0),
                l.num(feed, 0),
                l.num((rpm / v.rpm - 1.0) * 100.0, 0),
            );
            out.push(Lever {
                id: "rpm_and_feed".into(),
                title: tr!(l, "Tourner et avancer plus vite ensemble", "Raise speed and feed together"),
                detail: tr!(
                    l,
                    "La broche a de la réserve : {r} tr/min et {f} mm/min (+{gain} %). Même épaisseur de copeau, même effort par dent.",
                    "The spindle has headroom: {r} rpm and {f} mm/min (+{gain}%). Same chip thickness, same load per tooth."
                ),
                time_gain: Some(1.0 - v.feed_mm_min / feed),
                caution: beyond_lab.clone(),
                patch: RequestPatch {
                    overrides: Some(Overrides { rpm: Some(rpm), feed_mm_min: Some(feed), ..Default::default() }),
                    ..Default::default()
                },
            });
        }
    }

    // 3a. Fraise trop sortie de la pince.
    if limits.contains(&LimitingFactor::Slenderness) || m.slenderness > p.slenderness_threshold {
        let short = ctx.tool.flute_length_mm + p.stickout_margin_mm;
        if short < m.stickout_mm - 0.5 {
            let (now, target) = (l.num(m.stickout_mm, 0), l.num(short, 0));
            out.push(Lever {
                id: "shorter_stickout".into(),
                title: tr!(l, "Rentrer la fraise dans la pince", "Push the tool further into the collet"),
                detail: tr!(
                    l,
                    "Sortie actuelle {now} mm. À {target} mm, la fraise plie beaucoup moins (la flexion varie avec le cube de la longueur).",
                    "Current stickout {now} mm. At {target} mm the tool bends much less (deflection grows with the cube of the length)."
                ),
                time_gain: None,
                caution: Some(tr!(
                    l,
                    "Gardez assez de longueur pour la profondeur totale et le dégagement.",
                    "Keep enough length for the total depth and clearance."
                )),
                patch: RequestPatch { stickout_mm: Some(short), ..Default::default() },
            });
        }
    }

    // 3b. Maintien de pièce limitant.
    if limits.contains(&LimitingFactor::Workholding)
        && ctx.req.workholding != Workholding::ScrewsClamps
    {
        out.push(Lever {
            id: "better_workholding".into(),
            title: tr!(l, "Mieux tenir la pièce", "Hold the part better"),
            detail: tr!(
                l,
                "Le maintien limite la profondeur. Vissée dans le martyr ou bridée, la pièce supporte des efforts bien plus élevés.",
                "Workholding limits the depth. Screwed to the spoilboard or clamped, the part withstands much higher forces."
            ),
            time_gain: None,
            caution: Some(tr!(l, "Placez vis et brides hors des parcours d'outil.", "Keep screws and clamps away from the toolpaths.")),
            patch: RequestPatch { workholding: Some(Workholding::ScrewsClamps), ..Default::default() },
        });
    }

    // 4. Copeau plus épais (avance seule).
    let step = 1.0 + lp.chip_step;
    if speedups
        && !homothetic
        && m.chip_zone.state == ChipState::Ok
        && m.chip_thickness_mm * step <= m.chip_zone.max_mm
        && v.feed_mm_min * step <= ctx.machine.max_feed_mm_min
        && max_load * step < lp.faster_below_load_pct
    {
        let feed = floor_step(v.feed_mm_min * step, rnd.feed_step);
        if feed > v.feed_mm_min {
            let (pct, f, chip) = (
                l.num(lp.chip_step * 100.0, 0),
                l.num(feed, 0),
                l.num(m.chip_thickness_mm * feed / v.feed_mm_min, 3),
            );
            out.push(Lever {
                id: "thicker_chip".into(),
                title: tr!(l, "Copeau plus épais (+{pct} % d'avance)", "Thicker chip (+{pct}% feed)"),
                detail: tr!(
                    l,
                    "Avance à {f} mm/min à rotation égale : copeau de {chip} mm, encore dans la zone saine.",
                    "Feed at {f} mm/min at the same speed: {chip} mm chip, still in the healthy zone."
                ),
                time_gain: Some(1.0 - v.feed_mm_min / feed),
                caution: Some(tr!(
                    l,
                    "Surveillez l'état de surface et que la pièce ne bouge pas.",
                    "Watch the surface finish and make sure the part does not move."
                )),
                patch: RequestPatch {
                    overrides: Some(Overrides { feed_mm_min: Some(feed), ..Default::default() }),
                    ..Default::default()
                },
            });
        }
    }

    // 5. Mode plus rapide (calcul théorique uniquement).
    let next = match ctx.req.aggressiveness {
        Aggressiveness::Prudent => Some(Aggressiveness::Normal),
        Aggressiveness::Normal => Some(Aggressiveness::Rapid),
        Aggressiveness::Rapid => None,
    };
    if let Some(next) = next {
        if speedups
            && source == SourceKind::Model
            && ctx.req.overrides.is_empty()
            && max_load < lp.faster_below_load_pct
        {
            let mode = mode_label(l, next);
            out.push(Lever {
                id: "faster_mode".into(),
                title: tr!(l, "Passer en mode {mode}", "Switch to {mode} mode"),
                detail: tr!(
                    l,
                    "Charges faibles ({load} %) : le mode {mode} augmente le copeau et la profondeur visés.",
                    "Low loads ({load}%): {mode} mode raises the target chip and depth."
                ),
                time_gain: None,
                caution: Some(tr!(
                    l,
                    "À réserver à une pièce bien tenue et une fraise courte.",
                    "Only for a well-held part and a short tool."
                )),
                patch: RequestPatch { aggressiveness: Some(next), ..Default::default() },
            });
        }
    }
    out
}
