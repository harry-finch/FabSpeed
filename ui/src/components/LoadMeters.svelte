<script lang="ts">
  // Charges estimées : part des limites physiques utilisée. Estimations
  // relatives (profils machine marqués « à vérifier »), pas des mesures.
  import { Info } from '@lucide/svelte';
  import { num, pct } from '../lib/format';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import type { Metrics } from '../lib/types';

  let { metrics }: { metrics: Metrics } = $props();

  const rows = $derived([
    {
      label: t('loads.spindle'),
      v: metrics.loads.spindle_pct,
      hint: t('loads.spindleHint', { p: num(metrics.cutting_power_w, 0), a: num(metrics.available_power_w, 0) }),
    },
    {
      label: t('loads.machine'),
      v: metrics.loads.machine_pct,
      hint: t('loads.machineHint', { f: num(metrics.cutting_force_n, 1) }),
    },
    {
      label: t('loads.tool'),
      v: metrics.loads.tool_pct,
      hint: t('loads.toolHint'),
    },
    {
      label: t('loads.holding'),
      v: metrics.loads.holding_pct,
      hint: t('loads.holdingHint'),
    },
  ]);

  const level = (v: number) => (v > 100 ? 'over' : v > 75 ? 'high' : 'ok');
</script>

<div class="loads">
  <div class="head">
    <span class="title">{t('loads.title')}</span>
    <span class="i" use:tip={t('loads.hint')}
      ><Info size={11} /></span
    >
  </div>
  {#each rows as r (r.label)}
    <div class="row" use:tip={r.hint}>
      <span class="lbl">{r.label}</span>
      <div class="track" role="meter" aria-label={t('loads.aria', { name: r.label })} aria-valuenow={r.v} aria-valuemin={0} aria-valuemax={100}>
        <div class="fill {level(r.v)}" style:width={`${Math.min(r.v, 100)}%`}></div>
      </div>
      <span class="pct tabular {level(r.v)}">{pct(r.v)}</span>
    </div>
  {/each}
</div>

<style>
  .loads {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 2px;
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

  .row {
    display: grid;
    grid-template-columns: 58px 1fr 40px;
    align-items: center;
    gap: var(--s2);
  }

  .lbl {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .track {
    height: 6px;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: 3px;
    transition: width 220ms ease;
  }

  .fill.ok {
    background: var(--meter-neutral);
  }

  .fill.high {
    background: var(--warning);
  }

  .fill.over {
    background: var(--danger);
  }

  .pct {
    text-align: right;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .pct.high {
    color: var(--warning);
  }

  .pct.over {
    color: var(--danger);
    font-weight: 600;
  }
</style>
