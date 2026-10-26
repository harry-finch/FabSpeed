<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Share2, Lock, X } from '@lucide/svelte';
  import { api, type SharingResponse, type SharingSettings, type SharingKind, type SharingChoice, type SharingPreview } from '../../lib/api';
  import { store } from '../../lib/store.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { dateTime } from '../../lib/labels';
  let dialog = $state<HTMLDialogElement>();
  let response = $state<SharingResponse | null>(null);
  let draft = $state<SharingSettings>({ destination: '', consent: { enabled: false, machines: [], tools: [], materials: [], trials: [] } });
  let preview = $state<SharingPreview | null>(null);
  let busy = $state(false);
  let error = $state('');
  let dirty = $state(false);
  let timer: ReturnType<typeof setInterval> | undefined;
  let polling = false;
  let expanded = $state<Record<SharingKind, boolean>>({machines:true,tools:false,materials:false,trials:false});
  const kinds: SharingKind[] = ['machines', 'tools', 'materials', 'trials'];
  const status = $derived(response?.status);
  async function refresh() {
    if (polling) return;
    polling = true;
    try { response = await api.sharing(); } catch { error = t('sharing.errorHelp'); }
    finally { polling = false; }
  }
  async function open() {
    if (!(await store.ensureAdmin())) return;
    error = ''; busy = true;
    await refresh();
    if (response) {
      draft = $state.snapshot(response.status.settings);
      draft.destination ??= response.status.default_destination ?? '';
      dirty = false; preview = null;
      dialog?.showModal();
      timer = setInterval(refresh, 1800);
    }
    busy = false;
  }
  function close() { clearInterval(timer); timer = undefined; }
  onDestroy(close);
  function change() { dirty = true; preview = null; }
  function select(kind: SharingKind, id: string, checked: boolean) {
    draft.consent[kind] = checked ? [...draft.consent[kind], id] : draft.consent[kind].filter(x => x !== id);
    change();
  }
  function missing(c: SharingChoice) {
    return !draft.consent.machines.includes(c.machine ?? '') || !draft.consent.tools.includes(c.tool ?? '') || !draft.consent.materials.includes(c.material ?? '');
  }
  function dependencyNames(c: SharingChoice) {
    return [['machines', c.machine], ['tools', c.tool], ['materials', c.material]].map(([kind, id]) => response?.choices[kind as SharingKind].find(r => r.id === id)?.name ?? id).join(' · ');
  }
  function dependencies(c: SharingChoice) {
    for (const [kind, id] of [['machines', c.machine], ['tools', c.tool], ['materials', c.material]] as const) {
      if (id && !draft.consent[kind].includes(id)) draft.consent[kind] = [...draft.consent[kind], id];
    }
    change();
  }
  async function action(kind: 'save' | 'preview' | 'publish') {
    if (busy) return;
    if (!(await store.ensureAdmin())) return;
    busy = true; error = '';
    try {
      const settings = { ...draft, destination: draft.destination?.trim() || null };
      if (kind === 'preview') preview = await api.previewSharing(settings);
      else {
        if (kind === 'save') { await api.saveSharing(settings); dirty = false; }
        else await api.publishSharing();
        await refresh();
      }
    } catch (e) { error = e instanceof Error ? e.message : t('sharing.errorHelp'); await refresh(); }
    finally { busy = false; }
  }
</script>

<section class="sharing-entry" aria-label={t('sharing.title')}>
  <div><h3><Share2 size={14} /> {t('sharing.title')}</h3><p>{t('sharing.intro')}</p></div>
  <button class="btn" type="button" disabled={busy} onclick={open}>{t('sharing.open')} {#if !store.auth.admin}<Lock size={12} />{/if}</button>
  {#if error && !dialog?.open}<p class="error" role="alert">{error}</p>{/if}
</section>
<dialog bind:this={dialog} onclose={close} aria-labelledby="sharing-title">
  <header><h2 id="sharing-title">{t('sharing.title')}</h2><button class="btn quiet icon" type="button" aria-label={t('sharing.close')} onclick={() => dialog?.close()}><X size={16} /></button></header>
  <div class="body">
    <p>{t('sharing.intro')}</p>
    {#if status}
      <section class="state" aria-label={t('sharing.status')}>
        <strong role="status" aria-live="polite">{t(`sharing.${status.phase}`)}</strong>
        <p>{t('sharing.last')}: {status.last ? dateTime(status.last.published_at) : t('sharing.never')}</p>
        {#if status.last}<p>{t('sharing.actual')}: <code>{status.last.destination}</code></p>{/if}
        {#if status.error === 'control_unavailable'}<p class="warn" role="alert">{t('sharing.controlFailure')}</p>{/if}
        {#if status.error}<p class="error" role="alert">{t('sharing.errorHelp')}</p><code>{status.error}</code>{/if}
        {#if status.local_access_blocked}<p class="warn">{t('sharing.blocked')}</p>{:else if status.phase === 'withdrawal_pending' || status.phase === 'unsafe'}<p class="warn">{t('sharing.readable')}</p>{/if}
      </section>
    {/if}
    {#if status && !status.supported_platform}<p class="warn" role="status">{t('sharing.unsupported')}</p>{/if}
    <fieldset disabled={busy || status?.supported_platform === false}>
      <label class="check"><input type="checkbox" bind:checked={draft.consent.enabled} onchange={change} />{t('sharing.enabled')}</label>
      <label for="exchange-file">{t('sharing.destination')}</label>
      <input id="exchange-file" class="path" type="text" bind:value={draft.destination} oninput={change} aria-describedby="exchange-hint" spellcheck="false" />
      <p id="exchange-hint" class="hint">{t('sharing.pathHint')}</p>
      {#if response}
        {#each kinds as kind (kind)}
          <details class="group" bind:open={expanded[kind]}><summary>{t(`sharing.${kind}`)} · {draft.consent[kind].length} / {response.choices[kind].length}</summary>
            <div class="choices">
              {#each response.choices[kind] as c (c.id)}
                <div class="choice">
                  <label class="check"><input type="checkbox" checked={draft.consent[kind].includes(c.id)} disabled={kind === 'trials' && !c.eligible && !draft.consent[kind].includes(c.id)} onchange={e => select(kind, c.id, e.currentTarget.checked)} />{c.name}</label>
                  <span class="hint">{c.published ? t('sharing.published') : t('sharing.notPublished')}</span>
                  {#if kind === 'trials'}
                    <p class="hint">{dependencyNames(c)}</p>
                    <span class="hint">{t(c.eligible ? 'sharing.eligible' : 'sharing.ineligible')}</span>
                    {#if c.eligible && draft.consent.trials.includes(c.id) && missing(c)}
                      <p class="warn">{t('sharing.missing')}</p><button class="btn" type="button" onclick={() => dependencies(c)}>{t('sharing.dependencies')}</button>
                    {/if}
                  {/if}
                </div>
              {:else}<p class="hint">{t('sharing.none')}</p>{/each}
            </div>
          </details>
        {/each}
      {/if}
    </fieldset>
    {#if dirty}<p class="warn" role="status">{t('sharing.draft')}</p>{/if}
    <div class="actions">
      <button class="btn" type="button" disabled={busy} onclick={() => action('preview')}>{t('sharing.preview')}</button>
      <button class="btn primary" type="button" disabled={busy || status?.supported_platform === false} onclick={() => action('save')}>{t('sharing.save')}</button>
      <button class="btn" type="button" disabled={busy || dirty || !status?.settings.destination || status.phase === 'unsafe'} onclick={() => action('publish')}>{t('sharing.publish')}</button>
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if preview}
      <section aria-label={t('sharing.preview')}><h3>{t('sharing.preview')}</h3><p class="hint">{t('sharing.previewHint')}</p>
        {#each ['machines', 'tools', 'materials', 'presets'] as const as kind (kind)}
          <p>{t(`sharing.${kind === 'presets' ? 'trials' : kind}`)}: {(preview.snapshot[kind] as {name: string}[]).map(r => r.name).join(', ') || t('sharing.none')}</p>
        {/each}
        {#if preview.excluded.length}<p class="warn">{t('sharing.missing')} ({preview.excluded.length})</p>{/if}
        <details><summary>{t('sharing.details')}</summary><pre>{JSON.stringify(preview.snapshot, null, 2)}</pre></details>
      </section>
    {/if}
    <p class="hint">{t('sharing.limits')}</p>
  </div>
</dialog>

<style>
  .sharing-entry { display:flex; align-items:center; flex-wrap:wrap; gap:var(--s3); padding:var(--s3); margin-bottom:var(--s3); border:1px solid var(--border); border-radius:var(--radius-lg); background:var(--bg-panel); }
  .sharing-entry div { flex:1; min-width:200px; } h3 { display:flex; align-items:center; gap:6px; margin:0; font-size:var(--fs-sm); }
  p { margin:var(--s2) 0; } .hint, .sharing-entry p { font-size:var(--fs-sm); color:var(--text-secondary); }
  dialog { width:min(760px, calc(100vw - 32px)); max-height:calc(100dvh - 32px); padding:0; color:var(--text); background:var(--bg-panel); border:1px solid var(--border); border-radius:var(--radius-lg); }
  dialog::backdrop { background: #0008; } header { display:flex; align-items:center; justify-content:space-between; padding:var(--s3); border-bottom:1px solid var(--border); } h2 { margin:0; font-size:var(--fs-lg); }
  .body { padding:var(--s3); } fieldset { border:0; padding:0; margin:var(--s3) 0; min-width:0; } label { display:block; font-size:var(--fs-sm); }
  .check { display:flex; align-items:center; gap:var(--s2); margin:var(--s2) 0; } .path { width:100%; box-sizing:border-box; margin-top:var(--s1); }
  .group { border-top:1px solid var(--border); padding:var(--s2) 0; } summary { cursor:pointer; font-size:var(--fs-sm); } .choices { max-height:240px; overflow:auto; } .choice { padding:var(--s1) 0; }
  .actions { display:flex; flex-wrap:wrap; gap:var(--s2); } .state { border-left:2px solid var(--border); padding-left:var(--s3); }
  .error { color:var(--danger); } .warn { color:var(--warning); } code, pre { overflow-wrap:anywhere; font-size:var(--fs-xs); } pre { white-space:pre-wrap; max-height:360px; overflow:auto; }
</style>
