// Formatage des nombres (selon la langue) et conversion d'unités pour l'affichage.
// L'API travaille toujours en millimètres ; seul l'affichage change.

import { i18n, intlLocale } from './i18n.svelte';

export type UnitSystem = 'mm' | 'in';

const MM_PER_IN = 25.4;
const cache = new Map<string, Intl.NumberFormat>();

/** Nombre dans la langue courante : 16 850,5 (fr) ou 16,850.5 (en). */
export function num(value: number | null | undefined, decimals = 0): string {
  if (value == null || !Number.isFinite(value)) return '—';
  const loc = intlLocale();
  const key = `${loc}:${decimals}`;
  let f = cache.get(key);
  if (!f) {
    f = new Intl.NumberFormat(loc, { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
    cache.set(key, f);
  }
  return f.format(value);
}

/** Nombre avec au plus `decimals` décimales, sans zéros inutiles. */
export function numTrim(value: number | null | undefined, decimals = 2): string {
  if (value == null || !Number.isFinite(value)) return '—';
  return new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: decimals }).format(value);
}

/** Nombre à coller dans un champ de FAO : décimales de la langue, sans séparateur de milliers. */
export function plain(value: number, decimals = 0): string {
  return new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: decimals, useGrouping: false }).format(value);
}

export const lengthUnit = (u: UnitSystem) => (u === 'mm' ? 'mm' : 'in');
export const speedUnit = (u: UnitSystem) => (u === 'mm' ? 'mm/min' : 'in/min');

export function toDisplayLength(mm: number, u: UnitSystem): number {
  return u === 'mm' ? mm : mm / MM_PER_IN;
}

export function fromDisplayLength(v: number, u: UnitSystem): number {
  return u === 'mm' ? v : v * MM_PER_IN;
}

export function lengthDecimals(u: UnitSystem, mmDecimals = 2): number {
  return u === 'mm' ? mmDecimals : 3;
}

export function fmtLength(mm: number | null | undefined, u: UnitSystem, mmDecimals = 2): string {
  if (mm == null) return '—';
  return numTrim(toDisplayLength(mm, u), lengthDecimals(u, mmDecimals));
}

export function fmtSpeed(mmMin: number | null | undefined, u: UnitSystem): string {
  if (mmMin == null) return '—';
  return u === 'mm' ? num(mmMin, 0) : num(mmMin / MM_PER_IN, 1);
}

/** Vitesse par seconde, telle que saisie dans les commandes MS/VS de ShopBot. */
export function fmtSpeedPerSecond(mmMin: number, u: UnitSystem): string {
  return u === 'mm' ? `${numTrim(mmMin / 60, 1)} mm/s` : `${numTrim(mmMin / 60 / MM_PER_IN, 2)} in/s`;
}

export function fmtDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds)) return '—';
  const s = Math.round(seconds);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  const r = s % 60;
  if (m < 60) return r ? `${m} min ${String(r).padStart(2, '0')} s` : `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${String(m % 60).padStart(2, '0')}`;
}

/** Pourcentage : « 12 % » en français, « 12% » en anglais. */
export const pct = (v: number, decimals = 0) => `${num(v, decimals)}${i18n.locale === 'fr' ? '\u202f' : ''}%`;
