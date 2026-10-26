// Libellés de l'interface, rédigés pour des non-machinistes : mot courant
// d'abord, terme technique dans l'infobulle. Fonctions (et non constantes)
// pour suivre la langue choisie.

import { intlLocale, t, type Key } from './i18n.svelte';
import type {
  Aggressiveness,
  Evidence,
  Helix,
  LimitingFactor,
  MaterialCategory,
  OperationKind,
  Origin,
  ToolFamily,
  ToolCondition,
  TrialOutcome,
  ValueStatus,
  Workholding,
} from './types';

type Item<T extends string> = { id: T; label: string; hint: string };

const item = <T extends string>(prefix: string, id: T): Item<T> => ({
  id,
  label: t(`${prefix}.${id}.label` as Key),
  hint: t(`${prefix}.${id}.hint` as Key),
});

const OP_IDS: OperationKind[] = ['profile', 'pocket', 'full_slot', 'finish', 'surfacing', 'engraving'];
export const operations = () => OP_IDS.map((id) => item('op', id));
export const operationLabel = (op: OperationKind) => t(`op.${op}.label` as Key);

/**
 * Opérations qui ont un sens pour un type de fraise. Une fraise en V ne fait
 * que graver ; une hémisphérique creuse des rainures à fond rond, finit un
 * relief ou grave, mais ne détoure pas, ne vide pas une poche à fond plat et
 * ne surface pas.
 */
const OPS_BY_FAMILY: Partial<Record<ToolFamily, OperationKind[]>> = {
  v_bit: ['engraving'],
  ball_nose: ['full_slot', 'finish', 'engraving'],
};
export const operationAllowed = (op: OperationKind, family: ToolFamily | undefined) =>
  !family || !OPS_BY_FAMILY[family] || OPS_BY_FAMILY[family]!.includes(op);
export const firstAllowedOperation = (family: ToolFamily) => OP_IDS.find((op) => operationAllowed(op, family))!;

const MODE_IDS: Aggressiveness[] = ['prudent', 'normal', 'rapid'];
export const modes = () => MODE_IDS.map((id) => item('mode', id));
export const modeLabel = (a: Aggressiveness) => t(`mode.${a}.label` as Key);

const WH_IDS: Workholding[] = ['screws_clamps', 'vacuum', 'tape', 'unknown'];
export const workholdings = () => WH_IDS.map((id) => item('workholding', id));
export const workholdingLabel = (w: Workholding) => t(`workholding.${w}.label` as Key);

export const categoryLabel = (c: MaterialCategory) => t(`category.${c}` as Key);
export const CATEGORY_ORDER: MaterialCategory[] = ['panel', 'wood', 'plastic', 'foam', 'composite', 'metal'];

export const FAMILY_IDS: ToolFamily[] = ['flat_endmill', 'ball_nose', 'compression', 'v_bit', 'diamond_cut', 'surfacing'];
export const familyLabel = (f: ToolFamily) => t(`family.${f}` as Key);

export const HELIX_IDS: Helix[] = ['upcut', 'downcut', 'compression', 'straight', 'unknown'];
export const helixLabel = (h: Helix) => t(`helix.${h}` as Key);

export const origin = (o: Origin) => ({ label: t(`origin.${o}.label` as Key), hint: t(`origin.${o}.hint` as Key) });
export const evidence = (e: Evidence) => ({ label: t(`evidence.${e}.label` as Key), hint: t(`evidence.${e}.hint` as Key) });
export const limitLabel = (l: LimitingFactor) => t(`limit.${l}` as Key);
export const valueStatusLabel = (s: ValueStatus) => t(`valueStatus.${s}` as Key);
export const VALUE_STATUS_IDS: ValueStatus[] = ['verified', 'estimate', 'to_verify'];

export const MACHINE_FIELD_IDS = [
  'rpm_min',
  'rpm_max',
  'spindle_power_w',
  'spindle_rated_rpm',
  'max_feed_mm_min',
  'max_lateral_force_n',
  'work_area_mm',
] as const;
export const machineFieldLabel = (f: (typeof MACHINE_FIELD_IDS)[number]) => t(`machineField.${f}` as Key);

const OUTCOME_IDS: TrialOutcome[] = ['good', 'adjust', 'failed'];
export const outcomes = () => OUTCOME_IDS.map((id) => item('outcome', id));
export const outcomeLabel = (o: TrialOutcome) => t(`outcome.${o}.label` as Key);

const CONDITION_IDS: (ToolCondition | 'unknown')[] = ['unknown', 'new', 'used'];
export const toolConditions = () => CONDITION_IDS.map((id) => item('toolCondition', id));
export const toolConditionLabel = (c: ToolCondition) => t(`toolCondition.${c}.label` as Key);

export type LayerId = 'base' | 'lab' | 'modified';
export const layer = (l: LayerId) => ({ label: t(`layer.${l}.label` as Key), hint: t(`layer.${l}.hint` as Key) });

/** « Ø6 · 2 dents · upcut · LU 25 » / « Ø6 · 2 flutes · upcut · FL 25 ». */
export function toolSummary(tool: ToolSpecLike, fmt: (v: number) => string): string {
  return `${toolHeadline(tool, fmt)} · ${t('tool.fluteLength', { v: fmt(tool.flute_length_mm) })}`;
}

/** Ce qui distingue une fraise d'un coup d'œil : « Ø6 · 2 dents · upcut ». */
export function toolHeadline(tool: ToolSpecLike, fmt: (v: number) => string): string {
  const parts = [`Ø${fmt(tool.diameter_mm)}`];
  // Taille diamant : des centaines de micro-dents, pas de goujures à compter.
  if (tool.family === 'diamond_cut' && !tool.flutes) parts.push(t('tool.diamondTeeth'));
  else {
    parts.push(tool.flutes ? t(tool.flutes > 1 ? 'tool.flutes' : 'tool.flute', { n: tool.flutes }) : t('tool.flutesUnknown'));
    if (tool.family !== 'flat_endmill') parts.push(familyLabel(tool.family).toLowerCase());
    else if (tool.helix !== 'unknown') parts.push(helixLabel(tool.helix).toLowerCase());
  }
  return parts.join(' · ');
}

type ToolSpecLike = { diameter_mm: number; flutes: number | null; family: ToolFamily; helix: Helix; flute_length_mm: number };

/** Unité de rotation : « tr/min » ou « rpm ». */
export const rpmUnit = () => t('unit.rpm');

/** Date et heure locales d'un horodatage RFC 3339. */
export function dateTime(rfc3339: string): string {
  const d = new Date(rfc3339);
  if (Number.isNaN(d.getTime())) return rfc3339;
  return d.toLocaleString(intlLocale(), { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}
