<script lang="ts">
  // Pictogramme de fraise (vue de côté) : famille et sens d'hélice.
  // Même trait que Lucide pour rester homogène avec les autres icônes.
  import type { Helix, ToolFamily } from '../lib/types';

  let { family, helix = 'unknown', size = 16 }: { family: ToolFamily; helix?: Helix; size?: number } = $props();

  // Traits d'hélice : « / » pour upcut, « \ » pour downcut.
  const slope = $derived(helix === 'downcut' ? -1 : 1);
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
  <rect x="9.5" y="1.5" width="5" height="7" rx="0.5" />
  {#if family === 'v_bit'}
    <path d="M8 9h8l-4 13z" />
  {:else if family === 'ball_nose'}
    <path d="M8.5 9h7v8a3.5 3.5 0 0 1-7 0z" />
    <path d={`M8.5 ${12 + slope * 1.5}l7 ${-slope * 3}`} />
  {:else if family === 'diamond_cut'}
    <rect x="8.5" y="9" width="7" height="12" rx="0.5" />
    <path d="M8.5 13l7 4M8.5 17l7-4M8.5 19.5l3.5 1.5M15.5 19.5l-3.5 1.5M8.5 10.5l3.5-1.5M15.5 10.5l-3.5-1.5" stroke-width="1" />
  {:else}
    <rect x="8.5" y="9" width="7" height="12" rx="0.5" />
    {#if helix === 'compression'}
      <path d="M8.5 13l7-2.5M8.5 18.5l7 2.5" />
    {:else}
      <path d={`M8.5 ${13 + slope * 1.5}l7 ${-slope * 3}M8.5 ${18 + slope * 1.5}l7 ${-slope * 3}`} />
    {/if}
  {/if}
</svg>
