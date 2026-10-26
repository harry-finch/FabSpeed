<script lang="ts">
  // Mise en page : barre d'application, réglage (gauche), espace de travail
  // (centre), assistant (droite), barre d'état.
  import AppBar from './components/AppBar.svelte';
  import SetupPanel from './components/SetupPanel.svelte';
  import Workspace from './components/Workspace.svelte';
  import AssistantPanel from './components/AssistantPanel.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import HelpDialog from './components/HelpDialog.svelte';
  import TrialDialog from './components/TrialDialog.svelte';
  import AuthDialog from './components/AuthDialog.svelte';
  import Onboarding from './components/Onboarding.svelte';
  import ValidateDialog from './components/ValidateDialog.svelte';
  import Library from './components/library/Library.svelte';
  import { onMount } from 'svelte';
  import { store } from './lib/store.svelte';

  let help = $state(false);

  // Chargement unique du catalogue (hors suivi réactif).
  onMount(() => {
    store.init();
  });

  // Recalcul et mémorisation à chaque changement de la configuration.
  $effect(() => {
    void store.request;
    store.schedule();
  });

  $effect(() => {
    void JSON.stringify(store.setup);
    void store.units;
    void store.operator;
    store.persist();
  });
</script>

<div class="app">
  <AppBar onHelp={() => (help = true)} />
  {#if store.module === 'calc'}
    <div class="main">
      <SetupPanel />
      <Workspace />
      <AssistantPanel />
    </div>
  {:else}
    <Library />
  {/if}
  <StatusBar />
</div>

{#if help}<HelpDialog onClose={() => (help = false)} />{/if}
{#if store.trialDialog}<TrialDialog onClose={() => (store.trialDialog = false)} />{/if}
{#if store.validateDialog}{#key store.validateDialog}<ValidateDialog id={store.validateDialog} onClose={() => (store.validateDialog = null)} />{/key}{/if}
{#if store.onboarding}<Onboarding />{/if}
{#if store.authDialog}{#key store.authDialog}<AuthDialog mode={store.authDialog} />{/key}{/if}

<div class="toasts" aria-live="polite">
  {#each store.toasts as t (t.id)}
    <div class="toast">{t.text}</div>
  {/each}
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    min-width: 0;
  }

  .main {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr) 320px;
    grid-template-areas: 'setup ws assist';
    min-height: 0;
  }

  .main > :global(:nth-child(1)) {
    grid-area: setup;
  }

  .main > :global(:nth-child(2)) {
    grid-area: ws;
  }

  .main > :global(:nth-child(3)) {
    grid-area: assist;
  }

  /* Écran moyen : l'assistant passe sous l'espace de travail. */
  @media (max-width: 1279px) {
    .main {
      grid-template-columns: 280px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) minmax(0, 300px);
      grid-template-areas:
        'setup ws'
        'setup assist';
    }

    .main > :global(:nth-child(3)) {
      border-left: 0;
      border-top: 1px solid var(--border);
    }
  }

  /* Petit écran : une seule colonne qui défile. */
  @media (max-width: 860px) {
    .app {
      height: auto;
      min-height: 100vh;
    }

    .main {
      grid-template-columns: 1fr;
      grid-template-rows: auto;
      grid-template-areas: 'setup' 'ws' 'assist';
    }
  }

  .toasts {
    position: fixed;
    bottom: 36px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 900;
    pointer-events: none;
  }

  .toast {
    padding: 8px 14px;
    border-radius: var(--radius-lg);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    font-size: var(--fs-sm);
    box-shadow: var(--shadow-pop);
    animation: rise 160ms ease-out;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
