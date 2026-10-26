<script lang="ts">
  // Une valeur à reporter dans la FAO : grand chiffre, unité, origine.
  // Un clic sur la valeur permet de la modifier ; le moteur vérifie alors
  // la valeur saisie sans jamais la corriger. Le bouton de copie reprend le
  // nombre brut, à coller champ par champ dans la FAO.
  import { Info, RotateCcw, Copy, ArrowUpToLine } from '@lucide/svelte';
  import { tick, type Snippet } from 'svelte';
  import OriginTag from './OriginTag.svelte';
  import NumberInput from './NumberInput.svelte';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import type { Origin } from '../lib/types';

  let {
    label,
    hint,
    display,
    unit,
    secondary,
    details,
    copyText,
    onCopy,
    preview = null,
    limit = null,
    origin,
    validated = false,
    editValue,
    editDecimals = 0,
    onEdit,
    onReset,
  }: {
    label: string;
    hint: string;
    display: string;
    unit: string;
    secondary?: string;
    /** Ligne secondaire enrichie (remplace `secondary`). */
    details?: Snippet;
    /** Nombre copié tel quel (sans séparateur de milliers). */
    copyText: string;
    onCopy: (text: string) => void;
    /** Valeur que donnerait le levier survolé. */
    preview?: string | null;
    /** Ce qui plafonne cette valeur, s'il y a lieu. */
    limit?: string | null;
    origin: Origin;
    validated?: boolean;
    editValue: number;
    editDecimals?: number;
    onEdit: (v: number | null) => void;
    onReset: () => void;
  } = $props();

  let editing = $state(false);
  let host: HTMLDivElement;

  async function startEdit() {
    editing = true;
    await tick();
    host.querySelector('input')?.focus();
  }
</script>

<div class="cell" class:override={origin === 'user_override'}>
  <div class="top">
    <span class="label">{label}</span>
    <span class="i" use:tip={hint}><Info size={11} /></span>
    <span class="grow"></span>
    <button class="mini copy" type="button" aria-label={t('readout.copyValue', { label })} use:tip={t('readout.copyValue', { label })} onclick={() => onCopy(copyText)}>
      <Copy size={12} />
    </button>
    {#if origin === 'user_override'}
      <button class="mini" type="button" use:tip={t('readout.reset')} onclick={onReset}>
        <RotateCcw size={12} />
      </button>
    {/if}
    <OriginTag {origin} {validated} />
  </div>
  <div class="main" bind:this={host}>
    {#if editing}
      <NumberInput
        label={label}
        value={editValue}
        decimals={editDecimals}
        width="110px"
        onchange={(v) => {
          onEdit(v);
          editing = false;
        }}
      />
      <span class="unit">{unit}</span>
      <button class="btn quiet" type="button" onclick={() => (editing = false)}>OK</button>
    {:else}
      <button class="value tabular" type="button" use:tip={t('readout.edit')} onclick={startEdit}>
        {display}
      </button>
      <span class="unit">{unit}</span>
      {#if preview != null && preview !== display}
        <span class="preview tabular" aria-live="polite">→ {preview}</span>
      {/if}
    {/if}
  </div>
  {#if details}<div class="sec">{@render details()}</div>{:else if secondary}<div class="sec">{secondary}</div>{/if}
  {#if limit}<div class="limit"><ArrowUpToLine size={11} /> {t('readout.cappedBy', { what: limit })}</div>{/if}
</div>

<style>
  .cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s2) var(--s3);
    min-width: 0;
    container: cell / inline-size;
    border-right: 1px solid var(--divider);
  }

  .cell:last-child {
    border-right: 0;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 18px;
  }

  .label {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
    line-height: 1.25;
    min-width: 0;
  }

  .top > :global(.tag) {
    flex: none;
  }

  /* Cellule étroite : l'étiquette d'origine passe en icône seule (libellé dans l'infobulle). */
  @container cell (max-width: 290px) {
    .top :global(.tag-text) {
      display: none;
    }
  }

  .i {
    flex: none;
    display: inline-flex;
    color: var(--text-muted);
    cursor: help;
  }

  .grow {
    flex: 1;
  }

  .mini {
    flex: none;
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--text-muted);
  }

  .mini:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Copie discrète : visible au survol de la carte ou au clavier. */
  .copy {
    opacity: 0;
  }

  .cell:hover .copy,
  .copy:focus-visible {
    opacity: 1;
  }

  @media (hover: none) {
    .copy {
      opacity: 1;
    }
  }

  .preview {
    color: var(--success);
    font-size: var(--fs-md);
    font-weight: 600;
    white-space: nowrap;
  }

  .limit {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 500;
  }

  .limit :global(svg) {
    flex: none;
    color: var(--warning);
  }

  .main {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-height: 36px;
  }

  .value {
    padding: 0 2px;
    margin-left: -2px;
    border: 0;
    border-radius: 3px;
    background: none;
    font-size: 28px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 36px;
    white-space: nowrap;
    cursor: text;
  }

  .value:hover {
    background: var(--bg-hover);
  }

  .override .value {
    color: var(--origin-user);
  }

  .unit {
    color: var(--text-secondary);
    font-size: var(--fs-md);
  }

  /* Sur deux lignes si besoin plutôt que tronqué : l'avance par dent reste lisible. */
  .sec {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.35;
  }
</style>
