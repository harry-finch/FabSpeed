<script lang="ts">
  // Origine d'une valeur : petite étiquette icône + texte (jamais la couleur seule).
  import { FlaskConical, Calculator, Pencil, GitBranch, ShieldCheck } from '@lucide/svelte';
  import { evidence, origin as originText } from '../lib/labels';
  import { labelled } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import type { Origin } from '../lib/types';

  /** `validated` : valeur d'un réglage validé (affichée « Validé » au lieu de « Testé »). */
  let { origin, validated = false }: { origin: Origin; validated?: boolean } = $props();
  const shownValidated = $derived(validated && origin === 'lab_preset');
  const Icon = $derived(
    shownValidated
      ? ShieldCheck
      : { lab_preset: FlaskConical, calculated: Calculator, user_override: Pencil, derived_from_preset: GitBranch }[origin],
  );
  const text = $derived(shownValidated ? evidence('validated') : originText(origin));
</script>

<!-- Le libellé peut être masqué quand la place manque (cellule étroite) : il reste dans l'infobulle. -->
<span class="tag {origin}" class:validated={shownValidated} aria-label={text.label} use:tip={labelled(text.label, text.hint)}>
  <Icon size={11} />
  <span class="tag-text">{text.label}</span>
</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: var(--fs-xs);
    font-weight: 600;
    white-space: nowrap;
  }

  .lab_preset {
    color: var(--origin-lab);
    background: var(--origin-lab-tint);
  }

  .lab_preset.validated {
    color: var(--success);
    background: var(--success-tint);
  }

  .calculated {
    color: var(--origin-calc);
    background: var(--neutral-tint);
  }

  .user_override {
    color: var(--origin-user);
    background: var(--origin-user-tint);
  }

  .derived_from_preset {
    color: var(--origin-derived);
    background: var(--warning-tint);
  }
</style>
