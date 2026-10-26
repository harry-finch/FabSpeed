<script lang="ts">
  // Barre d'application : identité, machine active (le « document »), unités,
  // langue, rôle (invité / administrateur), aide.
  import { ChevronDown, CircleHelp, Check, Cpu, Plus, Library, Lock, ShieldCheck, KeyRound, LogOut, Monitor, Sun, Moon } from '@lucide/svelte';
  import { theme, THEMES } from '../lib/theme.svelte';
  import Popover from './Popover.svelte';
  import { store } from '../lib/store.svelte';
  import { num } from '../lib/format';
  import { tip } from '../lib/tip';
  import { i18n, LOCALES, t } from '../lib/i18n.svelte';
  import { rpmUnit } from '../lib/labels';

  let { onHelp }: { onHelp: () => void } = $props();
  const machine = $derived(store.machine);
</script>

<header class="bar">
  <div class="brand">
    <img class="logo" src="/fabspeed-logo.svg?v=2" alt="" width="24" height="24" />
    <span class="name">FabSpeed</span>
    <span class="tag">Feeds &amp; Speeds</span>
  </div>

  <div class="machine">
    <Popover label={t('app.chooseMachine')} width={380}>
      {#snippet trigger({ open, toggle })}
        <button class="mbtn" class:open type="button" onclick={toggle} aria-haspopup="dialog" aria-expanded={open}>
          <Cpu size={14} />
          <span class="mlabel">{t('app.machine')}</span>
          <span class="mname">{machine?.name ?? '—'}</span>
          <ChevronDown size={13} />
        </button>
      {/snippet}
      {#snippet content({ close })}
        {#each store.catalog?.machines ?? [] as m (m.id)}
          <button
            class="row"
            class:selected={m.id === store.setup.machineId}
            type="button"
            onclick={() => {
              store.setMachine(m.id);
              close();
            }}
          >
            <span class="txt">
              <span class="rname">{m.name}</span>
              <span class="desc">{m.category} · {num(m.rpm_min, 0)}–{num(m.rpm_max, 0)} {rpmUnit()} · {t('app.maxFeed')} {num(m.max_feed_mm_min, 0)} mm/min</span>
            </span>
            {#if m.id === store.setup.machineId}<Check size={14} />{/if}
          </button>
        {/each}
        <div class="foot">
          <button
            class="btn quiet"
            type="button"
            onclick={() => {
              close();
              store.openLibrary('machines', null, true);
            }}><Plus size={14} /> {t('app.newMachine')}</button
          >
          <button
            class="btn quiet"
            type="button"
            onclick={() => {
              close();
              store.openLibrary('machines', store.setup.machineId);
            }}><Library size={14} /> {t('app.manageMachines')}</button
          >
        </div>
      {/snippet}
    </Popover>
  </div>

  <div class="modules" role="tablist" aria-label={t('app.module')}>
    <button type="button" role="tab" aria-selected={store.module === 'calc'} class:on={store.module === 'calc'} onclick={() => (store.module = 'calc')}>{t('app.calc')}</button>
    <button type="button" role="tab" aria-selected={store.module === 'library'} class:on={store.module === 'library'} onclick={() => (store.module = 'library')}>{t('app.library')}</button>
  </div>

  <div class="right">
    <div class="units" role="radiogroup" aria-label={t('app.units')}>
      {#each [['mm', 'mm'], ['in', t('app.inches')]] as [id, label] (id)}
        <button
          type="button"
          role="radio"
          aria-checked={store.units === id}
          class:on={store.units === id}
          onclick={() => (store.units = id as 'mm' | 'in')}>{label}</button
        >
      {/each}
    </div>
    <div class="units" role="radiogroup" aria-label={t('app.language')}>
      {#each LOCALES as l (l.id)}
        <button
          type="button"
          role="radio"
          aria-checked={i18n.locale === l.id}
          aria-label={l.label}
          class:on={i18n.locale === l.id}
          onclick={() => store.setLocale(l.id)}>{l.id.toUpperCase()}</button
        >
      {/each}
    </div>
    <div class="units theme" role="radiogroup" aria-label={t('app.theme')}>
      {#each THEMES as th (th)}
        {@const Icon = { system: Monitor, light: Sun, dark: Moon }[th]}
        <button
          type="button"
          role="radio"
          aria-checked={theme.choice === th}
          aria-label={t(`app.themes.${th}`)}
          class:on={theme.choice === th}
          use:tip={t(`app.themes.${th}`)}
          onclick={() => theme.set(th)}><Icon size={12} /></button
        >
      {/each}
    </div>
    {#if store.auth.admin}
      <Popover label={t('auth.adminMenu')} width={240}>
        {#snippet trigger({ open, toggle })}
          <button class="role admin" class:open type="button" onclick={toggle} aria-haspopup="menu" aria-expanded={open}>
            <ShieldCheck size={13} />
            {t('auth.admin')}
            <ChevronDown size={12} />
          </button>
        {/snippet}
        {#snippet content({ close })}
          <button
            class="row"
            type="button"
            onclick={() => {
              close();
              store.authDialog = 'change';
            }}><KeyRound size={14} /> <span class="rname">{t('auth.changePassword')}</span></button
          >
          <button
            class="row"
            type="button"
            onclick={() => {
              close();
              store.logout();
            }}><LogOut size={14} /> <span class="rname">{t('auth.signOut')}</span></button
          >
        {/snippet}
      </Popover>
    {:else}
      <button class="role" type="button" use:tip={t('auth.guestHint')} onclick={() => store.ensureAdmin()}>
        <Lock size={13} />
        {t('auth.guest')}
      </button>
    {/if}
    <button class="btn quiet icon" type="button" use:tip={t('app.help')} onclick={onHelp}>
      <CircleHelp size={16} />
    </button>
  </div>
</header>

<style>
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s4);
    height: 42px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-bottom: 1px solid var(--border);
  }

  .bar::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    background: var(--identity);
    opacity: 0.85;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .logo {
    width: 24px;
    height: 24px;
    display: block;
  }

  .name {
    font-weight: 700;
    font-size: var(--fs-lg);
    letter-spacing: -0.01em;
  }

  .tag {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  @media (max-width: 1100px) {
    .tag {
      display: none;
    }
  }

  .machine {
    width: 340px;
  }

  .mbtn {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: var(--control-h);
    padding: 0 var(--s2);
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
  }

  .mbtn:hover,
  .mbtn.open {
    border-color: var(--border-strong);
  }

  .mlabel {
    font-size: var(--fs-sm);
  }

  .mname {
    flex: 1;
    text-align: left;
    color: var(--text-primary);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    padding: 6px var(--s2);
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    color: var(--accent-text);
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .row.selected {
    background: var(--bg-selected);
  }

  .txt {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .rname {
    color: var(--text-primary);
    font-weight: 500;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .foot {
    display: flex;
    gap: var(--s1);
    border-top: 1px solid var(--divider);
    margin-top: var(--s1);
    padding-top: var(--s1);
  }

  /* Sélecteur de module, à la manière de Lightroom : texte seul, actif souligné. */
  .modules {
    display: flex;
    align-items: stretch;
    gap: var(--s3);
    height: 100%;
    margin-left: auto;
  }

  .modules button {
    padding: 0 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-md);
    font-weight: 500;
  }

  .modules button:hover {
    color: var(--text-primary);
  }

  .modules button.on {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .right {
    margin-left: var(--s4);
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .role {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 var(--s2);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .role:hover,
  .role.open {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .role.admin {
    color: var(--accent-text);
    border-color: var(--accent);
  }

  .units {
    display: flex;
    height: 24px;
    padding: 2px;
    gap: 2px;
    background: var(--segment-track);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .units button {
    padding: 0 var(--s2);
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .units button.on {
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .theme button {
    display: grid;
    place-items: center;
    padding: 0 6px;
  }
</style>
