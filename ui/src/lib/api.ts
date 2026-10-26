// Client de l'API REST FabSpeed (/api/v1). Même origine que l'interface.
// Chaque requête porte la langue choisie (en-tête Accept-Language) : le
// serveur renvoie textes de données, messages du moteur et erreurs traduits.
// En mode administrateur, le jeton de session part dans Authorization ; il est
// gardé pour l'onglet seulement (sessionStorage) : fermer l'onglet repasse en invité.

import { i18n, t } from './i18n.svelte';
import type { Catalog, Diagnosis, Machine, Material, Recommendation, RecommendationRequest, Tool, Trial, TrialInput } from './types';

export type SharingKind = 'machines' | 'tools' | 'materials' | 'trials';
export interface SharingSettings { destination: string | null; consent: { enabled: boolean; machines: string[]; tools: string[]; materials: string[]; trials: string[] } }
export interface SharingStatus { supported_platform: boolean; settings: SharingSettings; phase: 'disabled' | 'configured' | 'pending' | 'publishing' | 'published' | 'outdated' | 'failed' | 'withdrawal_pending' | 'withdrawn' | 'unsafe'; error: string | null; default_destination: string | null; last: { generation: number; revision: string; published_at: string; destination: string; empty: boolean } | null; local_access_blocked: boolean; existing_may_be_readable: boolean }
export interface SharingChoice { id: string; name: string; published: boolean; eligible?: boolean; reason?: string; machine?: string; tool?: string; material?: string }
export interface SharingResponse { status: SharingStatus; choices: Record<SharingKind, SharingChoice[]> }
export interface SharingPreview { snapshot: Record<string, unknown>; excluded: string[]; preview_only: boolean }

export interface LabCounts {
  machines: number;
  materials: number;
  tools: number;
  /** Fiches ajoutées ou modifiées au lab (machines, matières, fraises). */
  lab_records: number;
  trials: number;
  validated: number;
  archived: number;
}

/** Vérification d'une sauvegarde avant restauration. */
export interface RestoreCheck {
  created_at: string;
  engine_version: string;
  dataset_revision: string;
  /** false : ancienne sauvegarde (format 1), seule la base du lab est remplacée. */
  full: boolean;
  backup: LabCounts;
  current: LabCounts;
  issues: string[];
}

export type LibraryKind = 'machines' | 'materials' | 'tools';
export interface ImportReport {
  added: number;
  skipped: number;
  records: { kind: LibraryKind; id: string; name: string; action: 'add' | 'skip'; reason: string }[];
  issues: string[];
}

export interface AuthStatus {
  /** Un mot de passe administrateur existe sur ce poste. */
  configured: boolean;
  admin: boolean;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public code: string,
    public status: number,
  ) {
    super(message);
  }
}

const TOKEN_KEY = 'fabspeed.admin.v1';

/** Jeton de la session administrateur (null = invité). */
export const session = {
  get token(): string | null {
    try {
      return sessionStorage.getItem(TOKEN_KEY);
    } catch {
      return null;
    }
  },
  set token(v: string | null) {
    try {
      if (v) sessionStorage.setItem(TOKEN_KEY, v);
      else sessionStorage.removeItem(TOKEN_KEY);
    } catch {
      /* stockage indisponible : session limitée à la page */
    }
  },
};

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    const headers = new Headers(init?.headers);
    headers.set('accept-language', i18n.locale);
    const token = session.token;
    if (token) headers.set('authorization', `Bearer ${token}`);
    res = await fetch(`/api/v1${path}`, { ...init, headers });
  } catch {
    throw new ApiError(t('api.offline'), 'offline', 0);
  }
  const text = await res.text();
  const body = text ? JSON.parse(text) : null;
  if (!res.ok) {
    throw new ApiError(body?.error ?? t('api.httpError', { status: res.status }), body?.code ?? 'http_error', res.status);
  }
  return body as T;
}

/** Télécharge un fichier renvoyé par l'API (sauvegarde, CSV) sous le nom proposé par le serveur. */
async function download(path: string): Promise<string> {
  let res: Response;
  try {
    const headers = new Headers({ 'accept-language': i18n.locale });
    const token = session.token;
    if (token) headers.set('authorization', `Bearer ${token}`);
    res = await fetch(`/api/v1${path}`, { headers });
  } catch {
    throw new ApiError(t('api.offline'), 'offline', 0);
  }
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new ApiError(body?.error ?? t('api.httpError', { status: res.status }), body?.code ?? 'http_error', res.status);
  }
  const name = /filename="([^"]+)"/.exec(res.headers.get('content-disposition') ?? '')?.[1] ?? 'fabspeed';
  const url = URL.createObjectURL(await res.blob());
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return name;
}

const json = (body: unknown): RequestInit => ({
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(body),
});

export const api = {
  sharing: () => call<SharingResponse>('/fabapps/sharing'),
  saveSharing: (settings: SharingSettings) => call<SharingStatus>('/fabapps/sharing', json(settings)),
  previewSharing: (settings: SharingSettings) => call<SharingPreview>('/fabapps/sharing/preview', json(settings)),
  publishSharing: () => call<SharingStatus>('/fabapps/sharing/publish', { method: 'POST' }),
  catalog: () => call<Catalog>('/catalog'),
  recommend: (req: RecommendationRequest, signal?: AbortSignal) =>
    call<Recommendation>('/recommend', { ...json(req), signal }),
  diagnose: (request: RecommendationRequest, symptom_id: string) =>
    call<Diagnosis>('/diagnose', json({ request, symptom_id })),
  importToolset: (fileText: string) =>
    call<{ tools: Tool[] }>('/import/ivac-toolset', { method: 'POST', body: fileText }),
  reload: () => call<{ status: string; dataset_revision: string }>('/reload', { method: 'POST' }),

  // Base du lab : sauvegarde (admin), vérification puis restauration (admin), export CSV (tous).
  downloadBackup: () => download('/lab/backup'),
  exportTrialsCsv: () => download('/trials.csv'),
  checkRestore: (backupText: string) =>
    call<{ check: RestoreCheck }>('/lab/restore?check=true', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: backupText,
    }),
  restore: (backupText: string) =>
    call<{ restored: RestoreCheck; safety_backup: string }>('/lab/restore', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: backupText,
    }),

  exportLibrary: (kind?: LibraryKind) => download(`/library/export${kind ? `?kind=${kind}` : ''}`),
  importLibrary: (content: string, kind?: LibraryKind, check = false) =>
    call<ImportReport>(`/library/import${check ? '?check=true' : ''}`, json({ content, kind })),

  // Rôles : invité (lecture, calcul) ou administrateur (écriture).
  authStatus: () => call<AuthStatus>('/auth'),
  setupPassword: (password: string) => call<{ token: string }>('/auth/setup', json({ password })),
  login: (password: string) => call<{ token: string }>('/auth/login', json({ password })),
  logout: () => call<{ admin: false }>('/auth/logout', { method: 'POST' }),
  changePassword: (current: string, next: string) =>
    call<{ token: string }>('/auth/password', json({ current, new: next })),

  // Écriture dans data/lab/ (un fichier par fiche, suppression = archivage).
  addTrial: (input: TrialInput) => call<{ trial: Trial }>('/trials', json(input)),
  archiveTrial: (id: string) => call<{ archived: string }>(`/trials/${id}`, { method: 'DELETE' }),
  validateTrial: (id: string, body: { validated_by: string; note: string }) =>
    call<{ trial: Trial }>(`/trials/${id}/validation`, json(body)),
  unvalidateTrial: (id: string) => call<{ trial: Trial }>(`/trials/${id}/validation`, { method: 'DELETE' }),
  createTool: (tool: Tool) => call<{ tool: Tool }>('/tools', json(tool)),
  updateTool: (tool: Tool) => call<{ tool: Tool }>(`/tools/${tool.id}`, { ...json(tool), method: 'PUT' }),
  archiveTool: (id: string) => call<{ archived: string; restored_base: boolean }>(`/tools/${id}`, { method: 'DELETE' }),
  createMaterial: (m: Material) => call<{ material: Material }>('/materials', json(m)),
  updateMaterial: (m: Material) => call<{ material: Material }>(`/materials/${m.id}`, { ...json(m), method: 'PUT' }),
  archiveMaterial: (id: string) =>
    call<{ archived: string; restored_base: boolean }>(`/materials/${id}`, { method: 'DELETE' }),
  createMachine: (m: Machine) => call<{ machine: Machine }>('/machines', json(m)),
  updateMachine: (m: Machine) => call<{ machine: Machine }>(`/machines/${m.id}`, { ...json(m), method: 'PUT' }),
  archiveMachine: (id: string) =>
    call<{ archived: string; restored_base: boolean }>(`/machines/${id}`, { method: 'DELETE' }),
};
