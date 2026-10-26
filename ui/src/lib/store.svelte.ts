// État de l'application (runes Svelte 5) : catalogue, configuration de la
// coupe, recommandation courante, bibliothèque. La configuration de coupe est
// mémorisée dans le navigateur ; machines, fraises et essais vivent sur le
// serveur (data/lab/). Toute écriture passe par `#write`, qui demande d'abord
// le mot de passe administrateur si l'on est en mode invité.

import { api, ApiError, session, type AuthStatus, type RestoreCheck, type LibraryKind, type ImportReport } from './api';
import { i18n, t, type Locale } from './i18n.svelte';
import { familyLabel, firstAllowedOperation, operationAllowed, operationLabel } from './labels';
import type { UnitSystem } from './format';
import type {
  Aggressiveness,
  Catalog,
  Diagnosis,
  Machine,
  Material,
  OperationKind,
  Overrides,
  Recommendation,
  RecommendationRequest,
  RequestPatch,
  Tool,
  Trial,
  TrialInput,
  Workholding,
} from './types';

const SETUP_KEY = 'fabspeed.setup.v1';
const UNITS_KEY = 'fabspeed.units.v1';
const OPERATOR_KEY = 'fabspeed.operator.v1';
/** Premier lancement guidé déjà fait (ou passé) sur ce poste. */
const ONBOARDED_KEY = 'fabspeed.onboarded.v1';

export type AuthDialog = 'login' | 'setup' | 'change';

export interface Setup {
  machineId: string;
  materialId: string;
  toolId: string;
  operation: OperationKind;
  thicknessMm: number;
  /** Profondeur à usiner ; null = automatique selon l'opération. */
  depthMm: number | null;
  /** Pas latéral (fraction de D) ; null = valeur par défaut. */
  stepoverRatio: number | null;
  aggressiveness: Aggressiveness;
  workholding: Workholding;
  stickoutMm: number | null;
  maxDocMm: number | null;
  cutLengthMm: number | null;
  usePresets: boolean;
  overrides: Overrides;
}

const DEFAULT_SETUP: Setup = {
  machineId: 'shopbot-prs-standard-96x48-2016',
  materialId: 'material/wood.plywood',
  toolId: 'generic/flat-6-1f-up',
  operation: 'profile',
  thicknessMm: 18,
  depthMm: null,
  stepoverRatio: null,
  aggressiveness: 'normal',
  workholding: 'screws_clamps',
  stickoutMm: null,
  maxDocMm: null,
  cutLengthMm: null,
  usePresets: true,
  overrides: {},
};

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* stockage indisponible : l'application reste utilisable */
  }
}

export type Module = 'calc' | 'library';
export type AssistantTab = 'faster' | 'problem' | 'trials' | 'explain';
export type LibrarySection = 'machines' | 'materials' | 'tools' | 'trials' | 'data';

/** Réglages touchés par les leviers et le diagnostic, gardés pour « Réinitialiser ». */
type Baseline = Pick<Setup, 'aggressiveness' | 'workholding' | 'stickoutMm' | 'overrides'>;

/** Aperçu d'un levier survolé : la recommandation qu'il donnerait. */
export interface LeverPreview {
  id: string;
  reco: Recommendation;
}

export interface Toast {
  id: number;
  text: string;
}

/** Profondeur automatique selon l'opération. */
export function autoDepth(op: OperationKind, thickness: number): number {
  switch (op) {
    // Rainure et poche ne traversent pas : la moitié de l'épaisseur par défaut.
    case 'pocket':
    case 'full_slot':
      return Math.max(0.5, Math.round(thickness / 2));
    case 'surfacing':
    case 'engraving':
      return 1;
    default:
      return thickness;
  }
}

const message = (e: unknown) => (e instanceof ApiError ? e.message : String(e));

class Store {
  catalog = $state<Catalog | null>(null);
  setup = $state<Setup>(readJson(SETUP_KEY, DEFAULT_SETUP));
  units = $state<UnitSystem>(readJson<{ u: UnitSystem }>(UNITS_KEY, { u: 'mm' }).u);
  operator = $state<string>(readJson<{ name: string }>(OPERATOR_KEY, { name: '' }).name);
  reco = $state<Recommendation | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  online = $state(true);
  module = $state<Module>('calc');
  assistantTab = $state<AssistantTab>('faster');
  diagnosis = $state<Diagnosis | null>(null);
  trialDialog = $state(false);
  /** Premier lancement guidé affiché. */
  onboarding = $state(false);
  /** Essai en cours de validation (fenêtre ouverte), ou null. */
  validateDialog = $state<string | null>(null);
  auth = $state<AuthStatus>({ configured: false, admin: false });
  /** Fenêtre de mot de passe ouverte (null = fermée). */
  authDialog = $state<AuthDialog | null>(null);
  toasts = $state<Toast[]>([]);
  /** Réglage d'avant le premier levier ou diagnostic appliqué (null = rien appliqué). */
  baseline = $state<Baseline | null>(null);
  /** Aperçu du levier survolé dans l'assistant. */
  preview = $state<LeverPreview | null>(null);
  library = $state<{ section: LibrarySection; selected: string | null; creating: boolean }>({
    section: 'tools',
    selected: null,
    creating: false,
  });

  #timer: ReturnType<typeof setTimeout> | undefined;
  #abort: AbortController | undefined;
  #lastKey = '';
  #toastId = 0;
  #previewTimer: ReturnType<typeof setTimeout> | undefined;
  #previewAbort: AbortController | undefined;
  /** Action en attente de connexion : résolue à la fermeture de la fenêtre. */
  #authWaiter: ((ok: boolean) => void) | null = null;

  get tools(): Tool[] {
    return this.catalog?.tools ?? [];
  }

  get machine(): Machine | undefined {
    return this.catalog?.machines.find((m) => m.id === this.setup.machineId);
  }

  get material(): Material | undefined {
    return this.catalog?.materials.find((m) => m.id === this.setup.materialId);
  }

  get tool(): Tool | undefined {
    return this.tools.find((t) => t.id === this.setup.toolId);
  }

  get effectiveDepth(): number {
    return this.setup.depthMm ?? autoDepth(this.setup.operation, this.setup.thicknessMm);
  }

  get request(): RecommendationRequest | null {
    const s = this.setup;
    if (!this.catalog || !this.tool || !this.machine || !this.material) return null;
    const usesStepover = s.operation === 'pocket' || s.operation === 'surfacing';
    return {
      machine_id: s.machineId,
      material_id: s.materialId,
      tool_id: s.toolId,
      operation: s.operation,
      stepover_ratio: usesStepover && s.stepoverRatio != null ? s.stepoverRatio : undefined,
      depth_mm: this.effectiveDepth,
      aggressiveness: s.aggressiveness,
      workholding: s.workholding,
      stickout_mm: s.stickoutMm ?? undefined,
      max_doc_mm: s.maxDocMm ?? undefined,
      cut_length_mm: s.cutLengthMm ?? undefined,
      use_presets: s.usePresets,
      overrides: Object.keys(s.overrides).length ? { ...s.overrides } : undefined,
      // Métaux : l'administrateur (fabmanager) voit aussi les accélérations.
      supervised_speedups: this.material.supervision_required && this.auth.admin ? true : undefined,
    };
  }

  /** Un essai réussi existe pour cette fraise et cette matière sur la machine active. */
  isTested(toolId: string, materialId: string): boolean {
    const c = this.catalog;
    if (!c) return false;
    const m = this.setup.machineId;
    return (
      c.trials.some((t) => t.outcome === 'good' && t.tool_id === toolId && t.material_id === materialId && t.machine_id === m) ||
      c.presets.some((p) => p.tool === toolId && p.material === materialId && p.machines.includes(m))
    );
  }

  /** Origine d'une fiche : base (data/), lab (créée dans l'app) ou base modifiée. */
  layerOf(kind: 'tool' | 'machine' | 'material', id: string): 'base' | 'lab' | 'modified' {
    const l = this.catalog?.layers;
    if (!l) return 'base';
    if (l.overridden.includes(id)) return 'modified';
    const lab = { tool: l.lab_tools, machine: l.lab_machines, material: l.lab_materials ?? [] }[kind];
    return lab.includes(id) ? 'lab' : 'base';
  }

  async init() {
    try {
      this.catalog = await api.catalog();
      this.online = true;
      await this.refreshAuth();
      this.#sanitize();
      // Poste neuf : aucun mot de passe administrateur et présentation jamais vue.
      if (!this.auth.configured && !this.#onboarded()) this.onboarding = true;
    } catch (e) {
      this.online = false;
      this.error = message(e);
    }
  }

  #onboarded(): boolean {
    try {
      return localStorage.getItem(ONBOARDED_KEY) === '1';
    } catch {
      return true;
    }
  }

  /** Fin (ou abandon) du premier lancement guidé : ne plus le proposer d'office. */
  finishOnboarding() {
    this.onboarding = false;
    try {
      localStorage.setItem(ONBOARDED_KEY, '1');
    } catch {
      /* ignoré */
    }
  }

  /** Relit le catalogue après une écriture, puis force un recalcul. */
  async refreshCatalog() {
    this.catalog = await api.catalog();
    this.#sanitize();
    this.#lastKey = '';
    this.schedule();
  }

  /** Repli sur des valeurs valides si le catalogue a changé. */
  #sanitize() {
    const c = this.catalog!;
    if (!c.machines.some((m) => m.id === this.setup.machineId)) this.setup.machineId = c.machines[0]?.id ?? '';
    if (!c.materials.some((m) => m.id === this.setup.materialId)) this.setup.materialId = c.materials[0]?.id ?? '';
    if (!this.tools.some((t) => t.id === this.setup.toolId)) this.setup.toolId = this.tools[0]?.id ?? '';
    this.#fitOperation();
  }

  /** Opération impossible avec ce type de fraise : on passe à une opération possible, et on le dit. */
  #fitOperation() {
    const family = this.tool?.family;
    if (!family || operationAllowed(this.setup.operation, family)) return;
    const from = this.setup.operation;
    this.setup.operation = firstAllowedOperation(family);
    this.setup.depthMm = null;
    this.setup.stepoverRatio = null;
    this.toast(t('toast.operationChanged', { from: operationLabel(from), to: operationLabel(this.setup.operation), family: familyLabel(family) }));
  }

  persist() {
    write(SETUP_KEY, $state.snapshot(this.setup));
    write(UNITS_KEY, { u: this.units });
    write(OPERATOR_KEY, { name: this.operator });
  }

  /** Recalcul différé : regroupe les saisies rapides (curseurs, frappe). */
  schedule() {
    const req = this.request;
    if (!req) return;
    const key = i18n.locale + JSON.stringify(req);
    if (key === this.#lastKey) return;
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.#run(req, key), 90);
  }

  async #run(req: RecommendationRequest, key: string) {
    this.#abort?.abort();
    const ctrl = new AbortController();
    this.#abort = ctrl;
    this.loading = true;
    try {
      const reco = await api.recommend(req, ctrl.signal);
      if (ctrl.signal.aborted) return;
      this.reco = reco;
      this.preview = null;
      this.#lastKey = key;
      this.error = null;
      this.online = true;
    } catch (e) {
      if (ctrl.signal.aborted) return;
      if (e instanceof ApiError) {
        this.error = e.message;
        this.online = e.code !== 'offline';
      } else if ((e as Error).name !== 'AbortError') {
        this.error = String(e);
      }
    } finally {
      if (this.#abort === ctrl) this.loading = false;
    }
  }

  toast(text: string) {
    const id = ++this.#toastId;
    this.toasts.push({ id, text });
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), 4200);
  }

  /** Changement de contexte : les valeurs saisies à la main ne s'appliquent plus. */
  #contextChanged() {
    this.diagnosis = null;
    this.baseline = null;
    if (Object.keys(this.setup.overrides).length) {
      this.setup.overrides = {};
      this.toast(t('toast.overridesCleared'));
    }
  }

  /** Change la langue : textes des données et messages du serveur sont rechargés. */
  async setLocale(l: Locale) {
    if (l === i18n.locale) return;
    i18n.set(l);
    this.diagnosis = null;
    try {
      await this.refreshCatalog();
    } catch (e) {
      this.toast(message(e));
    }
  }

  setMachine(id: string) {
    if (id === this.setup.machineId) return;
    this.setup.machineId = id;
    this.#contextChanged();
  }

  setMaterial(id: string) {
    if (id === this.setup.materialId) return;
    const m = this.catalog?.materials.find((x) => x.id === id);
    this.setup.materialId = id;
    if (m?.default_thickness_mm) this.setup.thicknessMm = m.default_thickness_mm;
    this.setup.depthMm = null;
    this.setup.stepoverRatio = null;
    this.#contextChanged();
  }

  setTool(id: string) {
    if (id === this.setup.toolId) return;
    this.setup.toolId = id;
    this.setup.stickoutMm = null;
    this.#fitOperation();
    this.#contextChanged();
  }

  setOperation(op: OperationKind) {
    if (op === this.setup.operation) return;
    this.setup.operation = op;
    this.setup.depthMm = null;
    this.setup.stepoverRatio = null;
    this.#contextChanged();
  }

  setOverride(field: keyof Overrides, value: number | undefined) {
    const next = { ...this.setup.overrides };
    if (value == null || !Number.isFinite(value) || value <= 0) delete next[field];
    else next[field] = value;
    this.setup.overrides = next;
  }

  /** Quelque chose a été modifié depuis le calcul de départ (valeur saisie, levier, correction). */
  get modified(): boolean {
    return this.baseline != null || Object.keys(this.setup.overrides).length > 0;
  }

  /** Revient au calcul de départ : annule leviers, corrections et valeurs saisies. */
  resetAll() {
    const b = this.baseline;
    if (b) {
      this.setup.aggressiveness = b.aggressiveness;
      this.setup.workholding = b.workholding;
      this.setup.stickoutMm = b.stickoutMm;
    }
    this.setup.overrides = {};
    this.baseline = null;
    this.diagnosis = null;
  }

  applyPatch(patch: RequestPatch, text?: string) {
    this.cancelPreview();
    if (!this.baseline) {
      const s = $state.snapshot(this.setup);
      this.baseline = { aggressiveness: s.aggressiveness, workholding: s.workholding, stickoutMm: s.stickoutMm, overrides: s.overrides };
    }
    if (patch.clear_overrides) this.setup.overrides = {};
    if (patch.aggressiveness) this.setup.aggressiveness = patch.aggressiveness;
    if (patch.workholding) this.setup.workholding = patch.workholding;
    if (patch.stickout_mm != null) this.setup.stickoutMm = patch.stickout_mm;
    if (patch.overrides) this.setup.overrides = { ...this.setup.overrides, ...patch.overrides };
    if (text) this.toast(text);
  }

  /** Calcule, après un court délai, ce que donnerait un levier (survol, focus). */
  previewLever(id: string, patch: RequestPatch) {
    const req = this.request;
    if (!req) return;
    clearTimeout(this.#previewTimer);
    this.#previewTimer = setTimeout(async () => {
      this.#previewAbort?.abort();
      const ctrl = new AbortController();
      this.#previewAbort = ctrl;
      const next: RecommendationRequest = {
        ...req,
        aggressiveness: patch.aggressiveness ?? req.aggressiveness,
        workholding: patch.workholding ?? req.workholding,
        stickout_mm: patch.stickout_mm ?? req.stickout_mm,
        overrides: { ...(patch.clear_overrides ? {} : req.overrides), ...patch.overrides },
      };
      try {
        const reco = await api.recommend(next, ctrl.signal);
        if (!ctrl.signal.aborted) this.preview = { id, reco };
      } catch {
        /* l'aperçu est facultatif */
      }
    }, 150);
  }

  cancelPreview() {
    clearTimeout(this.#previewTimer);
    this.#previewAbort?.abort();
    this.preview = null;
  }

  async diagnose(symptomId: string) {
    const req = this.request;
    if (!req) return;
    try {
      this.diagnosis = await api.diagnose(req, symptomId);
    } catch (e) {
      this.toast(message(e));
    }
  }

  // ---- Essais -------------------------------------------------------------

  /** Saisie d'essai pré-remplie avec le contexte et les valeurs affichées. */
  draftTrial(): TrialInput | null {
    const v = this.reco?.values;
    const s = this.setup;
    if (!v) return null;
    return {
      operator: this.operator,
      machine_id: s.machineId,
      material_id: s.materialId,
      tool_id: s.toolId,
      operation: s.operation,
      aggressiveness: s.aggressiveness,
      workholding: s.workholding,
      thickness_mm: s.thicknessMm,
      depth_mm: v.depth_mm,
      stepover_ratio: v.stepover_ratio,
      stickout_mm: this.reco?.metrics?.stickout_mm ?? null,
      tool_condition: null,
      rpm: v.rpm,
      feed_mm_min: v.feed_mm_min,
      plunge_mm_min: v.plunge_mm_min,
      doc_mm: v.doc_mm,
      outcome: 'good',
      symptoms: [],
      notes: '',
    };
  }

  async recordTrial(input: TrialInput): Promise<Trial | null> {
    try {
      const r = await this.#write(() => api.addTrial(input));
      if (!r) return null;
      const { trial } = r;
      this.operator = input.operator;
      // Les valeurs testées deviennent la référence : on retire les saisies manuelles.
      if (trial.outcome === 'good') {
        this.setup.overrides = {};
        this.baseline = null;
      }
      await this.refreshCatalog();
      this.toast(
        trial.outcome === 'good'
          ? t('toast.trialGood')
          : t('toast.trialSaved'),
      );
      if (trial.outcome !== 'good' && trial.symptoms.length) {
        this.assistantTab = 'problem';
        await this.diagnose(trial.symptoms[0]);
      } else {
        this.assistantTab = 'trials';
      }
      return trial;
    } catch (e) {
      this.toast(message(e));
      return null;
    }
  }

  async archiveTrial(id: string) {
    try {
      if (!(await this.#write(() => api.archiveTrial(id)))) return;
      await this.refreshCatalog();
      this.toast(t('toast.trialArchived'));
    } catch (e) {
      this.toast(message(e));
    }
  }

  // ---- Rôles ----------------------------------------------------------------

  async refreshAuth() {
    try {
      this.auth = await api.authStatus();
      if (!this.auth.admin) session.token = null;
    } catch {
      this.auth = { configured: false, admin: false };
    }
  }

  /** Vrai si l'on est administrateur, au besoin après saisie du mot de passe. */
  ensureAdmin(): Promise<boolean> {
    if (this.auth.admin) return Promise.resolve(true);
    this.#authWaiter?.(false);
    this.authDialog = this.auth.configured ? 'login' : 'setup';
    return new Promise((resolve) => (this.#authWaiter = resolve));
  }

  /** Fermeture de la fenêtre de mot de passe (succès ou annulation). */
  closeAuthDialog(ok: boolean) {
    this.authDialog = null;
    const w = this.#authWaiter;
    this.#authWaiter = null;
    w?.(ok);
  }

  async #signedIn(token: string, text: string) {
    session.token = token;
    this.auth = { configured: true, admin: true };
    this.toast(text);
    this.closeAuthDialog(true);
  }

  /** Renvoie le message d'erreur à afficher dans la fenêtre, ou null. */
  async login(password: string): Promise<string | null> {
    try {
      const { token } = await api.login(password);
      await this.#signedIn(token, t('auth.signedIn'));
      return null;
    } catch (e) {
      return message(e);
    }
  }

  async setupPassword(password: string): Promise<string | null> {
    try {
      const { token } = await api.setupPassword(password);
      await this.#signedIn(token, t('auth.passwordSet'));
      return null;
    } catch (e) {
      return message(e);
    }
  }

  async changePassword(current: string, next: string): Promise<string | null> {
    try {
      const { token } = await api.changePassword(current, next);
      await this.#signedIn(token, t('auth.passwordChanged'));
      return null;
    } catch (e) {
      return message(e);
    }
  }

  async logout() {
    try {
      await api.logout();
    } catch {
      /* hors ligne : la session expirera seule */
    }
    session.token = null;
    this.auth = { ...this.auth, admin: false };
    this.trialDialog = false;
    this.toast(t('auth.signedOut'));
  }

  /** Écriture réservée à l'administrateur ; null si l'utilisateur renonce. */
  async #write<T>(fn: () => Promise<T>, retried = false): Promise<T | null> {
    if (!(await this.ensureAdmin())) return null;
    try {
      return await fn();
    } catch (e) {
      // Session perdue (serveur redémarré, inactivité) : on redemande le mot de passe une fois.
      if (e instanceof ApiError && e.code === 'admin_required' && !retried) {
        session.token = null;
        this.auth = { ...this.auth, admin: false };
        return this.#write(fn, true);
      }
      throw e;
    }
  }

  async openTrialDialog() {
    if (await this.ensureAdmin()) this.trialDialog = true;
  }

  /** Essais du même contexte qu'un essai (détourage ≈ rainure), du plus récent au plus ancien. */
  contextTrials(trial: Trial): Trial[] {
    const group = (op: OperationKind) => (op === 'full_slot' ? 'profile' : op);
    return (this.catalog?.trials ?? [])
      .filter(
        (x) =>
          x.machine_id === trial.machine_id &&
          x.material_id === trial.material_id &&
          x.tool_id === trial.tool_id &&
          group(x.operation) === group(trial.operation),
      )
      .sort((a, b) => b.recorded_at.localeCompare(a.recorded_at));
  }

  async openValidateDialog(id: string) {
    if (await this.ensureAdmin()) this.validateDialog = id;
  }

  async validateTrial(id: string, validatedBy: string, note: string): Promise<boolean> {
    try {
      const r = await this.#write(() => api.validateTrial(id, { validated_by: validatedBy, note }));
      if (!r) return false;
      if (validatedBy.trim()) this.operator = validatedBy.trim();
      await this.refreshCatalog();
      this.toast(t('toast.trialValidated'));
      return true;
    } catch (e) {
      this.toast(message(e));
      return false;
    }
  }

  async unvalidateTrial(id: string) {
    try {
      if (!(await this.#write(() => api.unvalidateTrial(id)))) return;
      await this.refreshCatalog();
      this.toast(t('toast.validationRemoved'));
    } catch (e) {
      this.toast(message(e));
    }
  }

  // ---- Base du lab : sauvegarde, restauration, export -----------------------

  async downloadBackup() {
    try {
      if (!(await this.ensureAdmin())) return;
      const name = await this.#write(() => api.downloadBackup());
      if (name) this.toast(t('toast.backupSaved', { name }));
    } catch (e) {
      this.toast(message(e));
    }
  }

  async exportTrialsCsv() {
    try {
      const name = await api.exportTrialsCsv();
      this.toast(t('toast.csvSaved', { name }));
    } catch (e) {
      this.toast(message(e));
    }
  }

  /** Vérifie une sauvegarde sans rien modifier ; renvoie le résumé ou un message d'erreur. */
  async checkRestore(text: string): Promise<RestoreCheck | string | null> {
    try {
      const r = await this.#write(() => api.checkRestore(text));
      return r ? r.check : null;
    } catch (e) {
      return message(e);
    }
  }

  async restore(text: string): Promise<boolean> {
    try {
      const r = await this.#write(() => api.restore(text));
      if (!r) return false;
      this.diagnosis = null;
      this.setup.overrides = {};
      this.baseline = null;
      await this.refreshCatalog();
      this.toast(t('toast.restored', { file: r.safety_backup }));
      return true;
    } catch (e) {
      this.toast(message(e));
      return false;
    }
  }

  // ---- Bibliothèque ---------------------------------------------------------

  openLibrary(section: LibrarySection, selected: string | null = null, creating = false) {
    this.library = { section, selected, creating };
    this.module = 'library';
  }

  async saveTool(tool: Tool, isNew: boolean): Promise<Tool | null> {
    try {
      const r = await this.#write(() => (isNew ? api.createTool(tool) : api.updateTool(tool)));
      if (!r) return null;
      const saved = r.tool;
      await this.refreshCatalog();
      this.toast(t(isNew ? 'toast.toolAdded' : 'toast.toolSaved', { name: saved.name }));
      return saved;
    } catch (e) {
      this.toast(message(e));
      return null;
    }
  }

  async archiveTool(id: string) {
    try {
      const r = await this.#write(() => api.archiveTool(id));
      if (!r) return;
      await this.refreshCatalog();
      this.toast(r.restored_base ? t('toast.baseRestored') : t('toast.toolArchived'));
    } catch (e) {
      this.toast(message(e));
    }
  }

  async exportLibrary(kind?: LibraryKind) {
    try {
      await this.#write(() => api.exportLibrary(kind));
    } catch (e) {
      this.toast(message(e));
    }
  }

  async importLibrary(text: string, kind?: LibraryKind, check = false): Promise<ImportReport | string | null> {
    try {
      const report = await this.#write(() => api.importLibrary(text, kind, check));
      if (!report) return null;
      if (!check) {
        await this.refreshCatalog();
        this.toast(t('transfer.done', { added: report.added, skipped: report.skipped }));
      }
      return report;
    } catch (e) {
      return message(e);
    }
  }

  async saveMaterial(material: Material, isNew: boolean): Promise<Material | null> {
    try {
      const r = await this.#write(() => (isNew ? api.createMaterial(material) : api.updateMaterial(material)));
      if (!r) return null;
      const saved = r.material;
      await this.refreshCatalog();
      this.toast(t(isNew ? 'toast.materialAdded' : 'toast.materialSaved', { name: saved.name }));
      return saved;
    } catch (e) {
      this.toast(message(e));
      return null;
    }
  }

  async archiveMaterial(id: string) {
    try {
      const r = await this.#write(() => api.archiveMaterial(id));
      if (!r) return;
      await this.refreshCatalog();
      this.toast(r.restored_base ? t('toast.baseRestored') : t('toast.materialArchived'));
    } catch (e) {
      this.toast(message(e));
    }
  }

  async saveMachine(machine: Machine, isNew: boolean): Promise<Machine | null> {
    try {
      const r = await this.#write(() => (isNew ? api.createMachine(machine) : api.updateMachine(machine)));
      if (!r) return null;
      const saved = r.machine;
      await this.refreshCatalog();
      this.toast(t(isNew ? 'toast.machineAdded' : 'toast.machineSaved', { name: saved.name }));
      return saved;
    } catch (e) {
      this.toast(message(e));
      return null;
    }
  }

  async archiveMachine(id: string) {
    try {
      const r = await this.#write(() => api.archiveMachine(id));
      if (!r) return;
      await this.refreshCatalog();
      this.toast(r.restored_base ? t('toast.baseRestored') : t('toast.machineArchived'));
    } catch (e) {
      this.toast(message(e));
    }
  }
}

export const store = new Store();
