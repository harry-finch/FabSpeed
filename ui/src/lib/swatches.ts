// Teinte représentative de chaque matière (nuancier et schéma de coupe).
// Couleurs d'identité de matière, sans valeur sémantique d'état.

import type { MaterialCategory } from './types';

const BY_ID: Record<string, string> = {
  'material/wood.plywood': '#c49a64',
  'material/wood.mdf': '#9a7650',
  'material/wood.osb': '#b8955a',
  'material/wood.softwood': '#d9b77e',
  'material/wood.oak': '#a8784a',
  'material/wood.hardwood': '#94623d',
  'material/composite.hpl': '#6d6f78',
  'material/plastic.pmma': '#8cc5e3',
  'material/plastic.pom': '#e6e2d6',
  'material/plastic.hdpe': '#d8dde0',
  'material/plastic.pvc-foam': '#eeeeee',
  'material/foam.xps': '#9fc7e8',
  'material/foam.tooling-board': '#c9a87a',
  'material/metal.aluminium': '#b9c2cc',
  'material/metal.brass': '#d1b052',
};

const BY_CATEGORY: Record<MaterialCategory, string> = {
  panel: '#c49a64',
  wood: '#a8784a',
  plastic: '#8cc5e3',
  foam: '#e6d9a8',
  composite: '#6d6f78',
  metal: '#b9c2cc',
};

export function swatch(id: string, category: MaterialCategory): string {
  return BY_ID[id] ?? BY_CATEGORY[category];
}
