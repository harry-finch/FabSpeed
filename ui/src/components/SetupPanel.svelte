<script lang="ts">
  // Panneau gauche : la coupe à réaliser, dans l'ordre où un débutant y pense.
  // Matière → Fraise → Opération → Conditions. Le reste est replié.
  import { ShieldAlert, Pencil, ExternalLink, RotateCcw } from '@lucide/svelte';
  import Section from './Section.svelte';
  import Field from './Field.svelte';
  import NumberInput from './NumberInput.svelte';
  import Segmented from './Segmented.svelte';
  import MaterialPicker from './MaterialPicker.svelte';
  import ToolPicker from './ToolPicker.svelte';
  import CustomToolForm from './CustomToolForm.svelte';
  import OperationGlyph from './OperationGlyph.svelte';
  import { store, autoDepth } from '../lib/store.svelte';
  import { familyLabel, modes, operationAllowed, operations, workholdings } from '../lib/labels';
  import { t } from '../lib/i18n.svelte';
  import { fromDisplayLength, lengthDecimals, lengthUnit, numTrim, toDisplayLength } from '../lib/format';
  import { tip } from '../lib/tip';

  let creating = $state(false);

  const u = $derived(store.units);
  const lu = $derived(lengthUnit(u));
  const dec = $derived(lengthDecimals(u));
  const s = $derived(store.setup);
  const material = $derived(store.material);
  const tool = $derived(store.tool);

  const disp = (mm: number | null) => (mm == null ? null : Number(toDisplayLength(mm, u).toFixed(4)));
  const back = (v: number | null) => (v == null ? null : fromDisplayLength(v, u));
  const usesStepover = $derived(s.operation === 'pocket' || s.operation === 'surfacing');
  const defaultStepover = $derived(s.operation === 'surfacing' ? 0.5 : (material?.default_stepover ?? 0.4));
  const stepoverPct = $derived(Math.round((s.stepoverRatio ?? defaultStepover) * 100));
  const depthIsThickness = $derived(['profile', 'finish'].includes(s.operation));
  const defaultStickout = $derived(tool ? tool.flute_length_mm + 3 : null);
  // La sortie de pince remonte sous la fraise dès qu'elle limite le réglage (ou a été saisie).
  const stickoutUp = $derived(s.stickoutMm != null || !!store.reco?.limits.includes('slenderness'));
  const stickoutLimits = $derived(!!store.reco?.limits.includes('slenderness'));
</script>

{#snippet stickoutField()}
  <Field label={t('setup.stickout')} unit={lu} hint={t('setup.stickoutHint')}>
    <NumberInput
      label={t('setup.stickout')}
      value={disp(s.stickoutMm)}
      placeholder={store.reco?.metrics
        ? numTrim(toDisplayLength(store.reco.metrics.stickout_mm, u), dec)
        : defaultStickout
          ? numTrim(toDisplayLength(defaultStickout, u), dec)
          : ''}
      decimals={dec}
      onchange={(v) => (store.setup.stickoutMm = back(v))}
    />
  </Field>
{/snippet}

<aside class="panel" aria-label={t('setup.title')}>
  <div class="head">{t('setup.title')}</div>
  <div class="scroll">
    <Section id="material" title={t('setup.material')} step={1}>
      <MaterialPicker />
      <Field label={t('setup.thickness')} unit={lu} hint={t('setup.thicknessHint')}>
        <NumberInput
          label={t('setup.thicknessLabel')}
          value={disp(s.thicknessMm)}
          decimals={dec}
          onchange={(v) => v && (store.setup.thicknessMm = back(v)!)}
        />
      </Field>
      {#if material?.supervision_required}
        <div class="notice warn"><ShieldAlert size={14} /> {t('setup.supervision')}</div>
      {/if}
    </Section>

    <Section id="tool" title={t('setup.tool')} step={2}>
      {#snippet actions()}
        {#if tool && !creating}
          {#if tool.url}
            <a class="btn quiet icon" href={tool.url} target="_blank" rel="noreferrer" use:tip={t('setup.supplierSheet')}>
              <ExternalLink size={13} />
            </a>
          {/if}
          <button
            class="btn quiet icon"
            type="button"
            use:tip={t('setup.editTool')}
            onclick={() => store.openLibrary('tools', tool.id)}
          >
            <Pencil size={13} />
          </button>
        {/if}
      {/snippet}
      {#if creating}
        <CustomToolForm onDone={() => (creating = false)} />
      {:else}
        <ToolPicker onCreate={() => (creating = true)} />
        {#if stickoutUp}
          {@render stickoutField()}
          {#if stickoutLimits}<div class="notice warn small">{t('setup.stickoutLimits')}</div>{/if}
        {/if}
      {/if}
    </Section>

    <Section id="operation" title={t('setup.operation')} step={3}>
      <div class="ops" role="radiogroup" aria-label={t('setup.operation')}>
        {#each operations() as op (op.id)}
          {@const allowed = operationAllowed(op.id, tool?.family)}
          <button
            type="button"
            role="radio"
            aria-checked={s.operation === op.id}
            class="op"
            class:on={s.operation === op.id}
            aria-disabled={!allowed}
            use:tip={allowed ? op.hint : t('setup.opNotForTool', { family: familyLabel(tool!.family) })}
            onclick={() => allowed && store.setOperation(op.id)}
          >
            <OperationGlyph op={op.id} />
            <span>{op.label}</span>
          </button>
        {/each}
      </div>
      <!-- Ce que fait l'opération choisie, toujours visible (détourage et rainure se ressemblent). -->
      <p class="op-desc">{operations().find((o) => o.id === s.operation)?.hint}</p>
      <Field
        label={t('setup.depth')}
        unit={lu}
        hint={depthIsThickness ? t('setup.depthHintProfile') : t('setup.depthHint')}
      >
        <NumberInput
          label={t('setup.depthHint')}
          value={disp(s.depthMm)}
          placeholder={numTrim(toDisplayLength(autoDepth(s.operation, s.thicknessMm), u), dec)}
          decimals={dec}
          onchange={(v) => (store.setup.depthMm = back(v))}
        />
        {#if s.depthMm != null}
          <button class="btn quiet icon" type="button" use:tip={t('setup.autoDepth')} onclick={() => (store.setup.depthMm = null)}>
            <RotateCcw size={13} />
          </button>
        {/if}
      </Field>
      {#if depthIsThickness && s.depthMm == null}
        <p class="follow">{t('setup.depthFollows')}</p>
      {/if}
      {#if usesStepover}
        <Field
          label={t('setup.stepover')}
          unit="%"
          hint={t('setup.stepoverHint')}
        >
          <input
            class="range"
            type="range"
            min="5"
            max="100"
            step="5"
            value={stepoverPct}
            aria-label={t('setup.stepoverPct')}
            oninput={(e) => (store.setup.stepoverRatio = Number((e.currentTarget as HTMLInputElement).value) / 100)}
          />
          <NumberInput
            label={t('setup.stepover')}
            value={stepoverPct}
            width="52px"
            decimals={0}
            onchange={(v) => (store.setup.stepoverRatio = v == null ? null : Math.min(100, Math.max(5, v)) / 100)}
          />
        </Field>
      {/if}
    </Section>

    <Section id="conditions" title={t('setup.conditions')} step={4}>
      <Field label={t('setup.workholding')} forId="workholding" hint={t('setup.workholdingHint')}>
        <select
          id="workholding"
          class="input select"
          value={s.workholding}
          onchange={(e) => (store.setup.workholding = (e.currentTarget as HTMLSelectElement).value as typeof s.workholding)}
        >
          {#each workholdings() as w (w.id)}
            <option value={w.id}>{w.label}</option>
          {/each}
        </select>
      </Field>
      <Field label={t('setup.mode')} hint={t('setup.modeHint')}>
        <Segmented label={t('setup.mode')} options={modes()} value={s.aggressiveness} onchange={(v) => (store.setup.aggressiveness = v)} />
      </Field>
    </Section>

    <Section id="advanced" title={t('setup.advanced')} open={false}>
      {#if !stickoutUp}{@render stickoutField()}{/if}
      <Field label={t('setup.maxDoc')} unit={lu} hint={t('setup.maxDocHint')}>
        <NumberInput
          label={t('setup.maxDocLabel')}
          value={disp(s.maxDocMm)}
          placeholder={t('setup.none')}
          decimals={dec}
          onchange={(v) => (store.setup.maxDocMm = back(v))}
        />
      </Field>
      <Field label={t('setup.cutLength')} unit={lu} hint={t('setup.cutLengthHint')}>
        <NumberInput
          label={t('setup.cutLengthLabel')}
          value={disp(s.cutLengthMm)}
          placeholder="—"
          decimals={0}
          onchange={(v) => (store.setup.cutLengthMm = back(v))}
        />
      </Field>
      <label class="check">
        <input type="checkbox" bind:checked={store.setup.usePresets} />
        {t('setup.usePresets')}
      </label>
    </Section>
  </div>
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    height: 32px;
    padding: 0 var(--s3);
    border-bottom: 1px solid var(--divider);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .scroll {
    overflow-y: auto;
    flex: 1;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
    font-weight: 500;
  }

  .notice.warn {
    color: var(--warning);
  }

  .notice.small {
    font-size: var(--fs-xs);
  }

  .follow {
    margin: -2px 0 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .ops {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }

  .op {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 7px 2px 5px;
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 500;
  }

  .op:hover {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }

  .op[aria-disabled='true'] {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .op[aria-disabled='true']:hover {
    color: var(--text-secondary);
    border-color: var(--border);
  }

  .op-desc {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    line-height: 1.45;
  }

  .op.on {
    color: var(--text-primary);
    border-color: var(--accent);
    background: var(--accent-tint);
  }

  .select {
    width: 100%;
    padding-right: 4px;
  }

  .range {
    flex: 1;
    min-width: 0;
    accent-color: var(--accent);
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
