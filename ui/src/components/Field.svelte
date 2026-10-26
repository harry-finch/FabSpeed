<script lang="ts">
  // Ligne de propriété : LIBELLÉ | VALEUR | UNITÉ, colonnes alignées.
  import { Info } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { tip } from '../lib/tip';

  let {
    label,
    hint,
    unit,
    forId,
    children,
  }: { label: string; hint?: string; unit?: string; forId?: string; children: Snippet } = $props();
</script>

<div class="field">
  <label class="label" for={forId}>
    {label}
    {#if hint}
      <span class="hint" use:tip={hint} tabindex="-1"><Info size={11} /></span>
    {/if}
  </label>
  <div class="value">{@render children()}</div>
  {#if unit !== undefined}<span class="unit">{unit}</span>{/if}
</div>

<style>
  .field {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--s2);
    min-height: var(--control-h);
  }

  .label {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .hint {
    display: inline-flex;
    color: var(--text-muted);
    cursor: help;
  }

  .hint:hover {
    color: var(--text-primary);
  }

  .value {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
  }

  .unit {
    min-width: 42px;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
</style>
