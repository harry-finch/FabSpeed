<script lang="ts">
  // Valider un réglage testé (administrateur) : rappel de l'essai et de tous
  // les essais du même contexte avant de confirmer. Le réglage validé devient
  // la référence du contexte, avant les essais plus récents.
  import { X, ShieldCheck, Check, TriangleAlert, CircleX } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { dateTime, modeLabel, operationLabel, outcomeLabel, rpmUnit } from '../lib/labels';
  import { fmtLength, fmtSpeed, lengthUnit, num, speedUnit } from '../lib/format';

  let { id, onClose }: { id: string; onClose: () => void } = $props();
  let dialog: HTMLDialogElement;

  const trial = $derived(store.catalog?.trials.find((x) => x.id === id));
  const others = $derived(trial ? store.contextTrials(trial) : []);
  const newerProblem = $derived(
    trial ? others.find((x) => x.outcome !== 'good' && x.recorded_at > trial.recorded_at) : undefined,
  );
  const material = $derived(store.catalog?.materials.find((m) => m.id === trial?.material_id));
  const machine = $derived(store.catalog?.machines.find((m) => m.id === trial?.machine_id));
  const u = $derived(store.units);
  const ICON = { good: Check, adjust: TriangleAlert, failed: CircleX } as const;

  let by = $state(store.operator);
  let note = $state('');
  let saving = $state(false);

  $effect(() => {
    dialog.showModal();
  });

  async function save() {
    if (!trial || saving) return;
    saving = true;
    const ok = await store.validateTrial(trial.id, by, note);
    saving = false;
    if (ok) dialog.close();
  }
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="validate-title">
  <header>
    <h2 id="validate-title"><ShieldCheck size={16} /> {t('validate.title')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={() => dialog.close()}><X size={16} /></button>
  </header>
  {#if trial}
    <form
      class="body"
      onsubmit={(e) => {
        e.preventDefault();
        save();
      }}
    >
      <p class="intro">{t('validate.intro')}</p>
      <p class="ctx">
        <b>{material?.name ?? trial.material_id}</b> · <b>{trial.tool_name.split('·')[0]}</b> · {operationLabel(trial.operation)} ·
        {modeLabel(trial.aggressiveness)} · {machine?.name ?? trial.machine_id}
      </p>
      <p class="vals tabular">
        {num(trial.rpm, 0)} {rpmUnit()} · {fmtSpeed(trial.feed_mm_min, u)} {speedUnit(u)} · {t('readout.plunge')} {fmtSpeed(trial.plunge_mm_min, u)} ·
        {t('trials.pass')} {fmtLength(trial.doc_mm, u)} {lengthUnit(u)}
      </p>

      <h3>{t('validate.contextTrials', { n: others.length })}</h3>
      <ul class="list">
        {#each others as x (x.id)}
          {@const I = ICON[x.outcome]}
          <li class:current={x.id === trial.id}>
            <span class="out {x.outcome}"><I size={13} /> {outcomeLabel(x.outcome)}</span>
            {#if x.validation}<span class="badge"><ShieldCheck size={11} /> {t('evidence.validated.label')}</span>{/if}
            <span class="v tabular">{num(x.rpm, 0)} · {fmtSpeed(x.feed_mm_min, u)} · {fmtLength(x.doc_mm, u)}</span>
            <span class="d">{dateTime(x.recorded_at)}{x.operator ? ` · ${x.operator}` : ''}{x.id === trial.id ? ` · ${t('validate.thisOne')}` : ''}</span>
          </li>
        {/each}
      </ul>
      {#if newerProblem}
        <p class="warn"><TriangleAlert size={13} /> {t('validate.newerProblem', { date: dateTime(newerProblem.recorded_at), outcome: outcomeLabel(newerProblem.outcome) })}</p>
      {/if}

      <label>
        <span>{t('validate.by')}</span>
        <input class="input" bind:value={by} placeholder={t('trialDialog.operatorPlaceholder')} />
      </label>
      <label>
        <span>{t('validate.note')} <span class="opt">({t('common.optional')})</span></span>
        <textarea class="input" rows="2" bind:value={note} placeholder={t('validate.notePlaceholder')}></textarea>
      </label>

      <div class="foot">
        <button class="btn quiet" type="button" onclick={() => dialog.close()}>{t('common.cancel')}</button>
        <button class="btn primary" type="submit" disabled={saving}><ShieldCheck size={14} /> {t('validate.submit')}</button>
      </div>
    </form>
  {:else}
    <p class="body">{t('trialDetail.gone')}</p>
  {/if}
</dialog>

<style>
  dialog {
    width: min(560px, 94vw);
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
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin: 0;
    font-size: var(--fs-lg);
  }

  h3 {
    margin: var(--s2) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4) var(--s4);
    max-height: 80vh;
    overflow: auto;
  }

  .intro,
  .ctx {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }

  .ctx b {
    color: var(--text-primary);
  }

  .vals {
    margin: 0;
    font-size: var(--fs-md, 14px);
    font-weight: 600;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: var(--fs-sm);
  }

  .list li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px var(--s2);
    padding: 5px var(--s2);
    border-radius: var(--radius);
  }

  .list li.current {
    background: var(--bg-selected);
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: 4px;
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

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--success);
    font-size: var(--fs-xs);
    font-weight: 600;
  }

  .v {
    color: var(--text-primary);
  }

  .d {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .warn {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: var(--s2);
    border-radius: var(--radius);
    background: var(--warning-tint);
    color: var(--warning);
    font-size: var(--fs-sm);
  }

  label {
    display: grid;
    gap: 4px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .opt {
    color: var(--text-muted);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    margin-top: var(--s1);
  }
</style>
