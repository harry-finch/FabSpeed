<script lang="ts">
  // Schéma de la coupe, à l'échelle : vue en coupe (matière, passes, fraise,
  // sortie de pince) et vue de dessus (engagement radial). Dessiné « au
  // trait » comme les figures Hairline : un trait de 1 px, quatre gris
  // (--hairline-*), des formes au contour remplies de la couleur du fond.
  // Trois couleurs seulement : la matière (teinte légère du brut), la coupe en
  // cours (tranche attaquée et secteur d'engagement, selon la zone de copeau)
  // et le danger. Les goujures défilent dans le sens d'évacuation des copeaux
  // (upcut / downcut) ; le défilement s'arrête sous prefers-reduced-motion.
  import type { ChipState, OperationKind, Tool } from '../lib/types';
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
    chipState,
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
    /** Zone de copeau (jauge) : colore la tranche attaquée et l'engagement. */
    chipState: ChipState;
    feedLabel: string;
    rpmLabel: string;
    units: UnitSystem;
    /** Diamètre qui coupe (largeur gravée pour une fraise en V). */
    cuttingDiameterMm?: number;
  } = $props();

  // Vue en coupe : cadrée au plus près pour occuper la place ; la vue de dessus
  // est un second dessin, calé à droite de la zone. Le dessin prend les
  // proportions de sa zone (mesurée) : la plaque s'allonge quand elle est large,
  // au lieu de laisser des marges vides de part et d'autre.
  const H = 300;
  const TOP = 36; // face de la pince (place pour la pince, l'étiquette de rotation est à côté)
  const BOTTOM = 276;
  const SLAB_X0 = 28;
  let boxW = $state(0);
  let boxH = $state(0);
  const W = $derived(Math.round(Math.min(Math.max(boxH > 0 ? (H * boxW) / boxH : 490, 490), 900)));
  const SLAB_X1 = $derived(W - 20);
  const TOOL_X = $derived(Math.round(SLAB_X0 + (SLAB_X1 - SLAB_X0) * 0.68));

  // Échelle commune (px/mm) : la fraise sortie + l'épaisseur occupent toute la
  // hauteur, tant que la fraise ne dépasse pas trois dixièmes de la largeur.
  const reach = $derived(Math.max(stickoutMm - depthMm, 0) + thicknessMm);
  const scale = $derived(
    Math.min(
      (BOTTOM - TOP) / Math.max(reach, 1),
      (0.3 * W) / Math.max(tool.diameter_mm, tool.shank_mm ?? 0, 1),
      20,
    ),
  );
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

  // Contour de la partie coupante (aussi le masque des goujures).
  const bodyD = $derived(
    ball
      ? `M${TOOL_X - r} ${fluteTop} H${TOOL_X + r} V${tipY - r} A${r} ${r} 0 0 1 ${TOOL_X - r} ${tipY - r} Z`
      : vbit
        ? `M${TOOL_X - r} ${fluteTop} H${TOOL_X + r} V${tipY - coneH} L${TOOL_X} ${tipY} L${TOOL_X - r} ${tipY - coneH} Z`
        : `M${TOOL_X - r} ${fluteTop} H${TOOL_X + r} V${tipY} H${TOOL_X - r} Z`,
  );

  type Pt = [number, number];
  // Polygone aux coins arrondis (rayon en px), pour le brut et la pince.
  function rounded(raw: Pt[], rad = 2): string {
    const p = raw.filter((q, i) => {
      const n = raw[(i + 1) % raw.length];
      return q[0] !== n[0] || q[1] !== n[1];
    });
    if (p.length < 3) return '';
    return (
      p
        .map((q, i) => {
          const a = p[(i - 1 + p.length) % p.length];
          const b = p[(i + 1) % p.length];
          const la = Math.hypot(q[0] - a[0], q[1] - a[1]);
          const lb = Math.hypot(b[0] - q[0], b[1] - q[1]);
          const k = Math.min(rad, la / 2, lb / 2);
          const s = [q[0] + ((a[0] - q[0]) * k) / la, q[1] + ((a[1] - q[1]) * k) / la];
          const e = [q[0] + ((b[0] - q[0]) * k) / lb, q[1] + ((b[1] - q[1]) * k) / lb];
          return `${i === 0 ? 'M' : 'L'}${s[0]} ${s[1]}Q${q[0]} ${q[1]} ${e[0]} ${e[1]}`;
        })
        .join('') + 'Z'
    );
  }

  // Brut non usiné : la matière, moins le sillon déjà creusé derrière la fraise.
  const grooved = $derived(cutDepthY - slabTop > 0.5);
  const through = $derived(cutDepthY >= BOTTOM - 0.5);
  const slabD = $derived(
    rounded(
      !grooved
        ? [[SLAB_X0, slabTop], [SLAB_X1, slabTop], [SLAB_X1, BOTTOM], [SLAB_X0, BOTTOM]]
        : through
          ? [[TOOL_X, slabTop], [SLAB_X1, slabTop], [SLAB_X1, BOTTOM], [TOOL_X, BOTTOM]]
          : [[TOOL_X, slabTop], [SLAB_X1, slabTop], [SLAB_X1, BOTTOM], [SLAB_X0, BOTTOM], [SLAB_X0, cutDepthY], [TOOL_X, cutDepthY]],
    ),
  );
  // Tranche attaquée : devant la fraise, sur la hauteur de la passe en cours.
  const bandH = $derived(grooved ? Math.min(docMm * scale, cutDepthY - slabTop) : 0);
  const colletD = $derived(
    rounded([
      [TOOL_X - shankR - 14, colletY - 30],
      [TOOL_X + shankR + 14, colletY - 30],
      [TOOL_X + shankR + 6, colletY],
      [TOOL_X - shankR - 6, colletY],
    ]),
  );

  // Goujures : arêtes (trait courant) et talons juste derrière (trait en retrait).
  // Hélice : courbes en S obliques (« / » upcut, « \ » downcut) qui défilent
  // en hauteur ; goujures droites (fraise en V, à surfacer) : traits parallèles
  // à l'axe qui défilent en largeur. Une période = l'écart entre deux arêtes.
  const straight = $derived(vbit || tool.family === 'surfacing' || tool.helix === 'straight');
  const teeth = $derived(Math.min(Math.max(tool.flutes ?? 2, 1), 4));
  const period = $derived(straight ? Math.max((2 * r) / Math.max(teeth, 2), 3) : 18 / teeth);
  const flutes = $derived.by(() => {
    const xl = TOOL_X - r;
    const xr = TOOL_X + r;
    const w = 2 * r;
    const rise = r * 0.9;
    let edges = '';
    let heels = '';
    if (straight) {
      const n = Math.ceil(w / period) + 3;
      for (let i = -1; i < n; i++) {
        const x = xl + period / 2 + i * period;
        edges += `M${x} ${fluteTop}V${tipY}`;
        heels += `M${x - 3} ${fluteTop}V${tipY}`;
      }
    } else {
      const n = Math.ceil((tipY - fluteTop) / period) + 4;
      for (let i = 0; i < n; i++) {
        const y = fluteTop + (i - 2) * period;
        const yl = y + (helixDir < 0 ? rise : 0);
        const yr = y + (helixDir < 0 ? 0 : rise);
        const o = -helixDir * 3;
        edges += `M${xl} ${yl}C${xl + w * 0.4} ${yl} ${xr - w * 0.4} ${yr} ${xr} ${yr}`;
        heels += `M${xl} ${yl + o}C${xl + w * 0.4} ${yl + o} ${xr - w * 0.4} ${yr + o} ${xr} ${yr + o}`;
      }
    }
    return { edges, heels };
  });

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
  // Passes précédentes (au-dessus de la bande engagée) : bord du brut restant.
  const y0 = $derived(cy + ir - engagedWidth);
  const topEdge = $derived(
    ae < 1
      ? `M0 ${cy + ir}H${cx}V${y0}H${INSET.w}`
      : `M0 ${cy - ir}H${cx}V${cy + ir}H0`,
  );
  // Secteur engagé : moitié avant de la fraise, sur la largeur de coupe.
  const yc = $derived(Math.min(Math.max(y0, cy - ir), cy + ir));
  const xc = $derived(cx + Math.sqrt(Math.max(ir * ir - (yc - cy) ** 2, 0)));
  const sectorArc = $derived(`M${xc} ${yc}A${ir} ${ir} 0 0 1 ${cx} ${cy + ir}`);
  const sectorD = $derived(`M${cx} ${yc}H${xc}A${ir} ${ir} 0 0 1 ${cx} ${cy + ir}Z`);
</script>

<div class="scene linework" class:ok={chipState === 'ok'} class:warn={chipState !== 'ok'} style:--mat={materialColor}>
<div class="side" bind:clientWidth={boxW} bind:clientHeight={boxH}>
<svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={t('scene.label')}>
  <defs>
    <clipPath id="flutes"><path d={bodyD} /></clipPath>
    <marker id="head" viewBox="0 0 8 8" refX="6" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L8 4L0 8z" class="head" />
    </marker>
  </defs>

  <!-- Matière (coupe) : brut non usiné, teinte légère sous un contour neutre ; le sillon reste vide -->
  <path d={slabD} class="plate" />
  <path d={slabD} class="tint" />
  <path d={slabD} class="ln edge" />
  <!-- Tranche que la fraise attaque : couleur de la zone de copeau -->
  {#if bandH > 0}
    <rect x={TOOL_X} y={cutDepthY - bandH} width={r + 9} height={bandH} class="cutband" />
  {/if}
  {#if roomy}<text x={SLAB_X0 + 8} y={cutDepthY - 8} class="lbl faint">{t('scene.groove')}</text>{/if}
  {#each passLines as y, i (i)}
    <line x1={SLAB_X0} x2={TOOL_X} y1={y} y2={y} class="ln pass" />
  {/each}
  <line x1={SLAB_X0} x2={SLAB_X1} y1={BOTTOM} y2={BOTTOM} class="ln edge" />
  <text x={SLAB_X0} y={BOTTOM + 16} class="lbl">{t('scene.spoilboard')}</text>

  <!-- Repère des passes -->
  {#if passes > 0}
    <g class="ln edge">
      <line x1={SLAB_X0 - 10} x2={SLAB_X0 - 10} y1={slabTop} y2={cutDepthY} />
      <line x1={SLAB_X0 - 14} x2={SLAB_X0 - 6} y1={slabTop} y2={slabTop} />
      <line x1={SLAB_X0 - 14} x2={SLAB_X0 - 6} y1={cutDepthY} y2={cutDepthY} />
    </g>
    <text x={SLAB_X0 + 8} y={roomy ? slabTop + Math.min(docMm * scale, 40) / 2 + 4 : slabTop - 8} class="lbl strong">
      {t(passes > 1 ? 'readout.passesN' : 'readout.passN', { n: passes })} × {fmtLength(docMm, units)} {lu}
    </text>
  {/if}

  <!-- Fraise : pince, queue, partie coupante (seul trait vif du schéma) -->
  <path d={colletD} class="plate ln edge" />
  <rect x={TOOL_X - shankR} y={colletY} width={2 * shankR} height={Math.max(fluteTop - colletY, 0)} class="plate ln edge" />
  <path d={bodyD} class="plate" />
  {#if tool.family !== 'diamond_cut'}
    <g clip-path="url(#flutes)">
      <g class="flutes" class:across={straight} style:--dir={helixDir} style:--period={`${period}px`}>
        <path d={flutes.heels} class="ln low" />
        <path d={flutes.edges} class="ln mid" />
      </g>
    </g>
  {/if}
  <path d={bodyD} class="ln hi" />
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
  <g class="ln edge">
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
    class="ln edge"
    marker-end="url(#head)"
  />
  <text x={TOOL_X + shankR + 22} y={colletY - 11} class="lbl strong">{rpmLabel}</text>
  <line x1={TOOL_X + r + 14} x2={TOOL_X + r + 70} y1={slabTop - 12} y2={slabTop - 12} class="ln edge" marker-end="url(#head)" />
  <text x={TOOL_X + r + 14} y={slabTop - 20} class="lbl strong">{feedLabel}</text>
</svg>
</div>

<!-- Vue de dessus : engagement radial -->
<svg class="top" viewBox="0 0 126 146" role="img" aria-label={t('scene.topView')}>
  <defs>
    <marker id="head-top" viewBox="0 0 8 8" refX="6" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L8 4L0 8z" class="head" />
    </marker>
    <clipPath id="insetclip"><rect width={INSET.w} height={INSET.h} rx="2" /></clipPath>
  </defs>
  <g transform={`translate(${INSET.x} ${INSET.y})`}>
    <rect width={INSET.w} height={INSET.h} rx="2" class="plate" />
    <text x="8" y="15" class="lbl">{t('scene.topView')}</text>
    <g clip-path="url(#insetclip)">
      <!-- Brut restant : teinte légère ; passes précédentes et sillon laissés vides -->
      {#if ae < 1}
        <rect x="0" y={y0} width={INSET.w} height={INSET.h - y0} class="tint" />
        <rect x="0" y={cy + ir} width={cx} height={INSET.h - cy - ir} class="tint" />
        <rect x="0" y={y0} width={cx} height={cy + ir - y0} class="plate" />
      {:else}
        <rect x="0" y="22" width={INSET.w} height={INSET.h - 22} class="tint" />
        <rect x="0" y={cy - ir} width={cx} height={2 * ir} class="plate" />
        <line x1="0" x2={INSET.w} y1="22" y2="22" class="ln edge" />
      {/if}
      <path d={topEdge} class="ln edge" />
      <circle cx={cx} cy={cy} r={ir} class="plate ln hi" />
      <path d={sectorD} class="cutband sector" />
      <path d={sectorArc} class="ln cutline" />
      <line x1={cx - ir - 10} x2={cx - ir - 2} y1={cy} y2={cy} class="ln edge" marker-end="url(#head-top)" />
    </g>
    <rect width={INSET.w} height={INSET.h} rx="2" class="ln edge" />
    <text x={INSET.w / 2} y={INSET.h + 16} class="lbl" text-anchor="middle">
      {t('scene.engagement', { v: Math.round(ae * 100) })}
    </text>
  </g>
</svg>
</div>

<style>
  .scene {
    /* Les quatre poids du trait ; repli sans color-mix sur les bords de l'application. */
    --le: var(--hairline-edge, var(--border-strong));
    --lm: var(--hairline-mid, var(--border-strong));
    --ll: var(--hairline-lo, var(--border));
    display: flex;
    gap: var(--s4);
    width: 100%;
    height: 100%;
  }

  /* Coupe en cours : sain en vert, frotte ou trop épais en orange. */
  .scene.ok {
    --cut: var(--success);
    --cut-tint: var(--success-tint);
  }

  .scene.warn {
    --cut: var(--warning);
    --cut-tint: var(--warning-tint);
  }

  svg {
    display: block;
  }

  .side {
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .side svg {
    width: 100%;
    height: 100%;
  }

  .top {
    flex: none;
    width: clamp(110px, 14%, 170px);
    align-self: flex-start;
  }

  /* Un seul trait : 1 px, constant à toute échelle. */
  .ln {
    fill: none;
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .ln :global(line) {
    vector-effect: non-scaling-stroke;
    stroke-linecap: round;
  }

  .ln.edge {
    stroke: var(--le);
  }

  .ln.mid {
    stroke: var(--lm);
  }

  .ln.low {
    stroke: var(--ll);
  }

  .ln.hi {
    stroke: var(--hairline-hi);
  }

  .ln.pass {
    stroke: var(--ll);
    stroke-dasharray: 2 3;
  }

  .ln.cutline {
    stroke: var(--cut);
  }

  /* Forme au contour : remplie du fond pour cacher ce qui est derrière. */
  .plate {
    fill: var(--hairline-plate);
  }

  .plate.ln {
    fill: var(--hairline-plate);
  }

  /* Matière : le brut, en teinte légère. */
  .tint {
    fill: var(--mat);
    fill-opacity: var(--scene-material-tint);
  }

  .cutband {
    fill: var(--cut-tint);
    stroke: var(--cut);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    stroke-linejoin: round;
  }

  .cutband.sector {
    stroke: none;
  }

  .head {
    fill: var(--le);
  }

  .faint {
    fill: var(--text-muted);
    font-size: 10px;
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

  .flutes {
    animation: spin 0.9s linear infinite;
  }

  .flutes.across {
    animation-name: spin-across;
  }

  /* Hélice : les arêtes montent (upcut) ou descendent (downcut) d'une période. */
  @keyframes spin {
    from {
      transform: translateY(0);
    }
    to {
      transform: translateY(calc(var(--dir) * var(--period)));
    }
  }

  /* Goujures droites : les arêtes passent d'un bord à l'autre. */
  @keyframes spin-across {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(var(--period));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .flutes {
      animation: none;
    }
  }

  .danger {
    fill: var(--scene-danger);
    stroke: var(--danger);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .danger-text {
    fill: var(--danger);
    font-weight: 600;
  }
</style>
