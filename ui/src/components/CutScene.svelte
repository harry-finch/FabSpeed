<script lang="ts">
  // Schéma de la coupe, à l'échelle : vue en coupe (matière, passes, fraise,
  // sortie de pince) et vue de dessus (engagement radial). Les rayures de la
  // fraise défilent dans le sens d'évacuation des copeaux (upcut / downcut).
  import type { OperationKind, Tool } from '../lib/types';
  import { fmtLength, lengthUnit, type UnitSystem } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  let {
    tool,
    thicknessMm,
    depthMm,
    docMm,
    passes,
    stickoutMm,
    stepoverRatio,
    operation,
    materialColor,
    feedLabel,
    rpmLabel,
    units,
    cuttingDiameterMm,
  }: {
    tool: Tool;
    thicknessMm: number;
    depthMm: number;
    docMm: number;
    passes: number;
    stickoutMm: number;
    stepoverRatio: number;
    operation: OperationKind;
    materialColor: string;
    feedLabel: string;
    rpmLabel: string;
    units: UnitSystem;
    /** Diamètre qui coupe (largeur gravée pour une fraise en V). */
    cuttingDiameterMm?: number;
  } = $props();

  // Vue en coupe : cadrée au plus près pour occuper la place ; la vue de dessus
  // est un second dessin, calé à droite de la zone.
  const W = 490;
  const H = 300;
  const TOP = 62; // face de la pince (place pour la pince et l'étiquette de rotation)
  const BOTTOM = 270;
  const SLAB_X0 = 28;
  const SLAB_X1 = 470;
  const TOOL_X = 330;

  // Échelle commune (px/mm) : la fraise sortie + l'épaisseur doivent tenir.
  const reach = $derived(Math.max(stickoutMm - depthMm, 0) + thicknessMm);
  const scale = $derived(Math.min(11, (BOTTOM - TOP) / Math.max(reach, 1)));
  const slabTop = $derived(BOTTOM - thicknessMm * scale);
  const tipY = $derived(slabTop + Math.min(depthMm, thicknessMm + 2) * scale);
  const colletY = $derived(tipY - stickoutMm * scale);
  const r = $derived(Math.max((tool.diameter_mm * scale) / 2, 3));
  const fluteTop = $derived(tipY - tool.flute_length_mm * scale);
  const shankR = $derived(Math.max(((tool.shank_mm ?? tool.diameter_mm) * scale) / 2, 2.5));
  const tooShort = $derived(tool.flute_length_mm + 1e-6 < Math.min(depthMm, thicknessMm));
  const cutDepthY = $derived(Math.min(tipY, BOTTOM));
  // Assez de hauteur usinée pour écrire dans le sillon ; sinon au-dessus.
  const roomy = $derived(cutDepthY - slabTop > 40);
  const passLines = $derived(
    Array.from({ length: Math.max(passes - 1, 0) }, (_, i) => slabTop + (i + 1) * docMm * scale).filter(
      (y) => y < cutDepthY - 1,
    ),
  );
  const helixDir = $derived(tool.helix === 'downcut' ? 1 : -1);
  const ball = $derived(tool.family === 'ball_nose');
  const vbit = $derived(tool.family === 'v_bit');
  // Hauteur du cône d'une fraise en V selon son angle (sinon toute la longueur utile).
  const coneH = $derived.by(() => {
    const a = tool.tip_angle_deg;
    if (!vbit || !a || a <= 0 || a >= 180) return tipY - fluteTop;
    return Math.min((tool.diameter_mm / (2 * Math.tan((a * Math.PI) / 360))) * scale, tipY - fluteTop);
  });
  const lu = $derived(lengthUnit(units));

  // Vue de dessus : engagement.
  const INSET = { x: 4, y: 4, w: 118, h: 118 };
  const ir = 22;
  const ae = $derived(
    operation === 'profile' || operation === 'full_slot' || operation === 'engraving'
      ? 1
      : operation === 'finish'
        ? 0.1
        : stepoverRatio,
  );
  const engagedWidth = $derived(2 * ir * ae);
  const cx = INSET.w / 2 + 8;
  const cy = INSET.h / 2 + 10;
</script>

<div class="scene">
<svg class="side" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={t('scene.label')}>
  <defs>
    <pattern id="grid" width="16" height="16" patternUnits="userSpaceOnUse">
      <circle cx="1" cy="1" r="0.7" style:fill="var(--scene-grid)" />
    </pattern>
    <pattern id="layers" width="8" height="6" patternUnits="userSpaceOnUse">
      <path d="M0 5.5H8" style:stroke="var(--scene-layers)" stroke-width="1" />
    </pattern>
    <clipPath id="flutes">
      {#if ball}
        <path d={`M${TOOL_X - r} ${fluteTop} H${TOOL_X + r} V${tipY - r} A${r} ${r} 0 0 1 ${TOOL_X - r} ${tipY - r} Z`} />
      {:else if vbit}
        <path d={`M${TOOL_X - r} ${fluteTop} H${TOOL_X + r} V${tipY - coneH} L${TOOL_X} ${tipY} L${TOOL_X - r} ${tipY - coneH} Z`} />
      {:else}
        <rect x={TOOL_X - r} y={fluteTop} width={2 * r} height={tipY - fluteTop} />
      {/if}
    </clipPath>
  </defs>

  <rect width={W} height={H} fill="url(#grid)" />

  <!-- Matière (coupe) -->
  <rect x={SLAB_X0} y={slabTop} width={SLAB_X1 - SLAB_X0} height={BOTTOM - slabTop} fill={materialColor} opacity="0.78" />
  <rect x={SLAB_X0} y={slabTop} width={SLAB_X1 - SLAB_X0} height={BOTTOM - slabTop} fill="url(#layers)" />
  <!-- Zone déjà usinée, derrière la fraise -->
  <rect x={SLAB_X0} y={slabTop} width={TOOL_X - SLAB_X0} height={cutDepthY - slabTop} class="removed" />
  <rect x={SLAB_X0} y={slabTop} width={TOOL_X - SLAB_X0} height={cutDepthY - slabTop} fill={materialColor} opacity="0.1" />
  {#if roomy}<text x={SLAB_X0 + 8} y={cutDepthY - 8} class="lbl faint">{t('scene.groove')}</text>{/if}
  {#each passLines as y, i (i)}
    <line x1={SLAB_X0} x2={TOOL_X} y1={y} y2={y} class="pass" />
  {/each}
  <line x1={SLAB_X0} x2={SLAB_X1} y1={BOTTOM} y2={BOTTOM} class="table" />
  <text x={SLAB_X0} y={BOTTOM + 16} class="lbl">{t('scene.spoilboard')}</text>

  <!-- Repère des passes -->
  {#if passes > 0}
    <g class="dim">
      <line x1={SLAB_X0 - 10} x2={SLAB_X0 - 10} y1={slabTop} y2={cutDepthY} />
      <line x1={SLAB_X0 - 14} x2={SLAB_X0 - 6} y1={slabTop} y2={slabTop} />
      <line x1={SLAB_X0 - 14} x2={SLAB_X0 - 6} y1={cutDepthY} y2={cutDepthY} />
    </g>
    <text x={SLAB_X0 + 8} y={roomy ? slabTop + Math.min(docMm * scale, 40) / 2 + 4 : slabTop - 8} class="lbl strong">
      {t(passes > 1 ? 'readout.passesN' : 'readout.passN', { n: passes })} × {fmtLength(docMm, units)} {lu}
    </text>
  {/if}

  <!-- Fraise : pince, queue, partie coupante -->
  <path
    d={`M${TOOL_X - shankR - 14} ${colletY - 30} H${TOOL_X + shankR + 14} L${TOOL_X + shankR + 6} ${colletY} H${TOOL_X - shankR - 6} Z`}
    class="collet"
  />
  <rect x={TOOL_X - shankR} y={colletY} width={2 * shankR} height={Math.max(fluteTop - colletY, 0)} class="shank" />
  <g clip-path="url(#flutes)">
    <rect x={TOOL_X - r} y={fluteTop} width={2 * r} height={tipY - fluteTop} class="flute" />
    {#if tool.family !== 'diamond_cut'}
      <g class="stripes" style:--dir={helixDir}>
        {#each Array.from({ length: Math.ceil((tipY - fluteTop) / 9) + 4 }, (_, i) => i) as i (i)}
          <line
            x1={TOOL_X - r}
            x2={TOOL_X + r}
            y1={fluteTop + i * 9 - 18 + (helixDir < 0 ? r * 0.9 : 0)}
            y2={fluteTop + i * 9 - 18 + (helixDir < 0 ? 0 : r * 0.9)}
          />
        {/each}
      </g>
    {:else}
      <rect x={TOOL_X - r} y={fluteTop} width={2 * r} height={tipY - fluteTop} class="knurl" />
    {/if}
  </g>
  {#if vbit && cuttingDiameterMm}
    <text x={TOOL_X + r + 10} y={Math.min(tipY, BOTTOM) + 14} class="lbl strong">
      {t('scene.engravedWidth', { v: fmtLength(cuttingDiameterMm, units), u: lu })}
    </text>
  {/if}
  {#if tooShort}
    <rect x={TOOL_X - shankR - 1} y={slabTop} width={2 * shankR + 2} height={Math.max(fluteTop - slabTop, 0)} class="danger" />
    <text x={TOOL_X + r + 10} y={slabTop + 14} class="lbl danger-text">{t('scene.tooShort')}</text>
  {/if}

  <!-- Sortie de pince -->
  <g class="dim">
    <line x1={TOOL_X - r - 16} x2={TOOL_X - r - 16} y1={colletY} y2={tipY} />
    <line x1={TOOL_X - r - 20} x2={TOOL_X - r - 12} y1={colletY} y2={colletY} />
    <line x1={TOOL_X - r - 20} x2={TOOL_X - r - 12} y1={tipY} y2={tipY} />
  </g>
  <text x={TOOL_X - r - 22} y={(colletY + Math.min(tipY, slabTop)) / 2} class="lbl" text-anchor="end">
    {t('scene.stickout', { v: fmtLength(stickoutMm, units, 0), u: lu })}
  </text>

  <!-- Rotation et avance -->
  <path
    d={`M${TOOL_X - shankR - 12} ${colletY + 12} A${shankR + 12} 5 0 1 0 ${TOOL_X + shankR + 12} ${colletY + 12}`}
    class="arrow"
    marker-end="url(#head)"
  />
  <text x={TOOL_X} y={colletY - 38} class="lbl strong" text-anchor="middle">{rpmLabel}</text>
  <line x1={TOOL_X + r + 14} x2={TOOL_X + r + 70} y1={slabTop - 12} y2={slabTop - 12} class="arrow" marker-end="url(#head)" />
  <text x={TOOL_X + r + 14} y={slabTop - 20} class="lbl strong">{feedLabel}</text>
  <defs>
    <marker id="head" viewBox="0 0 8 8" refX="6" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L8 4L0 8z" style:fill="var(--scene-arrow)" />
    </marker>
  </defs>

</svg>

<!-- Vue de dessus : engagement radial -->
<svg class="top" viewBox="0 0 126 146" role="img" aria-label={t('scene.topView')}>
  <defs>
    <marker id="head-top" viewBox="0 0 8 8" refX="6" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L8 4L0 8z" style:fill="var(--scene-arrow)" />
    </marker>
  </defs>
  <g transform={`translate(${INSET.x} ${INSET.y})`}>
    <rect width={INSET.w} height={INSET.h} rx="4" class="inset" />
    <text x="8" y="15" class="lbl">{t('scene.topView')}</text>
    <clipPath id="insetclip"><rect width={INSET.w} height={INSET.h} rx="4" /></clipPath>
    <clipPath id="toolclip"><circle cx={cx} cy={cy} r={ir} /></clipPath>
    <g clip-path="url(#insetclip)">
      <rect x="0" y="22" width={INSET.w} height={INSET.h - 22} fill={materialColor} opacity="0.55" />
      <!-- Passes précédentes (au-dessus de la bande engagée) et sillon derrière la fraise -->
      {#if ae < 1}
        <rect x="0" y="22" width={INSET.w} height={cy + ir - engagedWidth - 22} class="removed-inset" />
      {/if}
      <rect x="0" y={cy - ir} width={cx} height={2 * ir} class="removed-inset" />
      <circle cx={cx} cy={cy} r={ir} class="tooltop" />
      <rect x={cx} y={cy + ir - engagedWidth} width={ir} height={engagedWidth} class="engaged" clip-path="url(#toolclip)" />
      <line x1={cx - ir - 10} x2={cx - ir - 2} y1={cy} y2={cy} class="arrow" marker-end="url(#head-top)" />
    </g>
    <text x={INSET.w / 2} y={INSET.h + 16} class="lbl" text-anchor="middle">
      {t('scene.engagement', { v: Math.round(ae * 100) })}
    </text>
  </g>
</svg>
</div>

<style>
  .scene {
    display: flex;
    gap: var(--s4);
    width: 100%;
    height: 100%;
  }

  svg {
    display: block;
  }

  .side {
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .top {
    flex: none;
    width: clamp(110px, 14%, 170px);
    align-self: flex-start;
  }

  .removed {
    fill: var(--scene-removed);
  }

  .faint {
    fill: var(--text-muted);
    font-size: 10px;
  }

  .pass {
    stroke: var(--scene-pass);
    stroke-dasharray: 3 3;
  }

  .table {
    stroke: var(--scene-table);
    stroke-width: 2;
  }

  .dim line {
    stroke: var(--scene-dim);
    stroke-width: 1;
  }

  .lbl {
    fill: var(--text-secondary);
    font-size: 11px;
    font-family: var(--font);
  }

  .lbl.strong {
    fill: var(--text-primary);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .collet {
    fill: var(--scene-collet);
    stroke: var(--scene-collet-edge);
  }

  .shank {
    fill: var(--scene-shank);
  }

  .flute {
    fill: var(--scene-flute);
  }

  .knurl {
    fill: none;
    stroke: var(--scene-stripes);
    stroke-width: 0;
  }

  .stripes line {
    stroke: var(--scene-stripes);
    stroke-width: 2.2;
  }

  .stripes {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    from {
      transform: translateY(0);
    }
    to {
      transform: translateY(calc(var(--dir) * 9px));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .stripes {
      animation: none;
    }
  }

  .danger {
    fill: var(--scene-danger);
    stroke: var(--danger);
  }

  .danger-text {
    fill: var(--danger);
    font-weight: 600;
  }

  .arrow {
    fill: none;
    stroke: var(--scene-arrow);
    stroke-width: 1.4;
  }

  .inset {
    fill: var(--scene-inset);
    stroke: var(--border);
  }

  .removed-inset {
    fill: var(--scene-inset);
  }

  .tooltop {
    fill: var(--scene-flute);
    opacity: 0.9;
  }

  .engaged {
    fill: var(--accent);
    opacity: 0.75;
  }
</style>
