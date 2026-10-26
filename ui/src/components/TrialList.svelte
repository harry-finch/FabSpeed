<script lang="ts">
  // Liste compacte d'essais : date, résultat (icône + texte), valeurs, constats.
  // « Reprendre » applique les valeurs de l'essai comme saisie manuelle.
  import { Check, TriangleAlert, CircleX, Undo2, Archive, ShieldCheck, ShieldOff, Lock } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { dateTime, outcomeLabel, rpmUnit } from '../lib/labels';
  import { fmtLength, fmtSpeed, lengthUnit, num, pct, speedUnit } from '../lib/format';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import type { Trial } from '../lib/types';

  let { trials, showContext = false }: { trials: Trial[]; showContext?: boolean } = $props();

  let confirming = $state<string | null>(null);
  const u = $derived(store.units);
  const ICON = { good: Check, adjust: TriangleAlert, failed: CircleX } as const;
  const validatedHint = (x: Trial) =>
    t('validate.byOn', { date: dateTime(x.validation!.validated_at), by: x.validation!.validated_by || '—' }) +
    (x.validation!.note ? `\n${x.validation!.note}` : '');
  const symptomLabel = (id: string) => store.catalog?.symptoms.find((s) => s.id === id)?.label ?? id;
  const materialName = (id: string) => store.catalog?.materials.find((m) => m.id === id)?.name ?? id;

  function reuse(trial: Trial) {
    store.applyPatch(
      { overrides: { rpm: trial.rpm, feed_mm_min: trial.feed_mm_min, plunge_mm_min: trial.plunge_mm_min, doc_mm: trial.doc_mm } },
      t('trials.reused'),
    );
  }
</script>

<ul class="list">
  {#each trials as tr (tr.id)}
    {@const I = ICON[tr.outcome]}
    <li class="trial">
      <div class="top">
        <span class="out {tr.outcome}"><I size={13} /> {outcomeLabel(tr.outcome)}</span>
        {#if tr.validation}<span class="validated" use:tip={validatedHint(tr)}><ShieldCheck size={12} /> {t('evidence.validated.label')}</span>{/if}
        <span class="date">{dateTime(tr.recorded_at)}{tr.operator ? ` · ${tr.operator}` : ''}</span>
      </div>
      {#if showContext}
        <div class="ctx">{materialName(tr.material_id)} · {tr.tool_name}</div>
      {/if}
      <div class="vals tabular">
        {num(tr.rpm, 0)} {rpmUnit()} · {fmtSpeed(tr.feed_mm_min, u)} {speedUnit(u)} · {t('trials.pass')} {fmtLength(tr.doc_mm, u)} {lengthUnit(u)}
        {#if tr.stepover_ratio < 1}· {t('trials.stepover')} {pct(tr.stepover_ratio * 100)}{/if}
      </div>
      {#if tr.symptoms.length}
        <div class="sym">{tr.symptoms.map(symptomLabel).join(' · ')}</div>
      {/if}
      {#if tr.notes}<p class="notes">{tr.notes}</p>{/if}
      <div class="acts">
        {#if confirming === tr.id}
          <span class="ask">{t('trials.archiveAsk')}</span>
          <button class="btn quiet" type="button" onclick={() => (confirming = null)}>{t('common.no')}</button>
          <button
            class="btn"
            type="button"
            onclick={() => {
              confirming = null;
              store.archiveTrial(tr.id);
            }}>{t('common.yesArchive')}</button
          >
        {:else}
          {#if !showContext}
            <button class="btn quiet" type="button" use:tip={t('trials.reuseHint')} onclick={() => reuse(tr)}>
              <Undo2 size={13} /> {t('trials.reuse')}
            </button>
          {/if}
          {#if tr.outcome === 'good' && !tr.validation}
            <button class="btn quiet" type="button" use:tip={t('validate.hint')} onclick={() => store.openValidateDialog(tr.id)}>
              <ShieldCheck size={13} /> {t('validate.action')} {#if !store.auth.admin}<Lock size={11} />{/if}
            </button>
          {:else if tr.validation}
            <button class="btn quiet" type="button" use:tip={t('validate.removeHint')} onclick={() => store.unvalidateTrial(tr.id)}>
              <ShieldOff size={13} /> {t('validate.remove')}
            </button>
          {/if}
          <button class="btn quiet" type="button" use:tip={t('trials.archiveHint')} onclick={() => (confirming = tr.id)}>
            <Archive size={13} /> {t('common.archive')}
          </button>
        {/if}
      </div>
    </li>
  {/each}
</ul>

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .trial {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: var(--s2) 0;
    border-bottom: 1px solid var(--divider);
    font-size: var(--fs-sm);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--s2);
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

  .validated {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--success);
    font-size: var(--fs-xs);
    font-weight: 600;
  }

  .date {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .ctx {
    color: var(--text-secondary);
  }

  .vals {
    color: var(--text-primary);
  }

  .sym {
    color: var(--warning);
  }

  .notes {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .acts {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: -8px;
  }

  .acts .btn {
    height: 24px;
  }

  .ask {
    padding: 0 var(--s2);
    color: var(--text-secondary);
  }
</style>
