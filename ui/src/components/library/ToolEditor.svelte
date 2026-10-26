<script lang="ts">
  // Fiche d'une fraise (inspecteur de la bibliothèque). Une fiche de base
  // modifiée est enregistrée comme version du lab ; l'originale reste intacte.
  import { untrack } from 'svelte';
  import { Save, Copy, Archive, Undo2, Crosshair, Lock, ExternalLink, TriangleAlert, X } from '@lucide/svelte';
  import Field from '../Field.svelte';
  import NumberInput from '../NumberInput.svelte';
  import { store } from '../../lib/store.svelte';
  import { CATEGORY_ORDER, FAMILY_IDS, HELIX_IDS, familyLabel, helixLabel, layer as layerText, rpmUnit } from '../../lib/labels';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { tip } from '../../lib/tip';
  import type { Helix, Tool, ToolFamily } from '../../lib/types';

  let { tool }: { tool: Tool | null } = $props();

  const blank: Tool = {
    id: '',
    name: '',
    reference: '',
    supplier: '',
    url: null,
    family: 'flat_endmill',
    helix: 'upcut',
    tool_material: 'carbide',
    diameter_mm: 6,
    flutes: 2,
    flute_length_mm: 22,
    shank_mm: null,
    overall_length_mm: null,
    tip_angle_deg: null,
    upcut_length_mm: null,
    max_rpm: null,
    supplier_rpm_min: null,
    supplier_rpm_recommended: null,
    recommended_materials: [],
    excluded_materials: [],
    notes: [],
    fabcam_identity: null,
  };

  // Brouillon initialisé une fois par fiche (le parent remonte l'éditeur à chaque sélection).
  const initial: Tool = structuredClone($state.snapshot(untrack(() => tool) ?? blank) as Tool);
  let d = $state<Tool>(structuredClone(initial));
  let notesText = $state(initial.notes.join('\n'));
  let confirmArchive = $state(false);
  let saving = $state(false);

  const isNew = $derived(!d.id);
  const layer = $derived(isNew ? null : store.layerOf('tool', d.id));
  const dirty = $derived(JSON.stringify({ ...d, notes: lines(notesText) }) !== JSON.stringify(initial));
  const valid = $derived(!!d.name.trim() && d.diameter_mm > 0 && d.flute_length_mm > 0 && (d.flutes == null || d.flutes >= 1));
  const materials = $derived(
    [...(store.catalog?.materials ?? [])].sort(
      (a, b) => CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category),
    ),
  );
  const machine = $derived(store.machine);
  /** La fraise demande plus que ce que la broche active sait faire. */
  const aboveSpindle = $derived(
    !!machine && d.supplier_rpm_min != null && d.supplier_rpm_min > machine.rpm_max,
  );
  const safeUrl = $derived(d.url && /^https?:\/\//i.test(d.url) ? d.url : null);
  const goodTrials = $derived(store.catalog?.trials.filter((x) => x.tool_id === d.id && x.outcome === 'good').length ?? 0);

  function lines(text: string): string[] {
    return text
      .split('\n')
      .map((x) => x.trim())
      .filter(Boolean);
  }

  function toggle(list: 'recommended_materials' | 'excluded_materials', id: string) {
    const other = list === 'recommended_materials' ? 'excluded_materials' : 'recommended_materials';
    d[list] = d[list].includes(id) ? d[list].filter((x) => x !== id) : [...d[list], id];
    d[other] = d[other].filter((x) => x !== id);
  }

  async function save() {
    if (!valid || saving) return;
    saving = true;
    const out = { ...$state.snapshot(d), notes: lines(notesText) } as Tool;
    const saved = await store.saveTool(out, isNew);
    saving = false;
    if (saved) store.library = { section: 'tools', selected: saved.id, creating: false };
  }

  function revert() {
    d = structuredClone(initial);
    notesText = initial.notes.join('\n');
  }

  function duplicate() {
    const copy = { ...$state.snapshot(d), id: '', name: t('editor.copyName', { name: d.name }), fabcam_identity: null } as Tool;
    d = copy;
    notesText = copy.notes.join('\n');
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
    <h2>{isNew ? t('library.newTool') : d.name}</h2>
    {#if layer}
      <span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>
    {/if}
  </header>
  {#if !isNew}<code class="id">{d.id}</code>{/if}
  {#if layer === 'base'}
    <p class="note">{t('editor.baseNote')}</p>
  {/if}

  <h3>{t('editor.identity')}</h3>
  <Field label={t('form.name')} forId="t-name"><input id="t-name" class="input full" required bind:value={d.name} placeholder={t('toolEditor.namePlaceholder')} /></Field>
  <Field label={t('assistant.reference')} forId="t-ref"><input id="t-ref" class="input full" bind:value={d.reference} placeholder="FC2D600" /></Field>
  <Field label={t('toolEditor.supplier')} forId="t-sup"><input id="t-sup" class="input full" bind:value={d.supplier} /></Field>
  <Field label={t('toolEditor.link')} forId="t-url">
    <input id="t-url" class="input full" value={d.url ?? ''} oninput={(e) => (d.url = (e.currentTarget as HTMLInputElement).value || null)} placeholder="https://…" />
    {#if safeUrl}
      <a class="btn quiet icon" href={safeUrl} target="_blank" rel="noreferrer" aria-label={t('toolEditor.openLink')} use:tip={t('toolEditor.openLink')}>
        <ExternalLink size={13} />
      </a>
    {/if}
  </Field>

  <h3>{t('toolEditor.geometry')}</h3>
  <Field label={t('form.type')} forId="t-family">
    <select id="t-family" class="input full" bind:value={d.family}>
      {#each FAMILY_IDS as id (id)}<option value={id as ToolFamily}>{familyLabel(id)}</option>{/each}
    </select>
  </Field>
  <Field label={t('form.helix')} forId="t-helix" hint={t('toolEditor.helixHint')}>
    <select id="t-helix" class="input full" bind:value={d.helix}>
      {#each HELIX_IDS as id (id)}<option value={id as Helix}>{helixLabel(id)}</option>{/each}
    </select>
  </Field>
  <Field label={t('toolEditor.material')} forId="t-mat">
    <select id="t-mat" class="input full" bind:value={d.tool_material}>
      <option value="carbide">{t('toolEditor.carbide')}</option>
      <option value="hss">{t('toolEditor.hss')}</option>
    </select>
  </Field>
  <Field label={t('form.diameter')} unit="mm"><NumberInput label={t('form.diameter')} value={d.diameter_mm} onchange={(v) => v && (d.diameter_mm = v)} step={0.5} /></Field>
  <Field label={t('form.flutes')} unit="" hint={t('toolEditor.flutesHint')}>
    <NumberInput label={t('form.flutesLabel')} value={d.flutes} decimals={0} placeholder={t('toolEditor.unknown')} onchange={(v) => (d.flutes = v == null ? null : Math.round(v))} />
  </Field>
  <Field label={t('form.fluteLength')} unit="mm" hint={t('form.fluteLengthHint')}><NumberInput label={t('form.fluteLengthLabel')} value={d.flute_length_mm} onchange={(v) => v && (d.flute_length_mm = v)} /></Field>
  <Field label={t('toolEditor.shank')} unit="mm"><NumberInput label={t('toolEditor.shankLabel')} value={d.shank_mm} placeholder="—" onchange={(v) => (d.shank_mm = v)} /></Field>
  <Field label={t('toolEditor.overall')} unit="mm"><NumberInput label={t('toolEditor.overallLabel')} value={d.overall_length_mm} placeholder="—" onchange={(v) => (d.overall_length_mm = v)} /></Field>
  {#if d.family === 'v_bit'}
    <Field label={t('toolEditor.angle')} unit="°" hint={t('toolEditor.angleHint')}>
      <NumberInput label={t('toolEditor.angleLabel')} value={d.tip_angle_deg} placeholder="—" invalid={!d.tip_angle_deg} onchange={(v) => (d.tip_angle_deg = v && v < 180 ? v : null)} />
    </Field>
  {/if}
  {#if d.family === 'compression'}
    <Field label={t('toolEditor.upcut')} unit="mm" hint={t('toolEditor.upcutHint')}>
      <NumberInput label={t('toolEditor.upcutLabel')} value={d.upcut_length_mm ?? null} placeholder="—" onchange={(v) => (d.upcut_length_mm = v)} />
    </Field>
  {/if}

  <h3>{t('toolEditor.supplierData')}</h3>
  <Field label={t('toolEditor.maxRpm')} unit={rpmUnit()} hint={t('toolEditor.maxRpmHint')}>
    <NumberInput label={t('toolEditor.maxRpmLabel')} value={d.max_rpm} decimals={0} placeholder="—" onchange={(v) => (d.max_rpm = v)} />
  </Field>
  <Field label={t('toolEditor.minRpm')} unit={rpmUnit()} hint={t('toolEditor.minRpmHint')}>
    <NumberInput label={t('toolEditor.minRpmLabel')} value={d.supplier_rpm_min} decimals={0} placeholder="—" onchange={(v) => (d.supplier_rpm_min = v)} />
  </Field>
  {#if aboveSpindle && machine}
    <p class="warn"><TriangleAlert size={13} /> {t('toolEditor.aboveSpindle', { machine: machine.name, max: num(machine.rpm_max, 0), unit: rpmUnit() })}</p>
  {/if}
  <Field label={t('toolEditor.recRpm')} unit={rpmUnit()}>
    <NumberInput label={t('toolEditor.recRpmLabel')} value={d.supplier_rpm_recommended} decimals={0} placeholder="—" onchange={(v) => (d.supplier_rpm_recommended = v)} />
  </Field>

  <h3>{t('toolEditor.materials')} <span class="opt">{t('toolEditor.materialsHint')}</span></h3>
  <div class="chips" role="group" aria-label={t('toolEditor.suited')}>
    {#each materials as m (m.id)}
      <button type="button" class="chip" class:on={d.recommended_materials.includes(m.id)} aria-pressed={d.recommended_materials.includes(m.id)} onclick={() => toggle('recommended_materials', m.id)}>{m.name}</button>
    {/each}
  </div>
  <p class="sub">{t('toolEditor.excludedBySupplier')}</p>
  <div class="chips" role="group" aria-label={t('toolEditor.excluded')}>
    {#each materials as m (m.id)}
      <button type="button" class="chip bad" class:on={d.excluded_materials.includes(m.id)} aria-pressed={d.excluded_materials.includes(m.id)} onclick={() => toggle('excluded_materials', m.id)}>{m.name}</button>
    {/each}
  </div>

  <h3>{t('trialDialog.notes')}</h3>
  <textarea class="input notes" rows="3" bind:value={notesText} placeholder={t('editor.notesPlaceholder')}></textarea>

  {#if !isNew && goodTrials}<p class="note">{t(goodTrials > 1 ? 'toolEditor.goodTrials' : 'toolEditor.goodTrial', { n: goodTrials })}</p>{/if}

  <div class="actions" class:pending={dirty}>
    {#if dirty && !store.auth.admin}<p class="guest"><Lock size={12} /> {t('editor.adminToSave')}</p>{/if}
    <button class="btn primary" type="submit" disabled={!valid || saving || (!isNew && !dirty)}><Save size={14} /> {isNew ? t('common.add') : t('common.save')} {#if !store.auth.admin}<Lock size={12} />{/if}</button>
    {#if !isNew && dirty}
      <button class="btn quiet" type="button" onclick={revert}><X size={14} /> {t('editor.revert')}</button>
    {/if}
    {#if !isNew}
      <button
        class="btn"
        type="button"
        use:tip={t('toolEditor.useHint')}
        onclick={() => {
          store.setTool(d.id);
          store.module = 'calc';
        }}><Crosshair size={14} /> {t('editor.use')}</button
      >
      <button class="btn quiet" type="button" use:tip={t('editor.duplicateHint')} onclick={duplicate}><Copy size={14} /> {t('editor.duplicate')}</button>
    {/if}
  </div>
  {#if layer === 'lab' || layer === 'modified'}
    <div class="danger">
      {#if confirmArchive}
        <span>{layer === 'modified' ? t('editor.dropLabVersion') : t('toolEditor.archiveAsk')}</span>
        <button class="btn quiet" type="button" onclick={() => (confirmArchive = false)}>{t('common.no')}</button>
        <button
          class="btn"
          type="button"
          onclick={() => {
            store.archiveTool(d.id);
            store.library = { section: 'tools', selected: layer === 'modified' ? d.id : null, creating: false };
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
