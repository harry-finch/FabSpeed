<script lang="ts">
  import { ChevronDown, FlaskConical, Search, Plus, Upload, Check, Ban, User, Library } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import LibraryTransfer from './library/LibraryTransfer.svelte';
  import ToolGlyph from './ToolGlyph.svelte';
  import { store } from '../lib/store.svelte';
  import { toolSummary } from '../lib/labels';
  import { t, type Key } from '../lib/i18n.svelte';
  import { numTrim } from '../lib/format';
  import { tip } from '../lib/tip';
  import type { Tool } from '../lib/types';

  let { onCreate }: { onCreate: () => void } = $props();

  let query = $state('');
  let transfer: { pick: () => Promise<void> };

  const materialId = $derived(store.setup.materialId);

  const summary = (tool: Tool) => toolSummary(tool, (v) => numTrim(v, 3));

  type Rank = 0 | 1 | 2 | 3;
  function rank(t: Tool): Rank {
    if (store.isTested(t.id, materialId)) return 0;
    if (t.excluded_materials.includes(materialId)) return 3;
    if (t.recommended_materials.includes(materialId)) return 1;
    return 2;
  }

  const groupLabel = (r: Rank) => t(`toolPicker.group${r}` as Key);

  const grouped = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = store.tools.filter(
      (t) => !q || t.name.toLowerCase().includes(q) || t.reference.toLowerCase().includes(q),
    );
    const out = new Map<Rank, Tool[]>();
    for (const t of list) {
      const r = rank(t);
      out.set(r, [...(out.get(r) ?? []), t]);
    }
    return ([0, 1, 2, 3] as Rank[])
      .filter((r) => out.has(r))
      .map((r) => ({ r, items: out.get(r)!.sort((a, b) => a.diameter_mm - b.diameter_mm) }));
  });

  const current = $derived(store.tool);
  const fromLab = (t: Tool) => store.layerOf('tool', t.id) !== 'base';
</script>

<LibraryTransfer kind="tools" controls={false} bind:this={transfer} />

<Popover label={t('toolPicker.choose')} width={360}>
  {#snippet trigger({ open, toggle })}
    <button class="trigger" class:open type="button" onclick={toggle} aria-haspopup="dialog" aria-expanded={open}>
      {#if current}
        <span class="glyph"><ToolGlyph family={current.family} helix={current.helix} size={20} /></span>
        <span class="txt">
          <span class="name">{current.reference || current.name}</span>
          <span class="sub">{summary(current)}</span>
        </span>
      {:else}
        <span class="name muted">{t('toolPicker.chooseEllipsis')}</span>
      {/if}
      <ChevronDown size={14} class="chev" />
    </button>
  {/snippet}
  {#snippet content({ close })}
    <div class="search">
      <Search size={13} />
      <!-- svelte-ignore a11y_autofocus -->
      <input placeholder={t('toolPicker.searchPlaceholder')} bind:value={query} autofocus aria-label={t('toolPicker.search')} />
    </div>
    {#each grouped as g (g.r)}
      <div class="group">{groupLabel(g.r)}</div>
      {#each g.items as tool (tool.id)}
        <button
          type="button"
          class="row"
          class:selected={tool.id === store.setup.toolId}
          class:dim={g.r === 3}
          onclick={() => {
            store.setTool(tool.id);
            close();
          }}
        >
          <span class="glyph"><ToolGlyph family={tool.family} helix={tool.helix} size={20} /></span>
          <span class="txt">
            <span class="name">{tool.reference || tool.name}</span>
            <span class="sub">{summary(tool)}</span>
          </span>
          <span class="marks">
            {#if fromLab(tool)}<span use:tip={t('toolPicker.labRecord')}><User size={12} /></span>{/if}
            {#if g.r === 0}<span class="lab"><FlaskConical size={12} /></span>{/if}
            {#if g.r === 3}<span class="warn"><Ban size={12} /></span>{/if}
            {#if tool.id === store.setup.toolId}<Check size={14} />{/if}
          </span>
        </button>
      {/each}
    {/each}
    <div class="foot">
      <button
        class="btn quiet"
        type="button"
        onclick={() => {
          close();
          onCreate();
        }}><Plus size={14} /> {t('toolPicker.newTool')}</button
      >
      <button
        class="btn quiet"
        type="button"
        use:tip={t('toolPicker.importHint')}
        onclick={async () => {
          close();
          await transfer.pick();
        }}><Upload size={14} /> {t('toolPicker.import')}</button
      >
      <button
        class="btn quiet"
        type="button"
        use:tip={t('toolPicker.manageHint')}
        onclick={() => {
          close();
          store.openLibrary('tools', store.setup.toolId);
        }}><Library size={14} /> {t('toolPicker.manage')}</button
      >
    </div>
  {/snippet}
</Popover>

<style>
  .trigger {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: 40px;
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

  .glyph {
    display: inline-flex;
    color: var(--text-secondary);
    flex: none;
  }

  .txt {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.25;
  }

  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  .muted {
    color: var(--text-muted);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 2px 2px 4px;
    padding: 0 var(--s2);
    height: var(--control-h);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-muted);
  }

  .search input {
    flex: 1;
    background: none;
    border: 0;
    outline: none;
    color: var(--text-primary);
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
    padding: 5px var(--s2);
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

  .row.dim .name {
    color: var(--text-secondary);
  }

  .marks {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }

  .marks :global(svg:last-child) {
    flex: none;
  }

  .lab {
    display: inline-flex;
    color: var(--origin-lab);
  }

  .warn {
    display: inline-flex;
    color: var(--warning);
  }

  .foot {
    display: flex;
    gap: var(--s1);
    border-top: 1px solid var(--divider);
    margin-top: var(--s1);
    padding-top: var(--s1);
  }
</style>
