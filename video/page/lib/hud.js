// Panel helpers: numbers as an instrument shows them (fixed width, units always).
import { level } from './draw.js';

const SUP = { '-': '⁻', 0: '⁰', 1: '¹', 2: '²', 3: '³', 4: '⁴', 5: '⁵', 6: '⁶', 7: '⁷', 8: '⁸', 9: '⁹' };

// 2.511e-6 -> "2.511×10⁻⁶"; values in [0.01, 1000) stay plain
export function sci(v, digits = 3) {
  if (!Number.isFinite(v)) return '–';
  if (v === 0) return '0';
  const a = Math.abs(v);
  if (a >= 0.01 && a < 1000) return v.toFixed(a >= 100 ? 1 : a >= 10 ? 2 : 3);
  const [m, e] = v.toExponential(digits - 1).split('e');
  const exp = String(Number(e)).split('').map((c) => SUP[c]).join('');
  return `${m}×10${exp}`;
}
export const fmtJ = (v, d = 3) => `${sci(v, d)} J`;
export const fmtW = (v, d = 3) => `${sci(v, d)} W`;

// one stage of an energy chain: value now, log level, peak held over the last moments
export function meter(k, c, value, max, peak, fmt, rangeDb = 50) {
  return { k, c, lv: level(value, max, rangeDb), pk: level(peak, max, rangeDb), v: fmt(value, 2) };
}
