<script lang="ts">
  import { ChevronDown, FlaskConical, ShieldAlert, Check, Plus, Library, Lock } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import { store } from '../lib/store.svelte';
  import { categoryLabel, CATEGORY_ORDER } from '../lib/labels';
  import { t } from '../lib/i18n.svelte';
  import { swatch } from '../lib/swatches';
  import { tip } from '../lib/tip';
  import type { Material } from '../lib/types';

  const groups = $derived(
    CATEGORY_ORDER.map((cat) => ({
      cat,
      items: (store.catalog?.materials ?? []).filter((m) => m.category === cat),
    })).filter((g) => g.items.length),
  );

  function testedOnMachine(m: Material): boolean {
    return store.tools.some((t) => store.isTested(t.id, m.id));
  }

  const current = $derived(store.material);
</script>

<Popover label={t('picker.chooseMaterial')} width={340}>
  {#snippet trigger({ open, toggle })}
    <button class="trigger" class:open type="button" onclick={toggle} aria-haspopup="dialog" aria-expanded={open}>
      {#if current}
        <span class="sw" style:background={swatch(current.id, current.category)}></span>
        <span class="name">{current.name}</span>
        {#if testedOnMachine(current)}
          <span class="lab" use:tip={t('picker.materialTestedHint')}>
            <FlaskConical size={12} /> {t('picker.tested')}
          </span>
        {/if}
      {:else}
        <span class="name muted">{t('picker.choose')}</span>
      {/if}
      <ChevronDown size={14} class="chev" />
    </button>
  {/snippet}
  {#snippet content({ close })}
    {#each groups as g (g.cat)}
      <div class="group">{categoryLabel(g.cat)}</div>
      {#each g.items as m (m.id)}
        <button
          type="button"
          class="row"
          class:selected={m.id === store.setup.materialId}
          onclick={() => {
            store.setMaterial(m.id);
            close();
          }}
        >
          <span class="sw" style:background={swatch(m.id, m.category)}></span>
          <span class="txt">
            <span class="rname">{m.name}</span>
            <span class="desc">{m.description}</span>
          </span>
          <span class="marks">
            {#if m.supervision_required}
              <span class="warn" use:tip={t('picker.supervised')}><ShieldAlert size={13} /></span>
            {/if}
            {#if testedOnMachine(m)}
              <span class="lab" use:tip={t('picker.testedOnMachine')}><FlaskConical size={12} /></span>
            {/if}
            {#if m.id === store.setup.materialId}<Check size={14} />{/if}
          </span>
        </button>
      {/each}
    {/each}
    <div class="foot">
      <button
        class="btn quiet"
        type="button"
        onclick={async () => {
          close();
          if (await store.ensureAdmin()) store.openLibrary('materials', null, true);
        }}><Plus size={14} /> {t('library.newMaterial')} {#if !store.auth.admin}<Lock size={12} />{/if}</button
      >
      <button
        class="btn quiet"
        type="button"
        onclick={() => {
          close();
          store.openLibrary('materials', store.setup.materialId);
        }}><Library size={14} /> {t('picker.manage')}</button
      >
    </div>
  {/snippet}
</Popover>

<style>
  .foot {
    display: flex;
    gap: var(--s1);
    border-top: 1px solid var(--divider);
    margin-top: var(--s1);
    padding-top: var(--s1);
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: 32px;
    padding: 0 var(--s2);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-align: left;
  }

  .trigger:hover,
  .trigger.open {
    border-color: var(--border-strong);
  }

  .trigger :global(.chev) {
    margin-left: auto;
    color: var(--text-muted);
    flex: none;
  }

  .sw {
    width: 14px;
    height: 14px;
    flex: none;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--swatch-edge);
  }

  .name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .muted {
    color: var(--text-muted);
  }

  .lab {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--origin-lab);
    font-size: var(--fs-xs);
    font-weight: 600;
    flex: none;
  }

  .group {
    padding: var(--s2) var(--s2) var(--s1);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
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
    min-width: 0;
    flex: 1;
  }

  .rname {
    font-weight: 500;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .marks {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent-text);
  }

  .warn {
    display: inline-flex;
    color: var(--warning);
  }
</style>
