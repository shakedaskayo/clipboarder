// Parse a color string into {r,g,b,a} in 0-255 / 0-1.
export interface RGBA { r: number; g: number; b: number; a: number; }

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const RGB = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:[\s,/]+([\d.]+%?))?\s*\)$/i;
const HSL = /^hsla?\(\s*([\d.]+)(?:deg)?[\s,]+([\d.]+)%[\s,]+([\d.]+)%(?:[\s,/]+([\d.]+%?))?\s*\)$/i;

export function parseColor(input: string): RGBA | null {
  const s = input.trim();
  let m = s.match(HEX);
  if (m) {
    let h = m[1];
    if (h.length === 3 || h.length === 4) h = h.split("").map(c => c + c).join("");
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    const a = h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1;
    return { r, g, b, a };
  }
  m = s.match(RGB);
  if (m) {
    return {
      r: clamp(+m[1]), g: clamp(+m[2]), b: clamp(+m[3]),
      a: m[4] ? alpha(m[4]) : 1,
    };
  }
  m = s.match(HSL);
  if (m) {
    const { r, g, b } = hslToRgb(+m[1], +m[2] / 100, +m[3] / 100);
    return { r, g, b, a: m[4] ? alpha(m[4]) : 1 };
  }
  return null;
}

export function toHex({ r, g, b, a }: RGBA): string {
  const h = (n: number) => n.toString(16).padStart(2, "0");
  if (a < 1) return `#${h(r)}${h(g)}${h(b)}${h(Math.round(a * 255))}`;
  return `#${h(r)}${h(g)}${h(b)}`;
}

export function toRgb({ r, g, b, a }: RGBA): string {
  if (a < 1) return `rgba(${r}, ${g}, ${b}, ${+a.toFixed(2)})`;
  return `rgb(${r}, ${g}, ${b})`;
}

export function toHsl({ r, g, b, a }: RGBA): string {
  const { h, s, l } = rgbToHsl(r, g, b);
  if (a < 1) return `hsla(${h}, ${s}%, ${l}%, ${+a.toFixed(2)})`;
  return `hsl(${h}, ${s}%, ${l}%)`;
}

export function cssColor({ r, g, b, a }: RGBA): string {
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

function clamp(n: number): number { return Math.max(0, Math.min(255, Math.round(n))); }
function alpha(s: string): number {
  if (s.endsWith("%")) return Math.max(0, Math.min(1, parseFloat(s) / 100));
  return Math.max(0, Math.min(1, parseFloat(s)));
}

function hslToRgb(h: number, s: number, l: number) {
  h = ((h % 360) + 360) % 360 / 360;
  if (s === 0) {
    const v = Math.round(l * 255);
    return { r: v, g: v, b: v };
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  return {
    r: Math.round(hue(p, q, h + 1/3) * 255),
    g: Math.round(hue(p, q, h) * 255),
    b: Math.round(hue(p, q, h - 1/3) * 255),
  };
}

function hue(p: number, q: number, t: number) {
  if (t < 0) t += 1;
  if (t > 1) t -= 1;
  if (t < 1/6) return p + (q - p) * 6 * t;
  if (t < 1/2) return q;
  if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
  return p;
}

function rgbToHsl(r: number, g: number, b: number) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s = 0; const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = (g - b) / d + (g < b ? 6 : 0); break;
      case g: h = (b - r) / d + 2; break;
      case b: h = (r - g) / d + 4; break;
    }
    h *= 60;
  }
  return { h: Math.round(h), s: Math.round(s * 100), l: Math.round(l * 100) };
}
