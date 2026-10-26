// Miroir TypeScript des types JSON de l'API (crates/fabspeed-core).
// Garder synchronisé avec docs/API.md.

export type Aggressiveness = 'prudent' | 'normal' | 'rapid';
export type Evidence = 'theoretical' | 'reference_data' | 'tested' | 'validated';
export type OperationKind = 'profile' | 'pocket' | 'full_slot' | 'finish' | 'surfacing' | 'engraving';
export type Workholding = 'screws_clamps' | 'vacuum' | 'tape' | 'unknown';
export type ToolFamily = 'flat_endmill' | 'ball_nose' | 'compression' | 'v_bit' | 'diamond_cut' | 'surfacing';
export type Helix = 'upcut' | 'downcut' | 'compression' | 'straight' | 'unknown';
export type MaterialCategory = 'wood' | 'panel' | 'plastic' | 'foam' | 'metal' | 'composite';
export type ValueStatus = 'verified' | 'estimate' | 'to_verify';

export interface SourceNote {
  status: ValueStatus;
  note: string;
}

export interface Machine {
  id: string;
  name: string;
  category: string;
  description: string;
  rpm_min: number;
  rpm_max: number;
  spindle_power_w: number;
  spindle_rated_rpm: number | null;
  max_feed_mm_min: number;
  max_plunge_mm_min: number | null;
  max_lateral_force_n: number;
  chipload_factor: number;
  controller: string | null;
  work_area_mm: [number, number, number] | null;
  sources: Record<string, SourceNote>;
  notes: string[];
}

export interface Material {
  id: string;
  name: string;
  category: MaterialCategory;
  aliases: string[];
  description: string;
  vc_m_min: number;
  k_fz: number;
  k_ap: number;
  kc_n_mm2: number;
  fz_min_mm: number;
  default_stepover: number;
  default_thickness_mm: number | null;
  max_profile_thickness_mm: number | null;
  supervision_required: boolean;
  tips: string[];
  source: string;
  evidence: Evidence;
}

export interface Tool {
  id: string;
  name: string;
  reference: string;
  supplier: string;
  url: string | null;
  family: ToolFamily;
  helix: Helix;
  tool_material: 'carbide' | 'hss';
  diameter_mm: number;
  flutes: number | null;
  flute_length_mm: number;
  shank_mm: number | null;
  overall_length_mm: number | null;
  tip_angle_deg: number | null;
  /** Fraise à compression : longueur de la partie upcut (mm), si connue. */
  upcut_length_mm?: number | null;
  max_rpm: number | null;
  supplier_rpm_min: number | null;
  supplier_rpm_recommended: number | null;
  recommended_materials: string[];
  excluded_materials: string[];
  notes: string[];
  fabcam_identity: { id: string; revision: string } | null;
}

export interface LabPreset {
  id: string;
  label: string;
  source: string;
  evidence: Evidence;
  machines: string[];
  material: string;
  tool: string;
  operations: OperationKind[];
  aggressiveness: Aggressiveness;
  rpm: number;
  feed_mm_min: number;
  doc_mm: number;
  plunge_mm_min: number | null;
  stepover_max: number | null;
  notes: string[];
  date?: string | null;
}

export interface Symptom {
  id: string;
  label: string;
  icon: string;
  cause: string;
  explanation: string;
  rpm_factor: number;
  feed_factor: number;
  doc_factor: number;
  advice: string[];
  categories: MaterialCategory[];
}

export interface Catalog {
  engine_version: string;
  meta: { revision: string; description: string; license: string };
  machines: Machine[];
  materials: Material[];
  tools: Tool[];
  presets: LabPreset[];
  symptoms: Symptom[];
  trials: Trial[];
  layers: { lab_machines: string[]; lab_tools: string[]; lab_materials: string[]; overridden: string[] };
  issues: { fatal: boolean; message: string }[];
}

export type TrialOutcome = 'good' | 'adjust' | 'failed';
export type ToolCondition = 'new' | 'used';

export interface TrialInput {
  operator: string;
  machine_id: string;
  material_id: string;
  tool_id: string;
  operation: OperationKind;
  aggressiveness: Aggressiveness;
  workholding: Workholding;
  thickness_mm: number | null;
  depth_mm: number | null;
  stepover_ratio: number;
  stickout_mm: number | null;
  /** État de la fraise au moment de l'essai (facultatif). */
  tool_condition?: ToolCondition | null;
  rpm: number;
  feed_mm_min: number;
  plunge_mm_min: number;
  doc_mm: number;
  outcome: TrialOutcome;
  symptoms: string[];
  notes: string;
}

export interface Trial extends TrialInput {
  id: string;
  recorded_at: string;
  tool_name: string;
  tool_diameter_mm: number;
  tool_flutes: number | null;
  engine_version: string;
  dataset_revision: string;
  /** Présente si l'administrateur a validé ce réglage. */
  validation?: TrialValidation | null;
}

export interface TrialValidation {
  /** Optional downstream binding; absent on historical validations. */
  binding?: {
    version: number;
    catalogue_id: string;
    trial_id: string;
    machine: { id: string; revision: string };
    tool: { id: string; revision: string };
    material: { id: string; revision: string };
    context_values_revision: string;
  };
  /** RFC 3339 (UTC). */
  validated_at: string;
  validated_by: string;
  note: string;
}

export interface Overrides {
  rpm?: number;
  feed_mm_min?: number;
  plunge_mm_min?: number;
  doc_mm?: number;
}

export interface RecommendationRequest {
  machine_id: string;
  material_id: string;
  tool_id?: string;
  tool?: Tool;
  operation: OperationKind;
  stepover_ratio?: number;
  depth_mm?: number;
  aggressiveness: Aggressiveness;
  workholding: Workholding;
  stickout_mm?: number;
  max_doc_mm?: number;
  cut_length_mm?: number;
  use_presets: boolean;
  overrides?: Overrides;
  /** Leviers d'accélération pour une matière sous supervision (honoré seulement en administrateur). */
  supervised_speedups?: boolean;
}

export interface RequestPatch {
  aggressiveness?: Aggressiveness;
  workholding?: Workholding;
  stickout_mm?: number;
  overrides?: Overrides;
  clear_overrides?: boolean;
}

export type Origin = 'lab_preset' | 'derived_from_preset' | 'calculated' | 'user_override';
export type ChipState = 'rubbing' | 'ok' | 'thick';
export type Severity = 'info' | 'warning' | 'blocking';
export type LimitingFactor =
  | 'spindle_max_rpm'
  | 'spindle_min_rpm'
  | 'tool_max_rpm'
  | 'machine_feed'
  | 'machine_plunge'
  | 'spindle_power'
  | 'machine_rigidity'
  | 'tool_strength'
  | 'workholding'
  | 'flute_length'
  | 'user_doc_cap'
  | 'slenderness';

export interface CuttingValues {
  rpm: number;
  feed_mm_min: number;
  plunge_mm_min: number;
  doc_mm: number;
  doc_max_mm: number;
  passes: number | null;
  depth_mm: number | null;
  stepover_mm: number;
  stepover_ratio: number;
  ramp_entry_advised: boolean;
}

export interface Metrics {
  chipload_mm: number;
  chip_thickness_mm: number;
  chip_zone: { min_mm: number; nominal_mm: number; max_mm: number; state: ChipState };
  rct_factor: number;
  cutting_speed_m_min: number;
  /** Diamètre qui coupe réellement (hémisphérique peu engagée, fraise en V). */
  cutting_diameter_mm: number;
  mrr_cm3_min: number;
  cutting_power_w: number;
  available_power_w: number;
  cutting_force_n: number;
  loads: { spindle_pct: number; machine_pct: number; tool_pct: number; holding_pct: number };
  stickout_mm: number;
  slenderness: number;
  estimated_time_s: number | null;
  /** Temps de coupe pour un mètre de trajectoire, toutes passes comprises. */
  time_per_metre_s: number;
}

export interface Warning {
  code: string;
  severity: Severity;
  message: string;
}

export interface Lever {
  id: string;
  title: string;
  detail: string;
  time_gain: number | null;
  caution: string | null;
  patch: RequestPatch;
}

export interface TraceStep {
  /** Code stable de l'étape (ne dépend pas de la langue). */
  key: string;
  label: string;
  formula: string;
  result: string;
}

export interface Recommendation {
  status: 'ok' | 'unsupported';
  context: {
    machine_id: string;
    machine_name: string;
    material_id: string;
    material_name: string;
    tool_id: string;
    tool_name: string;
    tool_diameter_mm: number;
    tool_flutes: number | null;
    operation: OperationKind;
    aggressiveness: Aggressiveness;
    workholding: Workholding;
  };
  values: CuttingValues | null;
  origins: { rpm: Origin; feed: Origin; plunge: Origin; doc: Origin } | null;
  metrics: Metrics | null;
  limits: LimitingFactor[];
  warnings: Warning[];
  levers: Lever[];
  trace: TraceStep[];
  provenance: {
    source: 'model' | 'lab_preset';
    preset_id: string | null;
    preset_label: string | null;
    preset_source: string | null;
    evidence: Evidence;
    engine_version: string;
    dataset_revision: string;
    notes: string[];
  };
  theoretical: { rpm: number; feed_mm_min: number; plunge_mm_min: number; doc_mm: number } | null;
  history: Trial[];
}

export interface Diagnosis {
  symptom: Symptom;
  before: CuttingValues;
  after: Recommendation;
  patch: RequestPatch;
}
