<script lang="ts">
  // Module « Bibliothèque » : machines, fraises et essais du lab.
  // Navigation à gauche, liste au centre, fiche sélectionnée dans l'inspecteur.
  import './editor.css';
  import { Cpu, Search, Plus, ClipboardList, Check, TriangleAlert, CircleX, FlaskConical, Lock, ShieldCheck, Layers, Database, FileSpreadsheet } from '@lucide/svelte';
  import ToolGlyph from '../ToolGlyph.svelte';
  import ToolEditor from './ToolEditor.svelte';
  import MachineEditor from './MachineEditor.svelte';
  import MaterialEditor from './MaterialEditor.svelte';
  import DataPanel from './DataPanel.svelte';
  import LibraryTransfer from './LibraryTransfer.svelte';
  import { swatch } from '../../lib/swatches';
  import TrialDetail from './TrialDetail.svelte';
  import { store, type LibrarySection } from '../../lib/store.svelte';
  import { CATEGORY_ORDER, FAMILY_IDS, categoryLabel, dateTime, familyLabel, layer as layerText, operationLabel, outcomeLabel, outcomes, rpmUnit, toolHeadline } from '../../lib/labels';
  import { t } from '../../lib/i18n.svelte';
  import { fmtSpeed, num, numTrim, speedUnit } from '../../lib/format';
  import { tip } from '../../lib/tip';
  import type { ToolFamily, TrialOutcome } from '../../lib/types';

  let query = $state('');
  let familyFilter = $state<ToolFamily | ''>('');
  let outcomeFilter = $state<TrialOutcome | ''>('');
  let materialFilter = $state('');

  const c = $derived(store.catalog);
  const lib = $derived(store.library);
  const u = $derived(store.units);
  const q = $derived(query.trim().toLowerCase());

  const SECTIONS: LibrarySection[] = ['machines', 'materials', 'tools', 'trials', 'data'];
  const count = (s: LibrarySection) =>
    s === 'data' ? null : (({ machines: c?.machines, materials: c?.materials, tools: c?.tools, trials: c?.trials })[s]?.length ?? 0);

  const machines = $derived((c?.machines ?? []).filter((m) => !q || m.name.toLowerCase().includes(q)));
  const materials = $derived(
    [...(c?.materials ?? [])]
      .filter((m) => !q || m.name.toLowerCase().includes(q) || m.description.toLowerCase().includes(q))
      .sort((a, b) => CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category)),
  );
  /** Types de fraises présents, pour les filtres rapides. */
  const families = $derived(FAMILY_IDS.filter((f) => c?.tools.some((x) => x.family === f)));
  const tools = $derived(
    [...(c?.tools ?? [])]
      .filter((x) => !familyFilter || x.family === familyFilter)
      .filter((x) => !q || x.name.toLowerCase().includes(q) || x.reference.toLowerCase().includes(q))
      .sort((a, b) => a.diameter_mm - b.diameter_mm || a.name.localeCompare(b.name)),
  );
  const trials = $derived(
    [...(c?.trials ?? [])]
      .filter((x) => !outcomeFilter || x.outcome === outcomeFilter)
      .filter((x) => !materialFilter || x.material_id === materialFilter)
      .filter((x) => !q || x.tool_name.toLowerCase().includes(q) || x.notes.toLowerCase().includes(q) || x.operator.toLowerCase().includes(q))
      .sort((a, b) => b.recorded_at.localeCompare(a.recorded_at)),
  );

  const selectedMaterial = $derived(lib.section === 'materials' && lib.selected ? c?.materials.find((m) => m.id === lib.selected) : undefined);
  const selectedMachine = $derived(lib.section === 'machines' && lib.selected ? c?.machines.find((m) => m.id === lib.selected) : undefined);
  const selectedTool = $derived(lib.section === 'tools' && lib.selected ? c?.tools.find((x) => x.id === lib.selected) : undefined);
  const selectedTrial = $derived(lib.section === 'trials' && lib.selected ? c?.trials.find((x) => x.id === lib.selected) : undefined);

  function go(section: LibrarySection) {
    query = '';
    familyFilter = '';
    store.library = { section, selected: null, creating: false };
  }
  const select = (id: string) => (store.library = { ...store.library, selected: id, creating: false });
  const create = () => (store.library = { ...store.library, selected: null, creating: true });

  const goodTrials = (toolId: string) => c?.trials.filter((x) => x.tool_id === toolId && x.outcome === 'good').length ?? 0;
  const materialName = (id: string) => c?.materials.find((m) => m.id === id)?.name ?? id;
  const ICON = { good: Check, adjust: TriangleAlert, failed: CircleX } as const;
</script>

<div class="lib">
  <nav class="nav" aria-label={t('library.title')}>
    <div class="head">{t('library.title')}</div>
    {#each SECTIONS as s (s)}
      <button type="button" class="sec" class:on={lib.section === s} aria-current={lib.section === s} onclick={() => go(s)}>
        {#if s === 'machines'}<Cpu size={15} />{:else if s === 'materials'}<Layers size={15} />{:else if s === 'data'}<Database size={15} />{:else if s === 'tools'}<ToolGlyph family="flat_endmill" helix="upcut" size={15} />{:else}<ClipboardList size={15} />{/if}
        <span>{t(`library.section.${s}`)}</span>
        {#if count(s) != null}<span class="n">{count(s)}</span>{/if}
      </button>
    {/each}
    {#if !store.auth.admin}<p class="help guest"><Lock size={12} /> {t('library.guestNote')}</p>{/if}
    <p class="help">
      {t('library.help1')} <code>data/lab/</code>{t('library.help2')} <code>data/lab/archive/</code>.
    </p>
  </nav>

  {#if lib.section === 'data'}
    <DataPanel />
  {:else}
  <section class="list" aria-label={t('library.list')}>
    <header class="bar">
      <div class="search">
        <Search size={13} />
        <input placeholder={t('library.searchPlaceholder')} bind:value={query} aria-label={t('library.search')} />
      </div>
      {#if lib.section === 'trials'}
        <select class="input filter" bind:value={outcomeFilter} aria-label={t('library.filterOutcome')}>
          <option value="">{t('library.allOutcomes')}</option>
          {#each outcomes() as o (o.id)}<option value={o.id}>{o.label}</option>{/each}
        </select>
        <select class="input filter" bind:value={materialFilter} aria-label={t('library.filterMaterial')}>
          <option value="">{t('library.allMaterials')}</option>
          {#each c?.materials ?? [] as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
        </select>
      {/if}
      <span class="grow"></span>
      {#if lib.section !== 'trials'}<LibraryTransfer kind={lib.section} />{/if}
      {#if lib.section === 'tools'}
        <button class="btn" type="button" onclick={create}><Plus size={14} /> {t('library.newTool')}</button>
      {:else if lib.section === 'machines'}
        <button class="btn" type="button" onclick={create}><Plus size={14} /> {t('library.newMachine')}</button>
      {:else if lib.section === 'materials'}
        <button class="btn" type="button" onclick={async () => (await store.ensureAdmin()) && create()}
          ><Plus size={14} /> {t('library.newMaterial')} {#if !store.auth.admin}<Lock size={12} />{/if}</button
        >
      {:else}
        <button class="btn quiet" type="button" use:tip={t('data.exportHint')} disabled={!trials.length} onclick={() => store.exportTrialsCsv()}>
          <FileSpreadsheet size={14} /> {t('data.exportShort')}
        </button>
        <button class="btn" type="button" use:tip={t('library.recordHint')} onclick={() => (store.module = 'calc')}>
          <ClipboardList size={14} /> {t('library.recordFromCalc')}
        </button>
      {/if}
    </header>

    {#if lib.section === 'tools' && families.length > 1}
      <div class="pills" role="radiogroup" aria-label={t('library.filterFamily')}>
        <button type="button" role="radio" class="pill" class:on={!familyFilter} aria-checked={!familyFilter} onclick={() => (familyFilter = '')}>{t('library.allTypes')}</button>
        {#each families as f (f)}
          <button type="button" role="radio" class="pill" class:on={familyFilter === f} aria-checked={familyFilter === f} onclick={() => (familyFilter = familyFilter === f ? '' : f)}>
            <ToolGlyph family={f} helix="upcut" size={13} />
            {familyLabel(f)}
          </button>
        {/each}
      </div>
    {/if}

    <div class="rows">
      {#if lib.section === 'machines'}
        {#each machines as m (m.id)}
          {@const layer = store.layerOf('machine', m.id)}
          <button type="button" class="row" class:selected={lib.selected === m.id} onclick={() => select(m.id)}>
            <Cpu size={16} />
            <span class="txt">
              <span class="name">{m.name}{#if m.id === store.setup.machineId}<span class="active">{' · '}{t('library.active')}</span>{/if}</span>
              <span class="sub">{m.category} · {num(m.rpm_min, 0)}–{num(m.rpm_max, 0)} {rpmUnit()} · {num(m.spindle_power_w / 1000, 1)} kW · {fmtSpeed(m.max_feed_mm_min, u)} {speedUnit(u)}</span>
            </span>
            <span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>
          </button>
        {/each}
      {:else if lib.section === 'materials'}
        {#each materials as m (m.id)}
          {@const layer = store.layerOf('material', m.id)}
          <button type="button" class="row" class:selected={lib.selected === m.id} onclick={() => select(m.id)}>
            <span class="swatch" style:background={swatch(m.id, m.category)}></span>
            <span class="txt">
              <span class="name">{m.name}{#if m.id === store.setup.materialId}<span class="active">{' · '}{t('library.active')}</span>{/if}</span>
              <span class="sub">{categoryLabel(m.category)} · Vc {num(m.vc_m_min, 0)} m/min · kc {num(m.kc_n_mm2, 0)} N/mm²{m.supervision_required ? ` · ${t('library.supervised')}` : ''}</span>
            </span>
            <span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>
          </button>
        {/each}
      {:else if lib.section === 'tools'}
        {#each tools as tool (tool.id)}
          {@const layer = store.layerOf('tool', tool.id)}
          {@const n = goodTrials(tool.id)}
          <button type="button" class="row" class:selected={lib.selected === tool.id} onclick={() => select(tool.id)}>
            <ToolGlyph family={tool.family} helix={tool.helix} size={18} />
            <span class="txt">
              <!-- Sans référence fournisseur, le nom (déjà descriptif) sert de titre. -->
              <span class="name">{tool.reference ? toolHeadline(tool, (v) => numTrim(v, 3)) : tool.name}</span>
              <span class="sub"
                >{[
                  t('tool.fluteLengthLong', { v: numTrim(tool.flute_length_mm) }),
                  t(`toolEditor.${tool.tool_material === 'hss' ? 'hss' : 'carbide'}`),
                  tool.supplier || null,
                ]
                  .filter(Boolean)
                  .join(' · ')}</span
              >
            </span>
            {#if tool.reference}<code class="ref">{tool.reference}</code>{/if}
            {#if n}<span class="tested" use:tip={t(n > 1 ? 'library.goodTrials' : 'library.goodTrial', { n })}><FlaskConical size={12} /> {n}</span>{/if}
            <span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>
          </button>
        {/each}
      {:else}
        {#each trials as tr (tr.id)}
          {@const I = ICON[tr.outcome]}
          <button type="button" class="row" class:selected={lib.selected === tr.id} onclick={() => select(tr.id)}>
            <span class="out {tr.outcome}"><I size={14} />{#if tr.validation}<ShieldCheck size={12} />{/if}</span>
            <span class="txt">
              <span class="name">{materialName(tr.material_id)} · {tr.tool_name.split('·')[0]} · {operationLabel(tr.operation)}</span>
              <span class="sub tabular">{num(tr.rpm, 0)} {rpmUnit()} · {fmtSpeed(tr.feed_mm_min, u)} {speedUnit(u)} · {t('trials.pass')} {numTrim(tr.doc_mm)} mm · {outcomeLabel(tr.outcome)}</span>
            </span>
            <span class="date">{dateTime(tr.recorded_at)}{tr.operator ? `\n${tr.operator}` : ''}</span>
          </button>
        {:else}
          <div class="empty">
            <ClipboardList size={22} />
            <p>{outcomeFilter || materialFilter || q ? t('library.noTrialsFilter') : t('library.noTrialsYet')}</p>
            <p class="muted">{t('library.noTrialsHint')}</p>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <aside class="inspector" aria-label={t('library.record')}>
    {#if lib.section === 'materials' && (lib.creating || selectedMaterial)}
      {#key lib.selected ?? 'new'}<MaterialEditor material={lib.creating ? null : (selectedMaterial ?? null)} />{/key}
    {:else if lib.section === 'machines' && (lib.creating || selectedMachine)}
      {#key lib.selected ?? 'new'}<MachineEditor machine={lib.creating ? null : (selectedMachine ?? null)} />{/key}
    {:else if lib.section === 'tools' && (lib.creating || selectedTool)}
      {#key lib.selected ?? 'new'}<ToolEditor tool={lib.creating ? null : (selectedTool ?? null)} />{/key}
    {:else if lib.section === 'trials' && selectedTrial}
      {#key selectedTrial.id}<TrialDetail trial={selectedTrial} />{/key}
    {:else}
      <div class="placeholder">{t('library.selectHint')}</div>
    {/if}
  </aside>
  {/if}
</div>

<style>
  .lib {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr) 380px;
    min-height: 0;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: var(--s3);
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    height: 32px;
    padding: 0 var(--s3);
    margin-bottom: var(--s1);
    border-bottom: 1px solid var(--divider);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .sec {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin: 0 var(--s2);
    height: 30px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-secondary);
    text-align: left;
  }

  .sec:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .sec.on {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .n {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .help {
    margin: auto var(--s3) 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    line-height: 1.5;
  }

  .help.guest {
    margin-bottom: var(--s2);
    color: var(--text-secondary);
  }

  .help.guest + .help {
    margin-top: 0;
  }

  .help code {
    font-family: var(--mono);
    font-size: 10px;
  }

  .list {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-workspace);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: 40px;
    padding: 0 var(--s3);
    border-bottom: 1px solid var(--divider);
    background: var(--bg-panel);
    flex-wrap: wrap;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 220px;
    height: var(--control-h);
    padding: 0 var(--s2);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-muted);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
  }

  .filter {
    width: auto;
  }

  .grow {
    flex: 1;
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: var(--s2) var(--s3);
    border-bottom: 1px solid var(--divider);
    background: var(--bg-panel);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 9px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg-control);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }

  .pill:hover {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }

  .pill.on {
    color: var(--text-primary);
    border-color: var(--accent);
    background: var(--accent-tint);
  }

  .ref {
    flex: none;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--neutral-tint);
    color: var(--text-secondary);
    font-family: var(--mono);
    font-size: var(--fs-xs);
  }

  .rows {
    flex: 1;
    overflow-y: auto;
    padding: var(--s2);
  }

  .swatch {
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--swatch-edge);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
    width: 100%;
    padding: 7px var(--s3);
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-secondary);
    text-align: left;
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .row.selected {
    background: var(--bg-selected);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .txt {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .name {
    color: var(--text-primary);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .active {
    color: var(--accent-text);
    font-weight: 400;
  }

  .sub {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .layer {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: var(--fs-xs);
    font-weight: 600;
  }

  .layer.base {
    color: var(--text-muted);
  }

  .layer.lab {
    color: var(--origin-user);
    background: var(--origin-user-tint);
  }

  .layer.modified {
    color: var(--warning);
    background: var(--warning-tint);
  }

  .tested {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--origin-lab);
    font-size: var(--fs-xs);
    font-weight: 600;
  }

  .out {
    display: inline-flex;
  }

  .out.good {
    color: var(--success);
  }

  .out.adjust {
    color: var(--warning);
  }

  .out.failed {
    color: var(--danger);
  }

  .date {
    flex: none;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    text-align: right;
    white-space: pre-line;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 48px var(--s5);
    color: var(--text-secondary);
    text-align: center;
  }

  .empty p {
    margin: 0;
    max-width: 420px;
  }

  .muted {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .inspector {
    min-height: 0;
    overflow-y: auto;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
  }

  .placeholder {
    padding: var(--s5) var(--s4);
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  @media (max-width: 1279px) {
    .lib {
      grid-template-columns: 180px minmax(0, 1fr) 340px;
    }
  }

  @media (max-width: 860px) {
    .lib {
      grid-template-columns: 1fr;
    }
  }
</style>
