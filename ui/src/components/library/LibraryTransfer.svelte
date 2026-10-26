<script lang="ts">
  import { Upload, Download, X, Lock } from '@lucide/svelte';
  import { store } from '../../lib/store.svelte';
  import { t } from '../../lib/i18n.svelte';
  import type { ImportReport, LibraryKind } from '../../lib/api';

  let { kind, controls = true }: { kind?: LibraryKind; controls?: boolean } = $props();
  let fileInput: HTMLInputElement;
  let dialog = $state<HTMLDialogElement>();
  let pending = $state<{ text: string; name: string; kind?: LibraryKind; report: ImportReport } | null>(null);
  let error = $state('');
  let busy = $state(false);

  export async function pick() {
    if (await store.ensureAdmin()) fileInput.click();
  }
  async function onFile(file: File) {
    busy = true;
    error = '';
    try {
      if (file.size > 50 * 1024 * 1024) throw new Error(t('transfer.tooLarge'));
      const selectedKind = kind;
      const text = await file.text();
      const report = await store.importLibrary(text, selectedKind, true);
      if (typeof report === 'string') error = report;
      else if (report) {
        pending = { text, name: file.name, kind: selectedKind, report };
        queueMicrotask(() => dialog?.showModal());
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  async function confirm() {
    if (!pending || busy) return;
    busy = true;
    error = '';
    const report = await store.importLibrary(pending.text, pending.kind);
    busy = false;
    if (typeof report === 'string') error = report;
    else if (report) dialog?.close();
  }
</script>

<input bind:this={fileInput} type="file" accept=".json,.toml,application/json" hidden onchange={async (e) => {
  const file = e.currentTarget.files?.[0];
  e.currentTarget.value = '';
  if (file) await onFile(file);
}} />
{#if controls}
<div class="transfer">
  <button class="btn quiet" type="button" disabled={busy} onclick={pick}>
    <Upload size={14} /> {t('transfer.import')} {#if !store.auth.admin}<Lock size={12} />{/if}
  </button>
  <button class="btn quiet" type="button" disabled={busy} onclick={() => store.exportLibrary(kind)}>
    <Download size={14} /> {t('transfer.export')}
  </button>
</div>
{/if}
{#if error && !pending}<p class="error" role="alert">{error}</p>{/if}

{#if pending}
  <dialog bind:this={dialog} onclose={() => { pending = null; error = ''; }} oncancel={(e) => { if (busy) e.preventDefault(); }} aria-labelledby="import-title">
    <header>
      <h2 id="import-title">{t('transfer.title')}</h2>
      <button class="btn quiet icon" type="button" disabled={busy} aria-label={t('common.close')} onclick={() => dialog?.close()}><X size={16} /></button>
    </header>
    <div class="body">
      <p class="file">{pending.name}</p>
      <p>{t('transfer.hint')}</p>
      <p>{t('transfer.summary', { added: pending.report.added, skipped: pending.report.skipped })}</p>
      <div class="records">
        <table>
          <thead><tr><th>{t('transfer.record')}</th><th>{t('transfer.action')}</th></tr></thead>
          <tbody>
            {#each pending.report.records as record, i (i)}
              <tr>
                <td><span>{record.name}</span><small>{t(`library.section.${record.kind}`)} · {record.id}</small></td>
                <td title={record.reason}>{t(record.action === 'add' ? 'transfer.add' : 'transfer.skip')}
                  {#if record.reason}<small>{record.reason}</small>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if !pending.report.records.length}<p>{t('transfer.empty')}</p>{/if}
      {#if pending.report.issues.length}<ul>{#each pending.report.issues as issue (issue)}<li>{issue}</li>{/each}</ul>{/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <footer>
        <button class="btn quiet" type="button" disabled={busy} onclick={() => dialog?.close()}>{t('common.cancel')}</button>
        <button class="btn primary" type="button" disabled={busy || !pending.report.added} onclick={confirm}>{t('transfer.confirm', { n: pending.report.added })}</button>
      </footer>
    </div>
  </dialog>
{/if}

<style>
  .transfer { display: flex; align-items: center; gap: var(--s1); flex-wrap: wrap; }
  dialog { width: min(680px, 94vw); max-height: 90vh; padding: 0; border: 1px solid var(--border); border-radius: var(--radius-lg); background: var(--bg-elevated); color: var(--text-primary); box-shadow: var(--shadow-pop); }
  dialog::backdrop { background: var(--backdrop); }
  header { display: flex; align-items: center; justify-content: space-between; padding: var(--s3); border-bottom: 1px solid var(--border); }
  h2 { font-size: var(--fs-lg); margin: 0; }
  .body { padding: var(--s3); display: flex; flex-direction: column; gap: var(--s2); }
  p { margin: 0; font-size: var(--fs-sm); }
  .file { font-weight: 600; overflow-wrap: anywhere; }
  .records { overflow: auto; max-height: 45vh; }
  table { border-collapse: collapse; width: 100%; font-size: var(--fs-sm); }
  th, td { text-align: left; vertical-align: top; padding: var(--s2); border-bottom: 1px solid var(--border); overflow-wrap: anywhere; }
  small { display: block; color: var(--text-secondary); font-size: var(--fs-xs); }
  footer { display: flex; justify-content: flex-end; gap: var(--s2); }
  .error { color: var(--danger); }
</style>
