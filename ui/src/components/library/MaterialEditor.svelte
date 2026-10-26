<script lang="ts">
  // Fiche d'une matière (inspecteur de la bibliothèque). Une nouvelle matière
  // part d'une matière proche. Les constantes de base sont calées sous les
  // réglages historiques du lab : les modifier sort de ce calibrage, ce que
  // l'éditeur rappelle et que le moteur signale dans chaque calcul.
  import { untrack } from 'svelte';
  import { Save, Copy, Archive, Undo2, Crosshair, Lock, TriangleAlert, X } from '@lucide/svelte';
  import Field from '../Field.svelte';
  import NumberInput from '../NumberInput.svelte';
  import { store } from '../../lib/store.svelte';
  import { CATEGORY_ORDER, categoryLabel, layer as layerText } from '../../lib/labels';
  import { t } from '../../lib/i18n.svelte';
  import { tip } from '../../lib/tip';
  import type { Material } from '../../lib/types';

  let { material }: { material: Material | null } = $props();

  /** Nouvelle matière : copie d'une matière proche, constantes non calibrées. */
  function fromTemplate(tpl: Material): Material {
    const copy = structuredClone($state.snapshot(tpl) as Material);
    copy.id = '';
    copy.name = '';
    copy.aliases = []; // un alias appartient à une seule fiche
    copy.description = '';
    copy.tips = [];
    copy.source = t('materialEditor.fromTemplate', { name: tpl.name });
    copy.evidence = 'theoretical';
    return copy;
  }

  const templates = store.catalog?.materials ?? [];
  const source = untrack(() => material);
  const initial: Material = source
    ? structuredClone($state.snapshot(source) as Material)
    : fromTemplate(templates.find((m) => m.id === 'material/wood.plywood') ?? templates[0]);
  let d = $state<Material>(structuredClone(initial));
  let tipsText = $state(initial.tips.join('\n'));
  let confirmArchive = $state(false);
  let saving = $state(false);

  const isNew = $derived(!d.id);
  const layer = $derived(isNew ? null : store.layerOf('material', d.id));
  const lines = (text: string) =>
    text
      .split('\n')
      .map((x) => x.trim())
      .filter(Boolean);
  const dirty = $derived(JSON.stringify({ ...d, tips: lines(tipsText) }) !== JSON.stringify(initial));
  const CONSTANTS = ['vc_m_min', 'k_fz', 'k_ap', 'kc_n_mm2', 'fz_min_mm'] as const;
  const constantsChanged = $derived(CONSTANTS.some((k) => d[k] !== initial[k]));
  const valid = $derived(!!d.name.trim() && CONSTANTS.every((k) => d[k] > 0) && d.default_stepover > 0 && d.default_stepover <= 1);
  const goodTrials = $derived(store.catalog?.trials.filter((x) => x.material_id === d.id && x.outcome === 'good').length ?? 0);

  function revert() {
    d = structuredClone(initial);
    tipsText = initial.tips.join('\n');
  }

  function startFrom(id: string) {
    const tpl = templates.find((m) => m.id === id);
    if (!tpl) return;
    d = fromTemplate(tpl);
    tipsText = '';
  }

  async function save() {
    if (!valid || saving) return;
    saving = true;
    const out = { ...$state.snapshot(d), tips: lines(tipsText) } as Material;
    const saved = await store.saveMaterial(out, isNew);
    saving = false;
    if (saved) store.library = { section: 'materials', selected: saved.id, creating: false };
  }

  function duplicate() {
    const copy = fromTemplate(d);
    copy.name = t('editor.copyName', { name: d.name });
    copy.description = d.description;
    d = copy;
    tipsText = '';
  }
</script>

<form
  class="editor"
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <header>
    <h2>{isNew ? t('library.newMaterial') : d.name}</h2>
    {#if layer}<span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>{/if}
  </header>
  {#if !isNew}<code class="id">{d.id}</code>{/if}
  {#if isNew}
    <Field label={t('machineEditor.startFrom')} forId="mat-tpl" hint={t('materialEditor.startFromHint')}>
      <select id="mat-tpl" class="input full" onchange={(e) => startFrom((e.currentTarget as HTMLSelectElement).value)}>
        {#each templates as tpl (tpl.id)}<option value={tpl.id} selected={tpl.id === 'material/wood.plywood'}>{tpl.name}</option>{/each}
      </select>
    </Field>
    <p class="note">{t('materialEditor.newNote')}</p>
  {:else if layer === 'base'}
    <p class="note">{t('editor.baseNote')}</p>
  {/if}
  {#if !isNew && constantsChanged}
    <p class="calib"><TriangleAlert size={13} /> {t('materialEditor.calibration')}</p>
  {:else if layer === 'modified' || layer === 'lab'}
    <p class="note">{t('materialEditor.labNote')}</p>
  {/if}

  {#if !store.auth.admin}
    <p class="locked"><Lock size={13} /> {t('materialEditor.locked')}</p>
  {/if}

  <!-- Invité : fiche en lecture seule (fieldset désactivé). -->
  <fieldset class="fields" disabled={!store.auth.admin}>
  <h3>{t('editor.identity')}</h3>
  <Field label={t('form.name')} forId="mat-name"><input id="mat-name" class="input full" required bind:value={d.name} placeholder={t('materialEditor.namePlaceholder')} /></Field>
  <Field label={t('machineEditor.category')} forId="mat-cat">
    <select id="mat-cat" class="input full" bind:value={d.category}>
      {#each CATEGORY_ORDER as c (c)}<option value={c}>{categoryLabel(c)}</option>{/each}
    </select>
  </Field>
  <Field label={t('machineEditor.description')} forId="mat-desc"><input id="mat-desc" class="input full" bind:value={d.description} /></Field>

  <h3>{t('materialEditor.constants')}</h3>
  <Field label={t('materialEditor.vc')} unit="m/min" hint={t('materialEditor.vcHint')}>
    <NumberInput label={t('materialEditor.vc')} value={d.vc_m_min} decimals={0} step={10} onchange={(v) => v && (d.vc_m_min = v)} />
  </Field>
  <Field label={t('materialEditor.kfz')} unit="× Ø" hint={t('materialEditor.kfzHint')}>
    <NumberInput label={t('materialEditor.kfz')} value={d.k_fz} decimals={4} step={0.005} onchange={(v) => v && (d.k_fz = v)} />
  </Field>
  <Field label={t('materialEditor.kap')} unit="× Ø" hint={t('materialEditor.kapHint')}>
    <NumberInput label={t('materialEditor.kap')} value={d.k_ap} decimals={2} step={0.05} onchange={(v) => v && (d.k_ap = v)} />
  </Field>
  <Field label={t('materialEditor.kc')} unit="N/mm²" hint={t('materialEditor.kcHint')}>
    <NumberInput label={t('materialEditor.kc')} value={d.kc_n_mm2} decimals={0} step={5} onchange={(v) => v && (d.kc_n_mm2 = v)} />
  </Field>
  <Field label={t('materialEditor.fzMin')} unit="mm" hint={t('materialEditor.fzMinHint')}>
    <NumberInput label={t('materialEditor.fzMin')} value={d.fz_min_mm} decimals={3} step={0.005} onchange={(v) => v && (d.fz_min_mm = v)} />
  </Field>

  <h3>{t('materialEditor.usage')}</h3>
  <Field label={t('materialEditor.stepover')} unit="%" hint={t('materialEditor.stepoverHint')}>
    <NumberInput
      label={t('materialEditor.stepover')}
      value={Math.round(d.default_stepover * 100)}
      decimals={0}
      step={5}
      onchange={(v) => v && v <= 100 && (d.default_stepover = v / 100)}
    />
  </Field>
  <Field label={t('materialEditor.thickness')} unit="mm" hint={t('materialEditor.thicknessHint')}>
    <NumberInput label={t('materialEditor.thickness')} value={d.default_thickness_mm} placeholder="—" onchange={(v) => (d.default_thickness_mm = v)} />
  </Field>
  <Field label={t('materialEditor.maxProfile')} unit="mm" hint={t('materialEditor.maxProfileHint')}>
    <NumberInput label={t('materialEditor.maxProfile')} value={d.max_profile_thickness_mm} placeholder="—" onchange={(v) => (d.max_profile_thickness_mm = v)} />
  </Field>
  <label class="check">
    <input type="checkbox" bind:checked={d.supervision_required} />
    {t('materialEditor.supervision')}
  </label>

  <h3>{t('materialEditor.tips')}</h3>
  <textarea class="input notes" rows="3" bind:value={tipsText} placeholder={t('editor.notesPlaceholder')}></textarea>
  <Field label={t('readout.source')} forId="mat-src" hint={t('materialEditor.sourceHint')}>
    <input id="mat-src" class="input full" bind:value={d.source} />
  </Field>

  </fieldset>

  {#if !isNew && goodTrials}<p class="note">{t(goodTrials > 1 ? 'materialEditor.goodTrials' : 'materialEditor.goodTrial', { n: goodTrials })}</p>{/if}

  <div class="actions" class:pending={dirty}>
    {#if store.auth.admin}
      <button class="btn primary" type="submit" disabled={!valid || saving || (!isNew && !dirty)}
        ><Save size={14} /> {isNew ? t('common.add') : t('common.save')}</button
      >
      {#if !isNew && dirty}
        <button class="btn quiet" type="button" onclick={revert}><X size={14} /> {t('editor.revert')}</button>
      {/if}
    {:else}
      <button class="btn primary" type="button" onclick={() => store.ensureAdmin()}><Lock size={14} /> {t('materialEditor.unlock')}</button>
    {/if}
    {#if !isNew}
      <button
        class="btn"
        type="button"
        use:tip={t('materialEditor.useHint')}
        onclick={() => {
          store.setMaterial(d.id);
          store.module = 'calc';
        }}><Crosshair size={14} /> {t('editor.use')}</button
      >
      {#if store.auth.admin}
        <button class="btn quiet" type="button" use:tip={t('materialEditor.duplicateHint')} onclick={duplicate}><Copy size={14} /> {t('editor.duplicate')}</button>
      {/if}
    {/if}
  </div>
  {#if store.auth.admin && (layer === 'lab' || layer === 'modified')}
    <div class="danger">
      {#if confirmArchive}
        <span>{layer === 'modified' ? t('editor.dropLabVersion') : t('materialEditor.archiveAsk')}</span>
        <button class="btn quiet" type="button" onclick={() => (confirmArchive = false)}>{t('common.no')}</button>
        <button
          class="btn"
          type="button"
          onclick={() => {
            store.archiveMaterial(d.id);
            store.library = { section: 'materials', selected: layer === 'modified' ? d.id : null, creating: false };
          }}>{t('common.yes')}</button
        >
      {:else}
        <button class="btn quiet" type="button" onclick={() => (confirmArchive = true)}>
          {#if layer === 'modified'}<Undo2 size={14} /> {t('editor.restore')}{:else}<Archive size={14} /> {t('common.archive')}{/if}
        </button>
      {/if}
    </div>
  {/if}
</form>

<style>
  .fields {
    display: contents;
  }

  /* Lecture seule : champs visiblement inactifs. */
  .fields:disabled :global(.input) {
    color: var(--text-secondary);
    background: transparent;
    cursor: default;
  }

  .locked {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: var(--s2);
    border-radius: var(--radius);
    background: var(--bg-control);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .calib {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0;
    padding: var(--s2);
    border-radius: var(--radius);
    background: var(--warning-tint);
    color: var(--warning);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    min-height: var(--control-h);
  }

  .check input {
    accent-color: var(--accent);
  }
</style>
