<script lang="ts">
  // Ajout rapide d'une fraise : les quatre caractéristiques visibles sur la
  // fraise ou son emballage suffisent. Enregistrée dans la bibliothèque du lab
  // (data/lab/tools) ; la fiche complète se modifie dans la bibliothèque.
  import Field from './Field.svelte';
  import NumberInput from './NumberInput.svelte';
  import Segmented from './Segmented.svelte';
  import { store } from '../lib/store.svelte';
  import type { Helix, Tool, ToolFamily } from '../lib/types';
  import { t } from '../lib/i18n.svelte';

  let { onDone }: { onDone: () => void } = $props();

  let name = $state('');
  let diameter = $state<number | null>(6);
  let flutes = $state<number | null>(2);
  let fluteLength = $state<number | null>(22);
  let family = $state<ToolFamily>('flat_endmill');
  let helix = $state<Helix>('upcut');
  let angle = $state<number | null>(60);
  let saving = $state(false);

  const valid = $derived(
    !!diameter && !!flutes && Number.isInteger(flutes) && !!fluteLength && (family !== 'v_bit' || (!!angle && angle < 180)),
  );

  async function save() {
    if (!valid || saving) return;
    const label = name.trim() || `Ø${diameter} ${t(flutes! > 1 ? 'tool.flutes' : 'tool.flute', { n: flutes! })}`;
    const tool: Tool = {
      id: '',
      name: label,
      reference: label,
      supplier: '',
      url: null,
      family,
      helix: ({ flat_endmill: helix, ball_nose: 'upcut', compression: 'compression', v_bit: 'straight' } as Record<string, Helix>)[family] ?? 'unknown',
      tool_material: 'carbide',
      diameter_mm: diameter!,
      flutes: flutes!,
      flute_length_mm: fluteLength!,
      shank_mm: null,
      overall_length_mm: null,
      tip_angle_deg: family === 'v_bit' ? angle : null,
      upcut_length_mm: null,
      max_rpm: null,
      supplier_rpm_min: null,
      supplier_rpm_recommended: null,
      recommended_materials: [],
      excluded_materials: [],
      notes: [],
      fabcam_identity: null,
    };
    saving = true;
    const saved = await store.saveTool(tool, true);
    saving = false;
    if (saved) {
      store.setTool(saved.id);
      onDone();
    }
  }
</script>

<form
  class="form"
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <Field label={t('form.name')} forId="ct-name">
    <input id="ct-name" class="input" style:width="100%" placeholder={t('customTool.namePlaceholder')} bind:value={name} />
  </Field>
  <Field label={t('form.diameter')} unit="mm" hint={t('customTool.diameterHint')}>
    <NumberInput label={t('form.diameter')} value={diameter} onchange={(v) => (diameter = v)} step={0.5} />
  </Field>
  <Field label={t('form.flutes')} unit="" hint={t('customTool.flutesHint')}>
    <NumberInput label={t('form.flutesLabel')} value={flutes} onchange={(v) => (flutes = v && Math.round(v))} decimals={0} />
  </Field>
  <Field label={t('form.fluteLength')} unit="mm" hint={t('form.fluteLengthHint')}>
    <NumberInput label={t('form.fluteLengthLabel')} value={fluteLength} onchange={(v) => (fluteLength = v)} />
  </Field>
  <Field label={t('form.type')}>
    <Segmented
      label={t('form.toolType')}
      value={family}
      onchange={(v) => (family = v)}
      options={[
        { id: 'flat_endmill', label: t('customTool.flat') },
        { id: 'ball_nose', label: t('customTool.ball') },
        { id: 'compression', label: t('customTool.compressionShort'), hint: t('customTool.compression') },
        { id: 'v_bit', label: t('customTool.vShort'), hint: t('customTool.v') },
      ]}
    />
  </Field>
  {#if family === 'v_bit'}
    <Field label={t('toolEditor.angle')} unit="°" hint={t('toolEditor.angleHint')}>
      <NumberInput label={t('toolEditor.angleLabel')} value={angle} decimals={0} onchange={(v) => (angle = v)} />
    </Field>
  {/if}
  {#if family === 'flat_endmill'}
    <Field label={t('form.helix')}>
      <Segmented
        label={t('form.helixDirection')}
        value={helix}
        onchange={(v) => (helix = v)}
        options={[
          { id: 'upcut', label: t('helix.upcut'), hint: t('customTool.upcutHint') },
          { id: 'downcut', label: t('helix.downcut'), hint: t('customTool.downcutHint') },
          { id: 'straight', label: t('helix.straight') },
        ]}
      />
    </Field>
  {/if}
  <div class="actions">
    <button type="button" class="btn quiet" onclick={onDone}>{t('common.cancel')}</button>
    <button type="submit" class="btn primary" disabled={!valid || saving}>{t('common.add')}</button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s2);
    margin: 0 calc(-1 * var(--s1));
    background: var(--bg-workspace);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    margin-top: var(--s1);
  }
</style>
