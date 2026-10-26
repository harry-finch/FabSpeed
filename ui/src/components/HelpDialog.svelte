<script lang="ts">
  // Aide courte : mode d'emploi, lexique, légende des origines.
  import { X, Compass } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import OriginTag from './OriginTag.svelte';
  import { t } from '../lib/i18n.svelte';

  let { onClose }: { onClose: () => void } = $props();
  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="help-title">
  <header>
    <h2 id="help-title">{t('help.title')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={() => dialog.close()}><X size={16} /></button>
  </header>
  <div class="body">
    <ol class="steps">
      <li><b>{t('help.step1.term')}</b> {t('help.step1.text')}</li>
      <li><b>{t('help.step2.term')}</b> {t('help.step2.text')}</li>
      <li><b>{t('help.step3.term')}</b> {t('help.step3.text')}</li>
      <li><b>{t('help.step4.term')}</b> {t('help.step4.text')}</li>
    </ol>
    <p>{t('help.intro')}</p>

    <h3>{t('help.glossary')}</h3>
    <dl>
      <dt>{t('readout.rpm')}</dt><dd>{t('help.def.rpm')}</dd>
      <dt>{t('readout.feed')}</dt><dd>{t('help.def.feed')}</dd>
      <dt>{t('readout.plunge')}</dt><dd>{t('help.def.plunge')}</dd>
      <dt>{t('readout.doc')}</dt><dd>{t('help.def.doc')}</dd>
      <dt>{t('setup.stepover')}</dt><dd>{t('help.def.stepover')}</dd>
      <dt>{t('help.chip')}</dt><dd>{t('help.def.chip')}</dd>
    </dl>

    <h3>{t('help.originsTitle')}</h3>
    <ul class="legend">
      <li><OriginTag origin="lab_preset" /> {t('help.origin.lab_preset')}</li>
      <li><OriginTag origin="derived_from_preset" /> {t('help.origin.derived_from_preset')}</li>
      <li><OriginTag origin="calculated" /> {t('help.origin.calculated')}</li>
      <li><OriginTag origin="user_override" /> {t('help.origin.user_override')}</li>
    </ul>
    <p class="muted">{t('help.override')}</p>
    <button
      class="btn tour"
      type="button"
      onclick={() => {
        dialog.close();
        store.onboarding = true;
      }}><Compass size={14} /> {t('help.tour')}</button
    >
  </div>
</dialog>

<style>
  .tour {
    align-self: flex-start;
  }

  dialog {
    width: min(560px, 92vw);
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
    padding: var(--s3) var(--s4) var(--s4);
    font-size: var(--fs-md);
    line-height: 1.55;
    max-height: 70vh;
    overflow: auto;
  }

  .steps {
    margin: 0 0 var(--s2);
    padding-left: 20px;
  }

  h3 {
    margin: var(--s4) 0 var(--s2);
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  dl {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 4px var(--s3);
    margin: 0;
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
  }

  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .muted {
    color: var(--text-muted);
  }
</style>
