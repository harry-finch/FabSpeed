<script lang="ts">
  // Zone centrale : contexte, valeurs à reporter, schéma à l'échelle, santé de la coupe.
  import { Copy, FlaskConical, Calculator, RotateCcw, Ban, LoaderCircle, WifiOff, ClipboardPen, Lock, ShieldCheck } from '@lucide/svelte';
  import ReadoutCell from './ReadoutCell.svelte';
  import CutScene from './CutScene.svelte';
  import ChipGauge from './ChipGauge.svelte';
  import LoadMeters from './LoadMeters.svelte';
  import Alerts from './Alerts.svelte';
  import ToolGlyph from './ToolGlyph.svelte';
  import { store } from '../lib/store.svelte';
  import { evidence, limitLabel, modeLabel, operationLabel, rpmUnit } from '../lib/labels';
  import { labelled, t } from '../lib/i18n.svelte';
  import {
    fmtDuration,
    fmtLength,
    fmtSpeed,
    fmtSpeedPerSecond,
    fromDisplayLength,
    lengthDecimals,
    lengthUnit,
    num,
    numTrim,
    pct,
    plain,
    speedUnit,
    toDisplayLength,
  } from '../lib/format';
  import { swatch } from '../lib/swatches';
  import { tip } from '../lib/tip';
  import type { LimitingFactor } from '../lib/types';

  /** Seuil d'affichage : au-delà de ce nombre de passes, le badge passe en ambre (durée). */
  const MANY_PASSES = 8;
  /** Écart relatif à partir duquel le copeau réel est affiché à côté de l'avance par dent. */
  const CHIP_DIFF = 0.05;

  // Chaque plafond se rapporte à la valeur qu'il limite.
  const LIMITS_OF: Record<'rpm' | 'feed' | 'plunge' | 'doc', LimitingFactor[]> = {
    rpm: ['spindle_max_rpm', 'spindle_min_rpm', 'tool_max_rpm'],
    feed: ['machine_feed'],
    plunge: ['machine_plunge'],
    doc: ['slenderness', 'flute_length', 'user_doc_cap', 'spindle_power', 'machine_rigidity', 'tool_strength', 'workholding'],
  };

  const reco = $derived(store.reco);
  const v = $derived(reco?.values ?? null);
  const m = $derived(reco?.metrics ?? null);
  const o = $derived(reco?.origins ?? null);
  const u = $derived(store.units);
  const material = $derived(store.material);
  const tool = $derived(store.tool);
  const machine = $derived(store.machine);
  const shopbot = $derived(machine?.controller === 'shopbot');
  const pv = $derived(store.preview?.reco.values ?? null);
  /** Plafonds atteints pour une valeur (libellés), ou null. */
  const capOf = (k: keyof typeof LIMITS_OF) => {
    const l = (reco?.limits ?? []).filter((x) => LIMITS_OF[k].includes(x));
    return l.length ? l.map(limitLabel).join(' · ') : null;
  };
  const cutThrough = $derived(
    ['profile', 'full_slot'].includes(store.setup.operation) && store.effectiveDepth >= store.setup.thicknessMm - 1e-6,
  );
  const fromLab = $derived(reco?.provenance.source === 'lab_preset');
  const ev = $derived(reco ? evidence(reco.provenance.evidence) : null);
  const validated = $derived(reco?.provenance.evidence === 'validated');
  /** Le diamètre qui coupe est plus petit que la fraise (hémisphérique peu engagée, fraise en V). */
  const reducedDiameter = $derived(
    !!reco?.metrics && !!tool && reco.metrics.cutting_diameter_mm < tool.diameter_mm - 1e-6,
  );

  // Conversion des vitesses saisies dans l'unité affichée.
  const speedIn = (x: number | null) => (x == null ? undefined : u === 'mm' ? x : x * 25.4);
  const speedOut = (x: number) => (u === 'mm' ? x : Number((x / 25.4).toFixed(1)));

  async function copyValue(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      store.toast(t('readout.valueCopied', { v: text }));
    } catch {
      store.toast(t('readout.copyFailed'));
    }
  }

  async function copy() {
    if (!reco || !v) return;
    const lines = [
      `FabSpeed — ${reco.context.material_name} ${fmtLength(store.setup.thicknessMm, u)} ${lengthUnit(u)} · ${reco.context.tool_name} · ${operationLabel(reco.context.operation)} · ${modeLabel(reco.context.aggressiveness)}`,
      labelled(t('readout.rpm'), `${num(v.rpm, 0)} ${rpmUnit()}`),
      labelled(t('readout.feed'), `${fmtSpeed(v.feed_mm_min, u)} ${speedUnit(u)}${shopbot ? ` (${fmtSpeedPerSecond(v.feed_mm_min, u)})` : ''}`),
      labelled(t('readout.plunge'), `${fmtSpeed(v.plunge_mm_min, u)} ${speedUnit(u)}${shopbot ? ` (${fmtSpeedPerSecond(v.plunge_mm_min, u)})` : ''}`),
      labelled(t('readout.doc'), `${fmtLength(v.doc_mm, u)} ${lengthUnit(u)}${v.passes ? ` × ${t('readout.passesN', { n: v.passes })}` : ''}`),
      v.stepover_ratio < 1 ? labelled(t('setup.stepover'), `${pct(v.stepover_ratio * 100)} (${fmtLength(v.stepover_mm, u)} ${lengthUnit(u)})`) : '',
      labelled(t('readout.source'), `${reco.provenance.preset_label ?? t('readout.theoreticalLower')} — ${ev?.label}`),
    ].filter(Boolean);
    try {
      await navigator.clipboard.writeText(lines.join('\n'));
      store.toast(t('readout.copied'));
    } catch {
      store.toast(t('readout.copyFailed'));
    }
  }
</script>

<main class="ws" aria-label={t('ws.label')}>
  <header class="ctx">
    {#if material}
      <span class="crumb"><span class="sw" style:background={swatch(material.id, material.category)}></span>{material.name} · {fmtLength(store.setup.thicknessMm, u)} {lengthUnit(u)}</span>
    {/if}
    {#if tool}
      <span class="sep">›</span>
      <span class="crumb"><ToolGlyph family={tool.family} helix={tool.helix} size={14} />{tool.reference || tool.name}</span>
    {/if}
    <span class="sep">›</span>
    <span class="crumb">
      {operationLabel(store.setup.operation)}
      {#if cutThrough}<span class="qual">· {t('ws.through')}</span>{/if}
      {#if v && tool?.family !== 'v_bit'}<span class="qual" use:tip={t('ws.engagementHint')}>· {pct(v.stepover_ratio * 100)} Ø</span>{/if}
    </span>
    <span class="sep">›</span>
    <span class="crumb">{modeLabel(store.setup.aggressiveness)}</span>
    <span class="grow"></span>
    {#if store.loading}<LoaderCircle size={14} class="spin" />{/if}
    {#if reco && reco.status === 'ok'}
      <span class="source" class:lab={fromLab} class:validated use:tip={`${validated ? `${reco.provenance.preset_label}\n` : ''}${reco.provenance.preset_source ?? t('ws.model')}\n${ev?.hint ?? ''}`}>
        {#if fromLab}
          {#if validated}<ShieldCheck size={13} /> {t('ws.validatedSetting')}{:else}<FlaskConical size={13} /> {reco.provenance.preset_label ?? t('ws.testedSetting')}{/if}
          {#if reco.provenance.evidence !== 'tested' && !validated}<span class="ev">· {ev?.label}</span>{/if}
        {:else}
          <Calculator size={13} /> {t('ws.theoretical')}
        {/if}
      </span>
    {/if}
  </header>

  {#if !store.online}
    <div class="empty"><WifiOff size={22} /><p>{t('api.offline')}</p><p class="muted">{t('ws.startServer')} <code>cargo run -p fabspeed-server</code></p></div>
  {:else if store.error && !reco}
    <div class="empty"><Ban size={22} /><p>{store.error}</p></div>
  {:else if reco && reco.status === 'unsupported'}
    <div class="empty">
      <Ban size={22} />
      <p>{t('ws.unsupported')}</p>
      <p class="muted">{reco.warnings.find((w) => w.severity === 'blocking')?.message}</p>
    </div>
  {:else if reco && v && m && o && tool}
    {@const chipDiffers = Math.abs(m.chip_thickness_mm - m.chipload_mm) > m.chipload_mm * CHIP_DIFF}
    <section class="readout" aria-label={t('readout.label')}>
      <ReadoutCell
        label={t('readout.rpm')}
        hint={t('readout.rpmHint')}
        display={num(v.rpm, 0)}
        unit={rpmUnit()}
        secondary={reducedDiameter
          ? t('readout.cuttingSpeedAt', { v: num(m.cutting_speed_m_min, 0), d: fmtLength(m.cutting_diameter_mm, u), u: lengthUnit(u) })
          : t('readout.cuttingSpeed', { v: num(m.cutting_speed_m_min, 0) })}
        copyText={plain(v.rpm, 0)}
        onCopy={copyValue}
        preview={pv ? num(pv.rpm, 0) : null}
        limit={capOf('rpm')}
        origin={o.rpm}
        {validated}
        editValue={v.rpm}
        onEdit={(x) => store.setOverride('rpm', x ?? undefined)}
        onReset={() => store.setOverride('rpm', undefined)}
      />
      <ReadoutCell
        label={t('readout.feed')}
        hint={t('readout.feedHint')}
        display={fmtSpeed(v.feed_mm_min, u)}
        unit={speedUnit(u)}
        copyText={u === 'mm' ? plain(v.feed_mm_min, 0) : plain(v.feed_mm_min / 25.4, 1)}
        onCopy={copyValue}
        preview={pv ? fmtSpeed(pv.feed_mm_min, u) : null}
        limit={capOf('feed')}
        origin={o.feed}
        {validated}
        editValue={speedOut(v.feed_mm_min)}
        editDecimals={u === 'mm' ? 0 : 1}
        onEdit={(x) => store.setOverride('feed_mm_min', speedIn(x))}
        onReset={() => store.setOverride('feed_mm_min', undefined)}
      >
        {#snippet details()}
          {#if shopbot}
            <button
              class="persec tabular"
              type="button"
              use:tip={t('readout.copyPerSecond')}
              onclick={() => copyValue(u === 'mm' ? plain(v.feed_mm_min / 60, 1) : plain(v.feed_mm_min / 60 / 25.4, 2))}
              >{t('readout.forShopbot', { v: fmtSpeedPerSecond(v.feed_mm_min, u) })}</button
            > ·
          {/if}
          {#if chipDiffers}
            <span use:tip={t('readout.chiploadRealHint')}
              >{t('readout.chiploadReal', { v: num(m.chipload_mm, 3), h: num(m.chip_thickness_mm, 3) })}</span
            >
          {:else}
            {t('readout.chipload', { v: num(m.chipload_mm, 3) })}
          {/if}
        {/snippet}
      </ReadoutCell>
      <ReadoutCell
        label={t('readout.plunge')}
        hint={t('readout.plungeHint')}
        display={fmtSpeed(v.plunge_mm_min, u)}
        unit={speedUnit(u)}
        secondary={v.ramp_entry_advised ? t('readout.ramp') : shopbot ? fmtSpeedPerSecond(v.plunge_mm_min, u) : undefined}
        copyText={u === 'mm' ? plain(v.plunge_mm_min, 0) : plain(v.plunge_mm_min / 25.4, 1)}
        onCopy={copyValue}
        preview={pv ? fmtSpeed(pv.plunge_mm_min, u) : null}
        limit={capOf('plunge')}
        origin={o.plunge}
        {validated}
        editValue={speedOut(v.plunge_mm_min)}
        editDecimals={u === 'mm' ? 0 : 1}
        onEdit={(x) => store.setOverride('plunge_mm_min', speedIn(x))}
        onReset={() => store.setOverride('plunge_mm_min', undefined)}
      />
      <ReadoutCell
        label={t('readout.doc')}
        hint={t('readout.docHint')}
        display={fmtLength(v.doc_mm, u)}
        unit={lengthUnit(u)}
        copyText={plain(toDisplayLength(v.doc_mm, u), lengthDecimals(u))}
        onCopy={copyValue}
        preview={pv ? fmtLength(pv.doc_mm, u) : null}
        limit={capOf('doc')}
        origin={o.doc}
        {validated}
        editValue={Number(toDisplayLength(v.doc_max_mm, u).toFixed(3))}
        editDecimals={lengthDecimals(u)}
        onEdit={(x) => store.setOverride('doc_mm', x == null ? undefined : fromDisplayLength(x, u))}
        onReset={() => store.setOverride('doc_mm', undefined)}
      >
        {#snippet details()}
          {#if v.passes}
            <span
              class="passes"
              class:many={v.passes > MANY_PASSES}
              use:tip={v.passes > MANY_PASSES ? t('readout.manyPassesHint') : undefined}
              >{t(v.passes > 1 ? 'readout.passesN' : 'readout.passN', { n: v.passes })}{#if pv?.passes && pv.passes !== v.passes}<span class="pvp"> → {pv.passes}</span>{/if}</span
            >
            · max {fmtLength(v.doc_max_mm, u)} {lengthUnit(u)}
          {/if}
          {#if m.estimated_time_s != null}
            · ≈ {fmtDuration(m.estimated_time_s)}
          {:else if m.time_per_metre_s > 0}
            · <span use:tip={t('readout.perMetreHint')}>{t('readout.perMetre', { v: fmtDuration(m.time_per_metre_s) })}</span>
          {/if}
        {/snippet}
      </ReadoutCell>
      <div class="actions">
        <button
          class="btn primary"
          type="button"
          onclick={() => store.openTrialDialog()}
          use:tip={t('readout.recordHint')}
        >
          <ClipboardPen size={14} /> {t('readout.record')} {#if !store.auth.admin}<Lock size={12} />{/if}
        </button>
        <button class="btn" type="button" onclick={copy} use:tip={t('readout.copyHint')}>
          <Copy size={14} /> {t('readout.copy')}
        </button>
        {#if store.modified}
          <button class="btn quiet" type="button" use:tip={t('readout.resetAllHint')} onclick={() => store.resetAll()}>
            <RotateCcw size={13} /> {t('readout.resetAll')}
          </button>
        {/if}
      </div>
    </section>

    <Alerts warnings={reco.warnings} />

    <section class="canvas" aria-label={t('ws.diagram')}>
      <CutScene
        {tool}
        thicknessMm={store.setup.thicknessMm}
        depthMm={v.depth_mm ?? v.doc_mm}
        docMm={v.doc_mm}
        passes={v.passes ?? 1}
        stickoutMm={m.stickout_mm}
        stepoverRatio={v.stepover_ratio}
        operation={store.setup.operation}
        materialColor={material ? swatch(material.id, material.category) : '#999'}
        feedLabel={`${fmtSpeed(v.feed_mm_min, u)} ${speedUnit(u)}`}
        rpmLabel={`${num(v.rpm, 0)} ${rpmUnit()}`}
        units={u}
        cuttingDiameterMm={m.cutting_diameter_mm}
      />
    </section>

    <section class="health" aria-label={t('ws.health')}>
      <ChipGauge metrics={m} />
      <LoadMeters metrics={m} />
      <dl class="facts tabular">
        <div><dt>{t('ws.mrr')}</dt><dd>{numTrim(m.mrr_cm3_min, 1)} cm³/min</dd></div>
        <div><dt>{t('ws.power')}</dt><dd>{num(m.cutting_power_w, 0)} W</dd></div>
        <div><dt>{t('ws.force')}</dt><dd>{numTrim(m.cutting_force_n, 1)} N</dd></div>
        {#if reducedDiameter}
          <div use:tip={tool?.family === 'v_bit' ? t('ws.engravedWidthHint') : t('ws.cuttingDiameterHint')}>
            <dt>{tool?.family === 'v_bit' ? t('ws.engravedWidth') : t('ws.cuttingDiameter')}</dt>
            <dd>{fmtLength(m.cutting_diameter_mm, u)} {lengthUnit(u)}</dd>
          </div>
        {/if}
        {#if tool?.family !== 'v_bit'}
          <div>
            <dt>{t('setup.stepover')}</dt>
            <dd>{pct(v.stepover_ratio * 100)} · {fmtLength(v.stepover_mm, u)} {lengthUnit(u)}</dd>
          </div>
        {/if}
      </dl>
    </section>
  {:else}
    <div class="empty"><LoaderCircle size={22} class="spin" /><p>{t('ws.computing')}</p></div>
  {/if}
</main>

<style>
  .ws {
    display: flex;
    flex-direction: column;
    min-width: 0;
    /* Les valeurs s'adaptent à la largeur réelle du centre (panneaux latéraux compris), pas à celle de l'écran. */
    container: ws / inline-size;
    min-height: 0;
    background: var(--bg-workspace);
    overflow: auto;
  }

  .ctx {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 0 var(--s3);
    border-bottom: 1px solid var(--divider);
    background: var(--bg-panel);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    flex: none;
    flex-wrap: wrap;
  }

  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }

  .sw {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .sep {
    color: var(--text-disabled);
  }

  .grow {
    flex: 1;
  }

  .source {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--neutral-tint);
    color: var(--text-secondary);
    font-weight: 600;
    white-space: nowrap;
  }

  .source.lab {
    color: var(--origin-lab);
    background: var(--origin-lab-tint);
  }

  .source.lab.validated {
    color: var(--success);
    border-color: var(--success-border);
    background: var(--success-tint);
  }

  .ev {
    font-weight: 500;
    opacity: 0.85;
  }

  :global(.spin) {
    animation: spin 0.9s linear infinite;
    color: var(--text-muted);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .readout {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr)) auto;
    align-items: stretch;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--divider);
    flex: none;
  }

  .actions {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    padding: var(--s2) var(--s3);
    border-left: 1px solid var(--divider);
  }

  .qual {
    color: var(--text-muted);
  }

  .persec {
    padding: 0 2px;
    margin-left: -2px;
    border: 0;
    border-radius: 3px;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline dotted;
    text-underline-offset: 2px;
    cursor: copy;
  }

  .persec:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .passes {
    display: inline-block;
    padding: 0 5px;
    border-radius: 3px;
    background: var(--neutral-tint);
    color: var(--text-primary);
    font-weight: 600;
  }

  .passes.many {
    background: var(--warning-tint);
    color: var(--warning);
  }

  .pvp {
    color: var(--success);
  }

  .canvas {
    flex: 1;
    min-height: 240px;
    padding: var(--s3) var(--s4);
  }

  .health {
    display: grid;
    grid-template-columns: minmax(200px, 1.2fr) minmax(200px, 1fr) minmax(170px, 0.8fr);
    gap: var(--s5);
    padding: var(--s3) var(--s4);
    border-top: 1px solid var(--divider);
    background: var(--bg-panel);
    flex: none;
  }

  .facts {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--fs-sm);
  }

  .facts div {
    display: flex;
    justify-content: space-between;
    gap: var(--s2);
  }

  .facts dt {
    color: var(--text-muted);
  }

  .facts dd {
    margin: 0;
    color: var(--text-primary);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: var(--s5);
    color: var(--text-secondary);
    text-align: center;
  }

  .empty p {
    margin: 0;
    max-width: 460px;
  }

  .muted {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  @container ws (max-width: 900px) {
    .readout {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .actions {
      grid-column: 1 / -1;
      flex-direction: row;
      border-left: 0;
      border-top: 1px solid var(--divider);
    }

    .health {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
