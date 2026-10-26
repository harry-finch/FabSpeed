<script lang="ts">
  // Jauge pédagogique de l'épaisseur du copeau : trop fin (la fraise frotte
  // et chauffe), zone saine, trop épais (efforts, casse).
  import { Info } from '@lucide/svelte';
  import { num } from '../lib/format';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import type { Metrics } from '../lib/types';

  let { metrics }: { metrics: Metrics } = $props();

  const z = $derived(metrics.chip_zone);
  const span = $derived(Math.max(z.max_mm * 1.35, metrics.chip_thickness_mm * 1.1));
  const at = (v: number) => `${Math.min(100, Math.max(0, (v / span) * 100))}%`;
  const STATE = {
    rubbing: { key: 'chip.rubbing', cls: 'warn' },
    ok: { key: 'chip.ok', cls: 'ok' },
    thick: { key: 'chip.thick', cls: 'warn' },
  } as const;
  const st = $derived(STATE[z.state]);
</script>

<div class="gauge">
  <div class="head">
    <span class="title">{t('chip.title')}</span>
    <span
      class="i"
      use:tip={t('chip.hint')}
      ><Info size={11} /></span
    >
  </div>
  <div class="bar" role="meter" aria-valuenow={metrics.chip_thickness_mm} aria-valuemin={0} aria-valuemax={span} aria-label={t('chip.title')}>
    <div class="zone rub" style:width={at(z.min_mm)}></div>
    <div class="zone good" style:left={at(z.min_mm)} style:width={`calc(${at(z.max_mm)} - ${at(z.min_mm)})`}></div>
    <div class="zone thick" style:left={at(z.max_mm)} style:right="0"></div>
    <div class="nominal" style:left={at(z.nominal_mm)} use:tip={t('chip.nominal', { v: num(z.nominal_mm, 3) })}></div>
    <div class="marker" style:left={at(metrics.chip_thickness_mm)}></div>
  </div>
  <!-- Valeur et verdict sous le repère, pour qu'on lise « zone saine » là où il se trouve. -->
  <div class="scale tabular">
    <span>{t('chip.rubs')}</span>
    <span class="val" style:left={`clamp(100px, ${at(metrics.chip_thickness_mm)}, calc(100% - 100px))`}>
      {t('readout.chipload', { v: num(metrics.chip_thickness_mm, 3) })} · <span class="state {st.cls}">{t(st.key)}</span>
    </span>
    <span>{t('chip.breaks')}</span>
  </div>
</div>

<style>
  .gauge {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .title {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .i {
    display: inline-flex;
    color: var(--text-muted);
    cursor: help;
  }

  .state {
    font-weight: 600;
  }

  .state.ok {
    color: var(--text-primary);
  }

  .state.warn {
    color: var(--warning);
  }

  .bar {
    position: relative;
    height: 10px;
    border-radius: 5px;
    background: var(--track);
    overflow: visible;
  }

  .zone {
    position: absolute;
    top: 0;
    bottom: 0;
  }

  .rub {
    left: 0;
    border-radius: 5px 0 0 5px;
    background: repeating-linear-gradient(
      135deg,
      color-mix(in srgb, var(--warning) 35%, transparent) 0 3px,
      color-mix(in srgb, var(--warning) 12%, transparent) 3px 6px
    );
  }

  .good {
    background: color-mix(in srgb, var(--success) 35%, transparent);
  }

  .thick {
    border-radius: 0 5px 5px 0;
    background: repeating-linear-gradient(
      135deg,
      color-mix(in srgb, var(--warning) 35%, transparent) 0 3px,
      color-mix(in srgb, var(--warning) 12%, transparent) 3px 6px
    );
  }

  .nominal {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 1px;
    background: var(--gauge-tick);
  }

  .marker {
    position: absolute;
    top: -4px;
    width: 4px;
    height: 18px;
    margin-left: -2px;
    border-radius: 2px;
    background: var(--gauge-marker);
    box-shadow: 0 0 0 2px var(--bg-panel);
    transition: left 220ms ease;
  }

  .scale {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    min-height: 32px;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  /* Bornes en dessous de la valeur : jamais de chevauchement. */
  .scale > span:first-child,
  .scale > span:last-child {
    padding-top: 15px;
  }

  .scale .val {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    color: var(--text-primary);
    font-weight: 600;
    font-size: var(--fs-sm);
    white-space: nowrap;
    transition: left 220ms ease;
  }
</style>
