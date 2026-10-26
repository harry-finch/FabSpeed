<script lang="ts">
  // « Base du lab » : contenu de la base, export des essais (tous),
  // sauvegarde et restauration (administrateur). La restauration passe
  // toujours par une vérification affichée avant confirmation.
  import { Download, Upload, FileSpreadsheet, Lock, TriangleAlert, X, ShieldCheck } from '@lucide/svelte';
  import CatalogueSharing from './CatalogueSharing.svelte';
  import LibraryTransfer from './LibraryTransfer.svelte';
  import { store } from '../../lib/store.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { dateTime } from '../../lib/labels';
  import type { LabCounts, RestoreCheck } from '../../lib/api';

  let fileInput: HTMLInputElement;
  let dialog = $state<HTMLDialogElement>();
  let pending = $state<{ text: string; name: string; check: RestoreCheck } | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const c = $derived(store.catalog);
  const counts = $derived<LabCounts>({
    machines: c?.machines.length ?? 0,
    materials: c?.materials.length ?? 0,
    tools: c?.tools.length ?? 0,
    lab_records: (c?.layers.lab_tools.length ?? 0) + (c?.layers.lab_machines.length ?? 0) + (c?.layers.lab_materials?.length ?? 0),
    trials: c?.trials.length ?? 0,
    validated: c?.trials.filter((x) => x.validation).length ?? 0,
    archived: 0,
  });
  const ROWS = ['machines', 'materials', 'tools', 'lab_records', 'trials', 'validated'] as const;

  async function pick() {
    if (await store.ensureAdmin()) fileInput.click();
  }

  async function onFile(file: File) {
    error = null;
    const text = await file.text();
    busy = true;
    const r = await store.checkRestore(text);
    busy = false;
    if (r === null) return;
    if (typeof r === 'string') {
      error = r;
      return;
    }
    pending = { text, name: file.name, check: r };
    queueMicrotask(() => dialog?.showModal());
  }

  async function confirm() {
    if (!pending || busy) return;
    busy = true;
    const ok = await store.restore(pending.text);
    busy = false;
    if (ok) dialog?.close();
  }
</script>

<input
  bind:this={fileInput}
  type="file"
  accept=".json,application/json"
  hidden
  onchange={async (e) => {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = '';
    if (f) await onFile(f);
  }}
/>

<section class="data" aria-label={t('library.section.data')}>
  <h2>{t('data.title')}</h2>
  <p class="intro">{t('data.intro')}</p>

  <CatalogueSharing />

  <div class="cards">
    <article class="card">
      <h3><Upload size={14} /> {t('transfer.title')}</h3>
      <p class="hint">{t('transfer.formats')}</p>
      <p class="hint">{t('transfer.hint')}</p>
      <LibraryTransfer />
    </article>
    <article class="card">
      <h3>{t('data.content')}</h3>
      <dl class="kv tabular">
        {#each ROWS as k (k)}<div><dt>{t(`data.count.${k}`)}</dt><dd>{counts[k]}</dd></div>{/each}
      </dl>
      <p class="hint">{t('data.where')} <code>data/</code></p>
    </article>

    <article class="card">
      <h3><FileSpreadsheet size={14} /> {t('data.export')}</h3>
      <p class="hint">{t('data.exportHint')}</p>
      <button class="btn" type="button" disabled={!counts.trials} onclick={() => store.exportTrialsCsv()}>
        <Download size={14} /> {t('data.exportButton')}
      </button>
    </article>

    <article class="card">
      <h3><Download size={14} /> {t('data.backup')}</h3>
      <p class="hint">{t('data.backupHint')}</p>
      <button class="btn" type="button" onclick={() => store.downloadBackup()}>
        <Download size={14} /> {t('data.backupButton')} {#if !store.auth.admin}<Lock size={12} />{/if}
      </button>
    </article>

    <article class="card">
      <h3><Upload size={14} /> {t('data.restore')}</h3>
      <p class="hint">{t('data.restoreHint')}</p>
      <button class="btn" type="button" disabled={busy} onclick={pick}>
        <Upload size={14} /> {t('data.restoreButton')} {#if !store.auth.admin}<Lock size={12} />{/if}
      </button>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </article>
  </div>
</section>

{#if pending}
  <dialog bind:this={dialog} onclose={() => (pending = null)} aria-labelledby="restore-title">
    <header>
      <h2 id="restore-title"><ShieldCheck size={16} /> {t('data.checkTitle')}</h2>
      <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={() => dialog?.close()}><X size={16} /></button>
    </header>
    <div class="body">
      <p class="ok">{t('data.checkOk', { name: pending.name })}</p>
      <p class="meta">
        {t('data.createdAt', { date: dateTime(pending.check.created_at) })} · FabSpeed {pending.check.engine_version || '—'} ·
        {t('trialDetail.data', { v: pending.check.dataset_revision || '—' })}
      </p>
      <table class="cmp tabular">
        <thead><tr><th></th><th>{t('data.currentBase')}</th><th>{t('data.backupFile')}</th></tr></thead>
        <tbody>
          {#each [...ROWS, 'archived'] as const as k (k)}
            <tr><td>{t(`data.count.${k}`)}</td><td>{pending.check.current[k]}</td><td>{pending.check.backup[k]}</td></tr>
          {/each}
        </tbody>
      </table>
      {#if pending.check.issues.length}
        <ul class="issues">{#each pending.check.issues as i (i)}<li>{i}</li>{/each}</ul>
      {/if}
      {#if !pending.check.full}<p class="meta">{t('data.labOnly')}</p>{/if}
      <p class="warn"><TriangleAlert size={14} /> {pending.check.full ? t('data.replaceWarning') : t('data.replaceWarningLab')}</p>
      <div class="foot">
        <button class="btn quiet" type="button" onclick={() => dialog?.close()}>{t('common.cancel')}</button>
        <button class="btn primary" type="button" disabled={busy} onclick={confirm}>{t('data.restoreConfirm')}</button>
      </div>
    </div>
  </dialog>
{/if}

<style>
  .data {
    grid-column: 2 / -1;
    padding: var(--s4);
    overflow: auto;
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  .intro {
    margin: var(--s1) 0 var(--s3);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    max-width: 70ch;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--s3);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-panel);
  }

  .card .btn {
    align-self: flex-start;
    margin-top: auto;
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .kv {
    margin: 0;
    display: grid;
    gap: 2px;
    font-size: var(--fs-sm);
  }

  .kv div {
    display: flex;
    justify-content: space-between;
  }

  .kv dt {
    color: var(--text-secondary);
  }

  .kv dd {
    margin: 0;
    font-weight: 600;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: var(--fs-sm);
  }

  dialog {
    width: min(520px, 94vw);
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

  dialog header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s3) var(--s4);
    border-bottom: 1px solid var(--border);
  }

  dialog h2 {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4) var(--s4);
  }

  .ok {
    margin: 0;
    color: var(--success);
    font-size: var(--fs-sm);
    font-weight: 600;
  }

  .meta {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .cmp {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-sm);
  }

  .cmp th,
  .cmp td {
    padding: 4px 6px;
    text-align: right;
    border-bottom: 1px solid var(--divider);
  }

  .cmp th:first-child,
  .cmp td:first-child {
    text-align: left;
    color: var(--text-secondary);
  }

  .issues {
    margin: 0;
    padding-left: 1.2em;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }

  .warn {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0;
    padding: var(--s2);
    border-radius: var(--radius);
    background: var(--warning-tint);
    color: var(--warning);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    margin-top: var(--s1);
  }
</style>
