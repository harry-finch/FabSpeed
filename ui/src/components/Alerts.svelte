<script lang="ts">
  // Avertissements : les bloquants et avertissements toujours visibles,
  // les informations repliées derrière un compteur.
  import { CircleX, TriangleAlert, Info, ChevronRight } from '@lucide/svelte';
  import type { Warning } from '../lib/types';
  import { t } from '../lib/i18n.svelte';

  let { warnings }: { warnings: Warning[] } = $props();
  let showInfo = $state(false);

  const strong = $derived(warnings.filter((w) => w.severity !== 'info'));
  const infos = $derived(warnings.filter((w) => w.severity === 'info'));
</script>

{#if warnings.length}
  <div class="alerts" aria-live="polite">
    {#each strong as w (w.code)}
      <div class="row {w.severity}">
        {#if w.severity === 'blocking'}<CircleX size={14} />{:else}<TriangleAlert size={14} />{/if}
        <span>{w.message}</span>
      </div>
    {/each}
    {#if infos.length}
      <button class="more" type="button" aria-expanded={showInfo} onclick={() => (showInfo = !showInfo)}>
        <ChevronRight size={12} class={showInfo ? 'open' : ''} />
        <Info size={13} />
        <span class="count">{t(infos.length > 1 ? 'alerts.infos' : 'alerts.info', { n: infos.length })}</span>
        <!-- Aperçu de la première remarque : on sait de quoi il s'agit sans ouvrir. -->
        {#if !showInfo}<span class="peek">{infos[0].message}</span>{/if}
      </button>
      {#if showInfo}
        {#each infos as w (w.code)}
          <div class="row info"><Info size={13} /><span>{w.message}</span></div>
        {/each}
      {/if}
    {/if}
  </div>
{/if}

<style>
  .alerts {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s1) var(--s3) var(--s2);
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
    padding: 5px var(--s2);
    border-radius: var(--radius);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .row :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .row.blocking {
    background: var(--danger-tint);
    color: var(--danger-text);
  }

  .row.blocking :global(svg) {
    color: var(--danger);
  }

  .row.warning {
    color: var(--text-primary);
  }

  .row.warning :global(svg) {
    color: var(--warning);
  }

  .row.info {
    color: var(--text-secondary);
  }

  .row.info :global(svg) {
    color: var(--info);
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    align-self: flex-start;
    padding: 3px var(--s2);
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .more {
    max-width: 100%;
    min-width: 0;
  }

  .count {
    flex: none;
    font-weight: 600;
  }

  .peek {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .peek::before {
    content: '— ';
  }

  .more:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .more :global(.open) {
    transform: rotate(90deg);
  }
</style>
