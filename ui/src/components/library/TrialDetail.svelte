<script lang="ts">
  // Détail d'un essai (inspecteur de la bibliothèque) : contexte complet,
  // valeurs, constats, provenance. « Ouvrir dans le calcul » restaure le contexte.
  import { Check, TriangleAlert, CircleX, Crosshair, Archive, ShieldCheck, ShieldOff, Lock } from '@lucide/svelte';
  import { store } from '../../lib/store.svelte';
  import { api } from '../../lib/api';
  import { dateTime, modeLabel, operationLabel, outcomeLabel, rpmUnit, toolConditionLabel, workholdingLabel } from '../../lib/labels';
  import { t } from '../../lib/i18n.svelte';
  import { fmtLength, fmtSpeed, lengthUnit, num, pct, speedUnit } from '../../lib/format';
  import { tip } from '../../lib/tip';
  import type { CuttingValues, Trial } from '../../lib/types';

  let { trial }: { trial: Trial } = $props();
  let confirmArchive = $state(false);
  /** Ce que donne le modèle seul pour le contexte de l'essai (sans réglages du lab). */
  let theory = $state<CuttingValues | null>(null);

  $effect(() => {
    const tr = trial;
    const ctrl = new AbortController();
    api
      .recommend(
        {
          machine_id: tr.machine_id,
          material_id: tr.material_id,
          tool_id: tr.tool_id,
          operation: tr.operation,
          stepover_ratio: tr.operation === 'pocket' || tr.operation === 'surfacing' ? tr.stepover_ratio : undefined,
          depth_mm: tr.depth_mm ?? undefined,
          aggressiveness: tr.aggressiveness,
          workholding: tr.workholding,
          stickout_mm: tr.stickout_mm ?? undefined,
          use_presets: false,
        },
        ctrl.signal,
      )
      .then((r) => (theory = r.values))
      .catch(() => (theory = null));
    return () => ctrl.abort();
  });

  /** Écart de l'essai au calcul théorique : « = », « +15 % », « −8 % ». */
  function gap(actual: number, model: number | undefined): string | null {
    if (model == null || !(model > 0)) return null;
    const p = ((actual - model) / model) * 100;
    if (Math.abs(p) < 0.5) return t('trialDetail.sameAsModel');
    return t('trialDetail.vsModel', { v: `${p > 0 ? '+' : '−'}${pct(Math.abs(p))}` });
  }

  const u = $derived(store.units);
  const ICON = { good: Check, adjust: TriangleAlert, failed: CircleX } as const;
  const I = $derived(ICON[trial.outcome]);
  const machine = $derived(store.catalog?.machines.find((m) => m.id === trial.machine_id));
  const material = $derived(store.catalog?.materials.find((m) => m.id === trial.material_id));
  const symptomLabel = (id: string) => store.catalog?.symptoms.find((s) => s.id === id)?.label ?? id;

  function open() {
    if (!machine || !material || !store.catalog?.tools.some((x) => x.id === trial.tool_id)) {
      store.toast(t('trialDetail.gone'));
      return;
    }
    store.setMachine(trial.machine_id);
    store.setMaterial(trial.material_id);
    store.setTool(trial.tool_id);
    store.setOperation(trial.operation);
    if (trial.thickness_mm) store.setup.thicknessMm = trial.thickness_mm;
    store.setup.depthMm = trial.depth_mm;
    store.setup.aggressiveness = trial.aggressiveness;
    store.setup.workholding = trial.workholding;
    store.setup.stepoverRatio = trial.operation === 'pocket' || trial.operation === 'surfacing' ? trial.stepover_ratio : null;
    store.assistantTab = 'trials';
    store.module = 'calc';
  }
</script>

<div class="editor">
  <header>
    <h2>{trial.tool_name}</h2>
  </header>
  <div class="out {trial.outcome}"><I size={15} /> {outcomeLabel(trial.outcome)}</div>
  <p class="note">{dateTime(trial.recorded_at)}{trial.operator ? ` · ${trial.operator}` : ''}</p>
  <code class="id">{trial.id}</code>

  <h3>{t('trialDetail.context')}</h3>
  <dl class="kv">
    <dt>{t('trialDetail.machine')}</dt><dd>{machine?.name ?? trial.machine_id}</dd>
    <dt>{t('toolEditor.material')}</dt><dd>{material?.name ?? trial.material_id}{trial.thickness_mm ? ` · ${fmtLength(trial.thickness_mm, u)} ${lengthUnit(u)}` : ''}</dd>
    <dt>{t('loads.tool')}</dt><dd>Ø{num(trial.tool_diameter_mm, 2)} · {trial.tool_flutes ? t(trial.tool_flutes > 1 ? 'tool.flutes' : 'tool.flute', { n: trial.tool_flutes }) : t('tool.flutesUnknown')}</dd>
    <dt>{t('help.step3.term')}</dt><dd>{operationLabel(trial.operation)}{trial.stepover_ratio < 1 ? ` · ${t('trials.stepover')} ${pct(trial.stepover_ratio * 100)}` : ''}</dd>
    <dt>{t('trialDetail.depth')}</dt><dd>{trial.depth_mm ? `${fmtLength(trial.depth_mm, u)} ${lengthUnit(u)}` : '—'}</dd>
    <dt>{t('trialDetail.mode')}</dt><dd>{modeLabel(trial.aggressiveness)}</dd>
    <dt>{t('loads.holding')}</dt><dd>{workholdingLabel(trial.workholding)}</dd>
    {#if trial.tool_condition}<dt>{t('trialDialog.toolCondition')}</dt><dd>{toolConditionLabel(trial.tool_condition)}</dd>{/if}
  </dl>

  <h3>{t('trialDetail.settings')}</h3>
  <dl class="kv tabular">
    {#each [
      { label: t('readout.rpm'), text: `${num(trial.rpm, 0)} ${rpmUnit()}`, g: gap(trial.rpm, theory?.rpm), model: theory ? `${num(theory.rpm, 0)} ${rpmUnit()}` : '' },
      { label: t('readout.feed'), text: `${fmtSpeed(trial.feed_mm_min, u)} ${speedUnit(u)}`, g: gap(trial.feed_mm_min, theory?.feed_mm_min), model: theory ? `${fmtSpeed(theory.feed_mm_min, u)} ${speedUnit(u)}` : '' },
      { label: t('readout.plunge'), text: `${fmtSpeed(trial.plunge_mm_min, u)} ${speedUnit(u)}`, g: gap(trial.plunge_mm_min, theory?.plunge_mm_min), model: theory ? `${fmtSpeed(theory.plunge_mm_min, u)} ${speedUnit(u)}` : '' },
      { label: t('assistant.pass'), text: `${fmtLength(trial.doc_mm, u)} ${lengthUnit(u)}`, g: gap(trial.doc_mm, theory?.doc_mm), model: theory ? `${fmtLength(theory.doc_mm, u)} ${lengthUnit(u)}` : '' },
    ] as row (row.label)}
      <dt>{row.label}</dt>
      <dd>
        {row.text}
        {#if row.g}<span class="gap" use:tip={t('trialDetail.modelHint', { v: row.model })}>{row.g}</span>{/if}
      </dd>
    {/each}
  </dl>

  {#if trial.symptoms.length}
    <h3>{t('trialDetail.observations')}</h3>
    <ul class="list">{#each trial.symptoms as s (s)}<li>{symptomLabel(s)}</li>{/each}</ul>
  {/if}
  {#if trial.notes}
    <h3>{t('trialDialog.notes')}</h3>
    <p class="text">{trial.notes}</p>
  {/if}
  <h3>{t('validate.section')}</h3>
  {#if trial.validation}
    <p class="validated"><ShieldCheck size={14} /> {t('validate.byOn', { date: dateTime(trial.validation.validated_at), by: trial.validation.validated_by || '—' })}</p>
    {#if trial.validation.note}<p class="text">{trial.validation.note}</p>{/if}
    <div class="actions">
      <button class="btn quiet" type="button" onclick={() => store.unvalidateTrial(trial.id)}><ShieldOff size={14} /> {t('validate.remove')} {#if !store.auth.admin}<Lock size={11} />{/if}</button>
    </div>
  {:else if trial.outcome === 'good'}
    <p class="note">{t('validate.notYet')}</p>
    <div class="actions">
      <button class="btn" type="button" onclick={() => store.openValidateDialog(trial.id)}><ShieldCheck size={14} /> {t('validate.title')} {#if !store.auth.admin}<Lock size={11} />{/if}</button>
    </div>
  {:else}
    <p class="note">{t('validate.onlyGood')}</p>
  {/if}

  <p class="note">FabSpeed {trial.engine_version} · {t('trialDetail.data', { v: trial.dataset_revision })}</p>

  <div class="actions">
    <button class="btn" type="button" onclick={open}><Crosshair size={14} /> {t('trialDetail.open')}</button>
  </div>
  <div class="danger">
    {#if confirmArchive}
      <span>{t('trials.archiveAsk')}</span>
      <button class="btn quiet" type="button" onclick={() => (confirmArchive = false)}>{t('common.no')}</button>
      <button
        class="btn"
        type="button"
        onclick={() => {
          store.archiveTrial(trial.id);
          store.library = { section: 'trials', selected: null, creating: false };
        }}>{t('common.yes')}</button
      >
    {:else}
      <button class="btn quiet" type="button" onclick={() => (confirmArchive = true)}><Archive size={14} /> {t('common.archive')}</button>
    {/if}
  </div>
</div>

<style>
  .validated {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--success);
    font-size: var(--fs-sm);
    font-weight: 600;
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }

  .out.good {
    color: var(--success);
  }

  .out.adjust {
    color: var(--warning);
  }

  .out.failed {
    color: var(--danger);
  }

  .kv {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: 3px var(--s2);
    margin: 0;
    font-size: var(--fs-sm);
  }

  .kv dt {
    color: var(--text-muted);
  }

  .kv dd {
    margin: 0;
  }

  .gap {
    margin-left: 6px;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    cursor: help;
  }

  .list {
    margin: 0;
    padding-left: 18px;
    color: var(--warning);
    font-size: var(--fs-sm);
  }

  .text {
    margin: 0;
    font-size: var(--fs-sm);
    line-height: 1.5;
    white-space: pre-wrap;
  }
</style>
