<script lang="ts">
  // Fiche d'une machine (inspecteur de la bibliothèque). Une nouvelle machine
  // part d'un profil existant ; chaque limite porte sa fiabilité (vérifiée,
  // estimation, à vérifier) pour ne jamais présenter une valeur inventée
  // comme une donnée constructeur.
  import { untrack } from 'svelte';
  import { Save, Copy, Archive, Undo2, Crosshair, Lock, X } from '@lucide/svelte';
  import Field from '../Field.svelte';
  import NumberInput from '../NumberInput.svelte';
  import { store } from '../../lib/store.svelte';
  import { VALUE_STATUS_IDS, layer as layerText, rpmUnit, valueStatusLabel } from '../../lib/labels';
  import { t } from '../../lib/i18n.svelte';
  import { tip } from '../../lib/tip';
  import type { Machine, ValueStatus } from '../../lib/types';

  let { machine }: { machine: Machine | null } = $props();

  const KEYS = ['rpm_min', 'rpm_max', 'spindle_power_w', 'max_feed_mm_min', 'max_lateral_force_n', 'chipload_factor'] as const;

  /** Nouvelle machine : copie d'un profil, valeurs marquées « à vérifier ». */
  function fromTemplate(tpl: Machine): Machine {
    const copy = structuredClone($state.snapshot(tpl) as Machine);
    copy.id = '';
    copy.name = '';
    copy.notes = [];
    copy.sources = Object.fromEntries(
      KEYS.map((k) => [k, { status: 'to_verify' as ValueStatus, note: t('machineEditor.fromProfile', { name: tpl.name }) }]),
    );
    return copy;
  }

  const templates = store.catalog?.machines ?? [];
  const source = untrack(() => machine);
  const initial: Machine = source
    ? structuredClone($state.snapshot(source) as Machine)
    : fromTemplate(templates.find((m) => m.id === 'generic-hobby-router') ?? templates[0]);
  let d = $state<Machine>(structuredClone(initial));
  let notesText = $state(initial.notes.join('\n'));
  let confirmArchive = $state(false);
  let saving = $state(false);

  const isNew = $derived(!d.id);
  const layer = $derived(isNew ? null : store.layerOf('machine', d.id));
  const dirty = $derived(JSON.stringify({ ...d, notes: lines(notesText) }) !== JSON.stringify(initial));
  const valid = $derived(
    !!d.name.trim() &&
      d.rpm_min > 0 &&
      d.rpm_max >= d.rpm_min &&
      d.spindle_power_w > 0 &&
      d.max_feed_mm_min > 0 &&
      d.max_lateral_force_n > 0 &&
      d.chipload_factor > 0,
  );

  function revert() {
    d = structuredClone(initial);
    notesText = initial.notes.join('\n');
  }

  function lines(text: string): string[] {
    return text
      .split('\n')
      .map((x) => x.trim())
      .filter(Boolean);
  }

  const statusOf = (k: string): ValueStatus => d.sources[k]?.status ?? 'verified';
  function setStatus(k: string, status: ValueStatus) {
    d.sources = { ...d.sources, [k]: { status, note: d.sources[k]?.note ?? '' } };
  }

  function startFrom(id: string) {
    const tpl = templates.find((m) => m.id === id);
    if (!tpl) return;
    d = fromTemplate(tpl);
    notesText = '';
  }

  async function save() {
    if (!valid || saving) return;
    saving = true;
    const out = { ...$state.snapshot(d), notes: lines(notesText) } as Machine;
    const saved = await store.saveMachine(out, isNew);
    saving = false;
    if (saved) store.library = { section: 'machines', selected: saved.id, creating: false };
  }

  function duplicate() {
    const copy = fromTemplate(d);
    copy.name = t('editor.copyName', { name: d.name });
    d = copy;
    notesText = '';
  }
</script>

{#snippet status(k: string)}
  <select
    class="input status {statusOf(k)}"
    aria-label={t('machineEditor.reliability')}
    value={statusOf(k)}
    use:tip={d.sources[k]?.note || t('machineEditor.reliability')}
    onchange={(e) => setStatus(k, (e.currentTarget as HTMLSelectElement).value as ValueStatus)}
  >
    {#each VALUE_STATUS_IDS as id (id)}<option value={id}>{valueStatusLabel(id)}</option>{/each}
  </select>
{/snippet}

<form
  class="editor"
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <header>
    <h2>{isNew ? t('library.newMachine') : d.name}</h2>
    {#if layer}<span class="layer {layer}" use:tip={layerText(layer).hint}>{layerText(layer).label}</span>{/if}
  </header>
  {#if !isNew}<code class="id">{d.id}</code>{/if}
  {#if isNew}
    <Field label={t('machineEditor.startFrom')} forId="m-tpl" hint={t('machineEditor.startFromHint')}>
      <select id="m-tpl" class="input full" onchange={(e) => startFrom((e.currentTarget as HTMLSelectElement).value)}>
        {#each templates as tpl (tpl.id)}<option value={tpl.id} selected={tpl.id === 'generic-hobby-router'}>{tpl.name}</option>{/each}
      </select>
    </Field>
  {:else if layer === 'base'}
    <p class="note">{t('editor.baseNote')}</p>
  {/if}

  <h3>{t('editor.identity')}</h3>
  <Field label={t('form.name')} forId="m-name"><input id="m-name" class="input full" required bind:value={d.name} placeholder={t('machineEditor.namePlaceholder')} /></Field>
  <Field label={t('machineEditor.category')} forId="m-cat">
    <input id="m-cat" class="input full" list="m-cats" bind:value={d.category} />
    <datalist id="m-cats"><option value={t('machineEditor.catGantry')}></option><option value={t('machineEditor.catDesktop')}></option><option value={t('machineEditor.catMill')}></option></datalist>
  </Field>
  <Field label={t('machineEditor.description')} forId="m-desc"><input id="m-desc" class="input full" bind:value={d.description} /></Field>

  <h3>{t('loads.spindle')}</h3>
  <Field label={t('toolEditor.minRpm')} unit={rpmUnit()}>
    <NumberInput label={t('machineEditor.minRpmLabel')} value={d.rpm_min} decimals={0} onchange={(v) => v && (d.rpm_min = v)} />
    {@render status('rpm_min')}
  </Field>
  <Field label={t('toolEditor.maxRpm')} unit={rpmUnit()}>
    <NumberInput label={t('toolEditor.maxRpmLabel')} value={d.rpm_max} decimals={0} onchange={(v) => v && (d.rpm_max = v)} />
    {@render status('rpm_max')}
  </Field>
  <Field label={t('machineEditor.power')} unit="W" hint={t('machineEditor.powerHint')}>
    <NumberInput label={t('machineEditor.powerLabel')} value={d.spindle_power_w} decimals={0} onchange={(v) => v && (d.spindle_power_w = v)} />
    {@render status('spindle_power_w')}
  </Field>
  <Field label={t('machineEditor.rated')} unit={rpmUnit()} hint={t('machineEditor.ratedHint')}>
    <NumberInput label={t('machineEditor.rated')} value={d.spindle_rated_rpm} decimals={0} placeholder="—" onchange={(v) => (d.spindle_rated_rpm = v)} />
  </Field>

  <h3>{t('machineEditor.axes')}</h3>
  <Field label={t('machineEditor.maxFeed')} unit="mm/min" hint={t('machineEditor.maxFeedHint')}>
    <NumberInput label={t('machineEditor.maxFeedLabel')} value={d.max_feed_mm_min} decimals={0} onchange={(v) => v && (d.max_feed_mm_min = v)} />
    {@render status('max_feed_mm_min')}
  </Field>
  <Field label={t('machineEditor.maxPlunge')} unit="mm/min">
    <NumberInput label={t('machineEditor.maxPlungeLabel')} value={d.max_plunge_mm_min} decimals={0} placeholder="—" onchange={(v) => (d.max_plunge_mm_min = v)} />
  </Field>
  <Field label={t('machineEditor.force')} unit="N" hint={t('machineEditor.forceHint')}>
    <NumberInput label={t('machineEditor.forceLabel')} value={d.max_lateral_force_n} decimals={0} onchange={(v) => v && (d.max_lateral_force_n = v)} />
    {@render status('max_lateral_force_n')}
  </Field>
  <Field label={t('machineEditor.stiffness')} unit="×" hint={t('machineEditor.stiffnessHint')}>
    <NumberInput label={t('machineEditor.stiffnessLabel')} value={d.chipload_factor} step={0.05} onchange={(v) => v && (d.chipload_factor = v)} />
    {@render status('chipload_factor')}
  </Field>
  <Field label={t('machineEditor.controller')} forId="m-ctrl" hint={t('machineEditor.controllerHint')}>
    <select id="m-ctrl" class="input full" value={d.controller ?? ''} onchange={(e) => (d.controller = (e.currentTarget as HTMLSelectElement).value || null)}>
      <option value="">{t('machineEditor.other')}</option>
      <option value="shopbot">ShopBot</option>
    </select>
  </Field>
  <Field label={t('machineEditor.travel')} unit="mm">
    <div class="xyz">
      {#each [0, 1, 2] as i (i)}
        <NumberInput
          label={t('machineEditor.axisTravel', { axis: 'XYZ'[i] })}
          value={d.work_area_mm?.[i] ?? null}
          decimals={0}
          placeholder="—"
          onchange={(v) => {
            const area: [number, number, number] = [...(d.work_area_mm ?? [0, 0, 0])] as [number, number, number];
            area[i] = v ?? 0;
            d.work_area_mm = area.some((x) => x > 0) ? area : null;
          }}
        />
      {/each}
    </div>
  </Field>

  <h3>{t('trialDialog.notes')}</h3>
  <textarea class="input notes" rows="3" bind:value={notesText} placeholder={t('machineEditor.notesPlaceholder')}></textarea>

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
        use:tip={t('machineEditor.useHint')}
        onclick={() => {
          store.setMachine(d.id);
          store.module = 'calc';
        }}><Crosshair size={14} /> {t('editor.use')}</button
      >
      <button class="btn quiet" type="button" use:tip={t('machineEditor.duplicateHint')} onclick={duplicate}><Copy size={14} /> {t('editor.duplicate')}</button>
    {/if}
  </div>
  {#if layer === 'lab' || layer === 'modified'}
    <div class="danger">
      {#if confirmArchive}
        <span>{layer === 'modified' ? t('editor.dropLabVersion') : t('machineEditor.archiveAsk')}</span>
        <button class="btn quiet" type="button" onclick={() => (confirmArchive = false)}>{t('common.no')}</button>
        <button
          class="btn"
          type="button"
          onclick={() => {
            store.archiveMachine(d.id);
            store.library = { section: 'machines', selected: layer === 'modified' ? d.id : null, creating: false };
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
