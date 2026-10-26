<script lang="ts">
  // Saisie numérique selon la langue (virgule décimale en français, point en
  // anglais ; en anglais la virgule est un séparateur de milliers), validée à
  // la sortie du champ ou sur Entrée. `value = null` affiche le `placeholder` (valeur auto).
  // Flèches haut/bas : ± step (Maj : × 10).
  import { i18n, intlLocale } from '../lib/i18n.svelte';

  let {
    value,
    onchange,
    placeholder = '',
    step = 1,
    min = 0,
    decimals = 2,
    id,
    label,
    width = '100%',
    invalid = false,
  }: {
    value: number | null;
    onchange: (v: number | null) => void;
    placeholder?: string;
    step?: number;
    min?: number;
    decimals?: number;
    id?: string;
    label?: string;
    width?: string;
    invalid?: boolean;
  } = $props();

  const show = (v: number | null) =>
    v == null ? '' : new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: decimals, useGrouping: false }).format(v);

  let text = $state('');
  let focused = $state(false);
  $effect(() => {
    if (!focused) text = show(value);
  });

  function parse(raw: string): number | null | undefined {
    const compact = raw.trim().replace(/\s/g, '');
    const t = i18n.locale === 'fr' ? compact.replace(',', '.') : compact.replace(/,/g, '');
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) && n > min ? n : undefined;
  }

  let bad = $derived(focused && parse(text) === undefined);

  function commit() {
    const v = parse(text);
    if (v === undefined) {
      text = show(value);
      return;
    }
    if (v !== value) onchange(v);
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
    if (e.key === 'Escape') {
      text = show(value);
      (e.currentTarget as HTMLInputElement).blur();
    }
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const base = parse(text) ?? value ?? parse(placeholder) ?? 0;
      const delta = (e.key === 'ArrowUp' ? 1 : -1) * step * (e.shiftKey ? 10 : 1);
      const next = Math.max(step, Math.round((base + delta) / step) * step);
      text = show(next);
      onchange(next);
    }
  }
</script>

<input
  {id}
  class="input tabular"
  class:bad={bad || invalid}
  style:width
  inputmode="decimal"
  aria-label={label}
  {placeholder}
  bind:value={text}
  onfocus={(e) => {
    focused = true;
    (e.currentTarget as HTMLInputElement).select();
  }}
  onblur={() => {
    focused = false;
    commit();
  }}
  onkeydown={key}
/>

<style>
  input {
    text-align: right;
  }

  input::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  .bad {
    border-color: var(--danger) !important;
  }
</style>
