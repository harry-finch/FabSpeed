<script lang="ts">
  // Pictogramme d'opération vu de dessus : pièce en trait plein, parcours de
  // fraise en pointillés, fraise en cercle.
  import type { OperationKind } from '../lib/types';

  let { op, size = 20 }: { op: OperationKind; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if op === 'profile'}
    <rect x="6" y="6" width="12" height="12" rx="2" />
    <rect x="3" y="3" width="18" height="18" rx="4" stroke-dasharray="2 2.6" stroke-width="1.2" />
    <circle cx="21" cy="12" r="1.8" fill="currentColor" stroke="none" />
  {:else if op === 'pocket'}
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="M6 8h12M6 12h12M6 16h12" stroke-dasharray="2 2.4" stroke-width="1.2" />
  {:else if op === 'full_slot'}
    <path d="M2 8.5h20M2 15.5h20" />
    <path d="M3 12h13" stroke-dasharray="2 2.4" stroke-width="1.2" />
    <circle cx="18.5" cy="12" r="3.5" />
  {:else if op === 'finish'}
    <path d="M7 3v18" stroke-width="2.4" />
    <path d="M11 3v18" stroke-dasharray="2 2.4" stroke-width="1.2" />
    <circle cx="14" cy="15" r="3" />
  {:else if op === 'surfacing'}
    <path d="M3 5h18v4.5H3v4.5h18v4.5H3" stroke-dasharray="2 2.2" stroke-width="1.2" />
    <rect x="2" y="2" width="20" height="20" rx="2" stroke-width="1.2" opacity="0.6" />
  {:else}
    <path d="M3 16c2-6 4-9 5-6s-1 7 1 6 3-8 5-8 0 9 2 9 3-4 5-6" />
    <path d="M2 20h20" stroke-width="1.2" opacity="0.6" />
  {/if}
</svg>
