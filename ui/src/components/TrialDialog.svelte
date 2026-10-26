<script lang="ts">
  // Noter un essai : contexte repris de la coupe en cours, valeurs réellement
  // utilisées (modifiables), résultat, constats, remarques. Un essai « Bon »
  // devient la référence testée pour ce contexte exact.
  import { X, Check, TriangleAlert, CircleX } from '@lucide/svelte';
  import NumberInput from './NumberInput.svelte';
  import Segmented from './Segmented.svelte';
  import { store } from '../lib/store.svelte';
  import { modeLabel, operationLabel, outcomes, rpmUnit, toolConditions, workholdingLabel } from '../lib/labels';
  import { t } from '../lib/i18n.svelte';
  import { fmtLength, lengthDecimals, lengthUnit, speedUnit, toDisplayLength, fromDisplayLength } from '../lib/format';
  import { tip } from '../lib/tip';
  import type { TrialInput, TrialOutcome } from '../lib/types';

  let { onClose }: { onClose: () => void } = $props();
  let dialog: HTMLDialogElement;

  const draft = store.draftTrial();
  let input = $state<TrialInput | null>(draft ? { ...draft, outcome: 'good' } : null);
  let outcome = $state<TrialOutcome | null>(null);
  let saving = $state(false);

  const u = store.units;
  const speedIn = (x: number) => (u === 'mm' ? x : Number((x / 25.4).toFixed(1)));
  const speedOut = (x: number | null) => (x == null ? 0 : u === 'mm' ? x : x * 25.4);
  const category = store.material?.category;
  const symptoms = (store.catalog?.symptoms ?? []).filter(
    (s) => !s.categories.length || (category && s.categories.includes(category)),
  );
  const ICON = { good: Check, adjust: TriangleAlert, failed: CircleX } as const;

  $effect(() => {
    dialog.showModal();
  });

  function toggle(id: string) {
    if (!input) return;
    input.symptoms = input.symptoms.includes(id) ? input.symptoms.filter((s) => s !== id) : [...input.symptoms, id];
  }

  async function save() {
    if (!input || !outcome) return;
    saving = true;
    const saved = await store.recordTrial({ ...$state.snapshot(input), outcome });
    saving = false;
    if (saved) dialog.close();
  }
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="trial-title">
  <header>
    <h2 id="trial-title">{t('readout.record')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={() => dialog.close()}><X size={16} /></button>
  </header>
  {#if input}
    <form
      class="body"
      onsubmit={(e) => {
        e.preventDefault();
        save();
      }}
    >
      <p class="ctx">
        <b>{store.material?.name}</b> {fmtLength(input.thickness_mm, u)} {lengthUnit(u)} · <b>{store.tool?.reference || store.tool?.name}</b> ·
        {operationLabel(input.operation)} · {modeLabel(input.aggressiveness)} ·
        {workholdingLabel(input.workholding).toLowerCase()} · {store.machine?.name}
      </p>

      <h3>{t('trialDialog.actual')}</h3>
      <div class="values">
        <label>
          <span>{t('readout.rpm')}</span>
          <NumberInput label={t('readout.rpm')} value={input.rpm} decimals={0} onchange={(v) => v && (input!.rpm = v)} />
          <em>{rpmUnit()}</em>
        </label>
        <label>
          <span>{t('readout.feed')}</span>
          <NumberInput label={t('readout.feed')} value={speedIn(input.feed_mm_min)} decimals={u === 'mm' ? 0 : 1} onchange={(v) => v && (input!.feed_mm_min = speedOut(v))} />
          <em>{speedUnit(u)}</em>
        </label>
        <label>
          <span>{t('readout.plunge')}</span>
          <NumberInput label={t('readout.plunge')} value={speedIn(input.plunge_mm_min)} decimals={u === 'mm' ? 0 : 1} onchange={(v) => v && (input!.plunge_mm_min = speedOut(v))} />
          <em>{speedUnit(u)}</em>
        </label>
        <label>
          <span>{t('assistant.pass')}</span>
          <NumberInput
            label={t('readout.doc')}
            value={Number(toDisplayLength(input.doc_mm, u).toFixed(3))}
            decimals={lengthDecimals(u)}
            onchange={(v) => v && (input!.doc_mm = fromDisplayLength(v, u))}
          />
          <em>{lengthUnit(u)}</em>
        </label>
      </div>

      <h3>{t('trialDialog.result')}</h3>
      <div class="outcomes" role="radiogroup" aria-label={t('trialDialog.resultAria')}>
        {#each outcomes() as o (o.id)}
          {@const I = ICON[o.id]}
          <button type="button" role="radio" aria-checked={outcome === o.id} class="outcome {o.id}" class:on={outcome === o.id} use:tip={o.hint} onclick={() => (outcome = o.id)}>
            <I size={15} />
            {o.label}
          </button>
        {/each}
      </div>
      {#if outcome === 'good'}
        <p class="hint">{t('trialDialog.goodHint')}</p>
      {/if}

      <h3>{t('trialDialog.observed')} <span class="opt">({t('common.optional')})</span></h3>
      <div class="chips">
        {#each symptoms as s (s.id)}
          <button type="button" class="chip" class:on={input.symptoms.includes(s.id)} aria-pressed={input.symptoms.includes(s.id)} onclick={() => toggle(s.id)}>{s.label}</button>
        {/each}
      </div>

      <h3>{t('trialDialog.toolCondition')} <span class="opt">({t('common.optional')})</span></h3>
      <Segmented
        label={t('trialDialog.toolCondition')}
        options={toolConditions()}
        value={input.tool_condition ?? 'unknown'}
        onchange={(v) => (input!.tool_condition = v === 'unknown' ? null : v)}
      />

      <h3>{t('trialDialog.notes')}</h3>
      <textarea class="input notes" rows="3" placeholder={t('trialDialog.notesPlaceholder')} bind:value={input.notes}></textarea>

      <div class="foot">
        <label class="op">
          <span>{t('trialDialog.operator')}</span>
          <input class="input" placeholder={t('trialDialog.operatorPlaceholder')} bind:value={input.operator} />
        </label>
        <span class="grow"></span>
        <button class="btn quiet" type="button" onclick={() => dialog.close()}>{t('common.cancel')}</button>
        <button class="btn primary" type="submit" disabled={!outcome || saving}>{t('trialDialog.save')}</button>
      </div>
    </form>
  {:else}
    <p class="body">{t('trialDialog.nothing')}</p>
  {/if}
</dialog>

<style>
  dialog {
    width: min(620px, 94vw);
    padding: 0;
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-pop);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s3) var(--s4);
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4) var(--s4);
    max-height: 78vh;
    overflow: auto;
  }

  .ctx {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }

  .ctx b {
    color: var(--text-primary);
    font-weight: 600;
  }

  h3 {
    margin: var(--s2) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .opt {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    color: var(--text-muted);
  }

  .values {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--s2);
  }

  .values label {
    display: grid;
    grid-template-columns: 1fr;
    gap: 3px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .values em {
    font-style: normal;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .outcomes {
    display: flex;
    gap: var(--s2);
  }

  .outcome {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-control);
    color: var(--text-secondary);
    font-weight: 600;
  }

  .outcome:hover {
    color: var(--text-primary);
  }

  .outcome.good.on {
    border-color: var(--success);
    color: var(--success);
    background: var(--success-tint);
  }

  .outcome.adjust.on {
    border-color: var(--warning);
    color: var(--warning);
    background: var(--warning-tint);
  }

  .outcome.failed.on {
    border-color: var(--danger);
    color: var(--danger);
    background: var(--danger-tint);
  }

  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--bg-control);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .chip:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .chip.on {
    border-color: var(--accent);
    background: var(--accent-tint);
    color: var(--text-primary);
  }

  .notes {
    height: auto;
    padding: 6px var(--s2);
    resize: vertical;
    font: inherit;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-top: var(--s2);
  }

  .op {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .op .input {
    width: 150px;
  }

  .grow {
    flex: 1;
  }

  @media (max-width: 560px) {
    .values {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .foot {
      flex-wrap: wrap;
    }
  }
</style>
