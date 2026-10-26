<script lang="ts">
  // Mot de passe administrateur : connexion, premier mot de passe du poste,
  // ou changement. Fermer la fenêtre annule l'action en attente.
  import { X, ShieldCheck } from '@lucide/svelte';
  import { store, type AuthDialog } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';

  const MIN_LENGTH = 8; // identique à MIN_PASSWORD_LEN côté serveur

  let { mode }: { mode: AuthDialog } = $props();
  let dialog: HTMLDialogElement;
  let current = $state('');
  let password = $state('');
  let confirm = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);
  let done = false;

  const choosing = $derived(mode !== 'login');
  const mismatch = $derived(choosing && confirm !== '' && confirm !== password);
  const valid = $derived(
    mode === 'login' ? password !== '' : password.length >= MIN_LENGTH && password === confirm && (mode !== 'change' || current !== ''),
  );

  $effect(() => {
    dialog.showModal();
  });

  async function submit() {
    if (!valid || busy) return;
    busy = true;
    error =
      mode === 'login'
        ? await store.login(password)
        : mode === 'setup'
          ? await store.setupPassword(password)
          : await store.changePassword(current, password);
    busy = false;
    // En cas de succès, le store a déjà fermé la fenêtre (composant démonté).
    if (!error) done = true;
  }
</script>

<dialog bind:this={dialog} onclose={() => !done && store.closeAuthDialog(false)} aria-labelledby="auth-title">
  <header>
    <h2 id="auth-title"><ShieldCheck size={16} /> {t(`auth.title.${mode}`)}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={() => dialog.close()}><X size={16} /></button>
  </header>
  <form
    class="body"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <p class="intro">{t(`auth.intro.${mode}`)}</p>
    {#if mode === 'change'}
      <label>
        <span>{t('auth.current')}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input" type="password" autocomplete="current-password" autofocus bind:value={current} />
      </label>
    {/if}
    <label>
      <span>{mode === 'login' ? t('auth.password') : t('auth.newPassword')}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="input"
        type="password"
        autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
        autofocus={mode !== 'change'}
        bind:value={password}
      />
    </label>
    {#if choosing}
      <label>
        <span>{t('auth.confirm')}</span>
        <input class="input" class:bad={mismatch} type="password" autocomplete="new-password" bind:value={confirm} />
      </label>
      <p class="hint">{mismatch ? t('auth.mismatch') : t('auth.minLength', { n: MIN_LENGTH })}</p>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if mode === 'login'}<p class="hint">{t('auth.forgot')}</p>{/if}
    <div class="foot">
      <button class="btn quiet" type="button" onclick={() => dialog.close()}>{t('common.cancel')}</button>
      <button class="btn primary" type="submit" disabled={!valid || busy}>{t(`auth.submit.${mode}`)}</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(420px, 94vw);
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

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: var(--s3) var(--s4) var(--s4);
  }

  .intro,
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }

  .hint {
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  label {
    display: grid;
    gap: 4px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .bad {
    border-color: var(--danger) !important;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: var(--fs-sm);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    margin-top: var(--s1);
  }
</style>
