<script lang="ts">
  // Panneau droit « Assistant atelier » : aller plus vite, diagnostiquer un
  // problème, comprendre le calcul. Aucune modification n'est appliquée
  // sans clic explicite.
  import {
    Gauge,
    Stethoscope,
    BookOpen,
    ArrowRight,
    Check,
    Flame,
    Wind,
    Droplets,
    AudioWaveform,
    TriangleAlert,
    Layers,
    ScanLine,
    Move,
    Zap,
    CircleHelp,
    X,
    ClipboardList,
    ClipboardPen,
    Library,
    Lock,
  } from '@lucide/svelte';
  import TrialList from './TrialList.svelte';
  import { store, type AssistantTab } from '../lib/store.svelte';
  import { evidence, limitLabel, MACHINE_FIELD_IDS, machineFieldLabel, rpmUnit, valueStatusLabel } from '../lib/labels';
  import { t } from '../lib/i18n.svelte';
  import { fmtLength, fmtSpeed, lengthUnit, num, pct, speedUnit } from '../lib/format';
  import { tip } from '../lib/tip';
  import type { Lever } from '../lib/types';

  const ICONS: Record<string, typeof Flame> = {
    flame: Flame,
    wind: Wind,
    droplets: Droplets,
    'audio-waveform': AudioWaveform,
    'triangle-alert': TriangleAlert,
    layers: Layers,
    'scan-line': ScanLine,
    move: Move,
    zap: Zap,
  };

  const TABS: { id: AssistantTab; icon: typeof Flame }[] = [
    { id: 'faster', icon: Gauge },
    { id: 'problem', icon: Stethoscope },
    { id: 'trials', icon: ClipboardList },
    { id: 'explain', icon: BookOpen },
  ];

  const reco = $derived(store.reco);
  const u = $derived(store.units);
  const category = $derived(store.material?.category);
  const symptoms = $derived(
    (store.catalog?.symptoms ?? []).filter((s) => !s.categories.length || (category && s.categories.includes(category))),
  );
  const d = $derived(store.diagnosis);
  const machine = $derived(store.machine);

  function apply(l: Lever) {
    store.applyPatch(l.patch, t('assistant.leverApplied', { title: l.title }));
  }

  const gain = (g: number | null) => (g != null && g > 0.005 ? t('assistant.timeGain', { v: num(g * 100, 0) }) : null);

  function delta(before: number, after: number): string {
    if (Math.abs(after - before) < 1e-9) return '=';
    const p = ((after - before) / before) * 100;
    return `${p > 0 ? '+' : '−'}${pct(Math.abs(p))}`;
  }
</script>

<aside class="panel" aria-label={t('assistant.label')}>
  <div class="tabs" role="tablist" aria-label={t('assistant.label')}>
    {#each TABS as tab (tab.id)}
      <button
        type="button"
        role="tab"
        aria-selected={store.assistantTab === tab.id}
        class:on={store.assistantTab === tab.id}
        onclick={() => (store.assistantTab = tab.id)}
      >
        <tab.icon size={13} />
        {t(`assistant.tab.${tab.id}`)}
      </button>
    {/each}
  </div>

  <div class="scroll" role="tabpanel">
    {#if store.assistantTab === 'faster'}
      <p class="intro">{t('assistant.fasterIntro')}</p>
      {#if store.material?.supervision_required && !reco?.warnings.some((w) => w.severity === 'blocking')}
        <div class="metal" class:admin={store.auth.admin}>
          <TriangleAlert size={14} />
          {#if store.auth.admin}
            <p>{t('assistant.metalsAdmin')}</p>
          {:else}
            <p>{t('assistant.metals')}</p>
            <button class="btn" type="button" onclick={() => store.ensureAdmin()}><Lock size={13} /> {t('assistant.metalsUnlock')}</button>
          {/if}
        </div>
      {/if}
      {#if reco?.levers.length}
        <ol class="levers">
          {#each reco.levers as l, i (l.id)}
            <!-- Survol ou focus : les valeurs que donnerait ce levier s'affichent au centre. -->
            <li
              class="lever"
              class:previewing={store.preview?.id === l.id}
              onmouseenter={() => store.previewLever(l.id, l.patch)}
              onmouseleave={() => store.cancelPreview()}
              onfocusin={() => store.previewLever(l.id, l.patch)}
              onfocusout={() => store.cancelPreview()}
            >
              <div class="lh">
                <span class="rank">{i + 1}</span>
                <span class="lt">{l.title}</span>
                {#if gain(l.time_gain)}<span class="gain">{gain(l.time_gain)}</span>{/if}
              </div>
              <p class="ld">{l.detail}</p>
              {#if l.caution}<p class="lc"><TriangleAlert size={12} /> {l.caution}</p>{/if}
              <button class="btn" type="button" onclick={() => apply(l)}>{t('assistant.try')} <ArrowRight size={13} /></button>
            </li>
          {/each}
        </ol>
      {:else if reco?.warnings.some((w) => w.severity === 'blocking')}
        <div class="empty">
          <TriangleAlert size={18} />
          <p>{t('assistant.blocked')}</p>
        </div>
      {:else if reco?.status === 'ok' && (!store.material?.supervision_required || store.auth.admin)}
        <div class="empty">
          <Check size={18} />
          <p>{t('assistant.balanced')}</p>
        </div>
      {/if}
      {#if reco?.limits.length}
        <h3>{t('assistant.limits')}</h3>
        <ul class="limits">
          {#each reco.limits as l (l)}<li>{limitLabel(l)}</li>{/each}
        </ul>
      {/if}
    {:else if store.assistantTab === 'problem'}
      {#if d}
        {@const S = ICONS[d.symptom.icon] ?? CircleHelp}
        <div class="diag">
          <div class="dh">
            <S size={16} />
            <span>{d.symptom.label}</span>
            <button class="btn quiet icon" type="button" use:tip={t('common.close')} onclick={() => (store.diagnosis = null)}><X size={14} /></button>
          </div>
          <p class="cause">{d.symptom.cause}</p>
          <p class="ld">{d.symptom.explanation}</p>
          {#if d.after.values}
            {@const a = d.after.values}
            <table class="cmp tabular">
              <thead><tr><th></th><th>{t('assistant.before')}</th><th>{t('assistant.after')}</th><th></th></tr></thead>
              <tbody>
                <tr><td>{t('readout.rpm')}</td><td>{num(d.before.rpm, 0)}</td><td>{num(a.rpm, 0)}</td><td class="dl">{delta(d.before.rpm, a.rpm)}</td></tr>
                <tr><td>{t('readout.feed')}</td><td>{fmtSpeed(d.before.feed_mm_min, u)}</td><td>{fmtSpeed(a.feed_mm_min, u)}</td><td class="dl">{delta(d.before.feed_mm_min, a.feed_mm_min)}</td></tr>
                <tr><td>{t('assistant.pass')}</td><td>{fmtLength(d.before.doc_max_mm, u)}</td><td>{fmtLength(a.doc_max_mm, u)}</td><td class="dl">{delta(d.before.doc_max_mm, a.doc_max_mm)}</td></tr>
              </tbody>
            </table>
            <p class="units">{rpmUnit()} · {speedUnit(u)} · {lengthUnit(u)}</p>
          {/if}
          {#if d.symptom.advice.length}
            <h3>{t('assistant.alsoCheck')}</h3>
            <ul class="advice">
              {#each d.symptom.advice as a (a)}<li>{a}</li>{/each}
            </ul>
          {/if}
          <button
            class="btn primary"
            type="button"
            onclick={() => {
              store.applyPatch(d.patch, t('assistant.fixesApplied'));
              store.diagnosis = null;
            }}>{t('assistant.applyFixes')}</button
          >
        </div>
      {:else}
        <p class="intro">{t('assistant.problemIntro')}</p>
        <div class="symptoms">
          {#each symptoms as s (s.id)}
            {@const S = ICONS[s.icon] ?? CircleHelp}
            <button class="symptom" type="button" onclick={() => store.diagnose(s.id)}>
              <S size={15} />
              <span>{s.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    {:else if store.assistantTab === 'trials'}
      <div class="trials-head">
        <p class="intro">
          {t('assistant.trialsFor')} <b>{store.material?.name}</b> {t('assistant.with')} <b>{store.tool?.reference || store.tool?.name}</b> {t('assistant.trialsOnMachine')}
        </p>
        <button class="btn primary" type="button" disabled={!reco?.values} onclick={() => store.openTrialDialog()}>
          <ClipboardPen size={14} /> {t('readout.record')} {#if !store.auth.admin}<Lock size={12} />{/if}
        </button>
      </div>
      {#if reco?.history.length}
        <TrialList trials={reco.history} />
      {:else}
        <div class="empty">
          <ClipboardList size={18} />
          <p>{t('assistant.noTrials')}</p>
        </div>
      {/if}
      <button class="btn quiet all" type="button" onclick={() => store.openLibrary('trials')}>
        <Library size={13} /> {t('assistant.allTrials')}
      </button>
    {:else if reco}
      <h3>{t('assistant.provenance')}</h3>
      <dl class="kv">
        <div><dt>{t('readout.source')}</dt><dd>{reco.provenance.preset_label ?? t('assistant.theoreticalModel')}</dd></div>
        {#if reco.provenance.preset_source}<div><dt>{t('assistant.reference')}</dt><dd>{reco.provenance.preset_source}</dd></div>{/if}
        <div>
          <dt>{t('assistant.reliability')}</dt>
          <dd use:tip={evidence(reco.provenance.evidence).hint}>{evidence(reco.provenance.evidence).label}</dd>
        </div>
        <div><dt>{t('assistant.data')}</dt><dd>{reco.provenance.dataset_revision}</dd></div>
        <div><dt>{t('assistant.engine')}</dt><dd>{reco.provenance.engine_version}</dd></div>
      </dl>
      {#each reco.provenance.notes as n (n)}<p class="note">{n}</p>{/each}

      {#if reco.theoretical && reco.values}
        <h3>{t('assistant.testedOrCalc')}</h3>
        <table class="cmp tabular">
          <thead><tr><th></th><th>{t('assistant.tested')}</th><th>{t('assistant.tab.explain')}</th></tr></thead>
          <tbody>
            <tr><td>{t('readout.rpm')}</td><td>{num(reco.values.rpm, 0)}</td><td>{num(reco.theoretical.rpm, 0)}</td></tr>
            <tr><td>{t('readout.feed')}</td><td>{fmtSpeed(reco.values.feed_mm_min, u)}</td><td>{fmtSpeed(reco.theoretical.feed_mm_min, u)}</td></tr>
            <tr><td>{t('assistant.maxPass')}</td><td>{fmtLength(reco.values.doc_max_mm, u)}</td><td>{fmtLength(reco.theoretical.doc_mm, u)}</td></tr>
          </tbody>
        </table>
        <p class="note">{t('assistant.calcOnly')}</p>
      {/if}

      <h3>{t('assistant.steps')}</h3>
      <ol class="trace">
        {#each reco.trace as step, i (i)}
          <li>
            <span class="tl">{step.label}</span>
            <code>{step.formula}</code>
            <span class="tr">{step.result}</span>
          </li>
        {/each}
      </ol>

      {#if machine}
        <h3>{t('assistant.machineLimits')}</h3>
        <dl class="kv">
          {#each MACHINE_FIELD_IDS as k (k)}
            {#if machine.sources[k] || k in machine}
              {@const src = machine.sources[k]}
              <div>
                <dt>{machineFieldLabel(k)}</dt>
                <dd>
                  {#if k === 'rpm_min' || k === 'rpm_max' || k === 'spindle_rated_rpm'}{num((machine as never)[k], 0)} {rpmUnit()}
                  {:else if k === 'spindle_power_w'}{num(machine.spindle_power_w, 0)} W
                  {:else if k === 'max_feed_mm_min'}{fmtSpeed(machine.max_feed_mm_min, u)} {speedUnit(u)}
                  {:else if k === 'max_lateral_force_n'}{num(machine.max_lateral_force_n, 0)} N
                  {:else if k === 'work_area_mm' && machine.work_area_mm}{machine.work_area_mm.map((x) => num(x, 0)).join(' × ')} mm
                  {:else}—{/if}
                  {#if src && src.status !== 'verified'}
                    <span class="verify {src.status}" use:tip={src.note}>{valueStatusLabel(src.status)}</span>
                  {/if}
                </dd>
              </div>
            {/if}
          {/each}
        </dl>
      {/if}
    {/if}
  </div>
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
  }

  .tabs {
    display: flex;
    height: 32px;
    border-bottom: 1px solid var(--divider);
    flex: none;
  }

  .tabs button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    font-weight: 500;
    white-space: nowrap;
  }

  .tabs button:hover {
    color: var(--text-primary);
  }

  .tabs button.on {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .trials-head {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s2);
    padding-bottom: var(--s2);
    border-bottom: 1px solid var(--divider);
  }

  .intro b {
    color: var(--text-primary);
    font-weight: 600;
  }

  .all {
    align-self: flex-start;
    margin-top: var(--s1);
  }

  .intro {
    margin: 0 0 var(--s1);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  h3 {
    margin: var(--s3) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .levers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .lever {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 5px;
    padding: var(--s3) 0;
    border-bottom: 1px solid var(--divider);
  }

  .lever.previewing .lt {
    color: var(--success);
  }

  .lever:first-child {
    padding-top: var(--s1);
  }

  .lh {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
  }

  .rank {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-tint);
    color: var(--accent-text);
    font-size: var(--fs-xs);
    font-weight: 700;
    flex: none;
  }

  .lt {
    font-weight: 600;
  }

  .gain {
    margin-left: auto;
    color: var(--accent-text);
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .ld {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }

  .lc {
    display: flex;
    gap: 5px;
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .lc :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--warning);
  }

  /* Icône à gauche, texte sur toute la largeur restante, bouton dessous. */
  .metal {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: var(--s2);
    row-gap: var(--s2);
    padding-bottom: var(--s2);
    border-bottom: 1px solid var(--divider);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .metal p {
    margin: 0;
    line-height: 1.5;
  }

  .metal > :global(svg) {
    margin-top: 2px;
    color: var(--warning);
  }

  .metal .btn {
    grid-column: 2;
    justify-self: start;
  }

  .empty {
    display: flex;
    gap: var(--s2);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .empty p {
    margin: 0;
    line-height: 1.5;
  }

  .limits,
  .advice {
    margin: 0;
    padding-left: 18px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.6;
  }

  .symptoms {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .symptom {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: 34px;
    padding: 6px var(--s2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-control);
    text-align: left;
    font-size: var(--fs-sm);
  }

  .symptom:hover {
    border-color: var(--border-strong);
    background: var(--bg-hover);
  }

  .symptom :global(svg) {
    color: var(--text-secondary);
    flex: none;
  }

  .diag {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .dh {
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-weight: 600;
  }

  .dh .btn {
    margin-left: auto;
  }

  .cause {
    margin: 0;
    font-weight: 500;
  }

  .cmp {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-sm);
  }

  .cmp th {
    text-align: right;
    font-weight: 500;
    color: var(--text-muted);
    padding: 3px 4px;
  }

  .cmp td {
    text-align: right;
    padding: 4px;
    border-top: 1px solid var(--divider);
  }

  .cmp td:first-child {
    text-align: left;
    color: var(--text-secondary);
  }

  .cmp .dl {
    color: var(--accent-text);
    font-weight: 600;
  }

  .units {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    text-align: right;
  }

  .kv {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: var(--fs-sm);
  }

  .kv div {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: var(--s2);
  }

  .kv dt {
    color: var(--text-muted);
  }

  .kv dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .verify {
    font-size: var(--fs-xs);
    font-weight: 600;
    padding: 0 4px;
    border-radius: 3px;
    cursor: help;
  }

  .verify.estimate {
    color: var(--text-secondary);
    background: var(--neutral-tint);
  }

  .verify.to_verify {
    color: var(--warning);
    background: var(--warning-tint);
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .trace {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .trace li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: var(--s2);
    border-left: 2px solid var(--border-strong);
  }

  .tl {
    font-weight: 600;
    font-size: var(--fs-sm);
  }

  .trace code {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-muted);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .tr {
    font-size: var(--fs-sm);
    color: var(--text-primary);
  }
</style>
