/**
 * Generate the extension icons as PNG files without any image dependency.
 *
 * A 128x128 logical canvas is drawn with 4x supersampling, then reduced to
 * 16, 32, 48 and 128 pixels. Output: chrome/icons/<size>.png
 *
 * Usage: node browser-extension/tools/generate_icons.js
 */
import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'chrome', 'icons');

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let crc = -1;
  for (let i = 0; i < buffer.length; i += 1) {
    crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ buffer[i]) & 0xff];
  }
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([length, body, crc]);
}

function encodePng(width, height, rgba) {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type: RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  return Buffer.concat([
    signature,
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/* ------------------------------------------------------------------ *
 * Drawing (logical 128x128 canvas)
 * ------------------------------------------------------------------ */

const BG_TOP = [24, 116, 184];
const BG_BOTTOM = [12, 66, 110];
const FG = [255, 255, 255];

function mix(a, b, t) {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

function insideRoundedRect(x, y, left, top, right, bottom, radius) {
  const cx = Math.min(Math.max(x, left + radius), right - radius);
  const cy = Math.min(Math.max(y, top + radius), bottom - radius);
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= radius * radius;
}

function insideTriangle(x, y, ax, ay, bx, by, cx, cy) {
  const d1 = (x - bx) * (ay - by) - (ax - bx) * (y - by);
  const d2 = (x - cx) * (by - cy) - (bx - cx) * (y - cy);
  const d3 = (x - ax) * (cy - ay) - (cx - ax) * (y - ay);
  const negative = d1 < 0 || d2 < 0 || d3 < 0;
  const positive = d1 > 0 || d2 > 0 || d3 > 0;
  return !(negative && positive);
}

function sample(x, y) {
  // Background: rounded square with a vertical gradient.
  if (!insideRoundedRect(x, y, 6, 6, 122, 122, 22)) return null;
  const t = y / 128;
  let colour = mix(BG_TOP, BG_BOTTOM, t);

  // Tray line under the arrow.
  if (insideRoundedRect(x, y, 30, 106, 98, 116, 5)) return FG;

  // Arrow shaft.
  if (x >= 52 && x <= 76 && y >= 26 && y <= 72) return FG;

  // Arrow head.
  if (insideTriangle(x, y, 34, 68, 94, 68, 64, 102)) return FG;

  return colour;
}

function render(size) {
  const samples = 4;
  const rgba = Buffer.alloc(size * size * 4);
  const scale = 128 / size;

  for (let py = 0; py < size; py += 1) {
    for (let px = 0; px < size; px += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let alpha = 0;
      for (let sy = 0; sy < samples; sy += 1) {
        for (let sx = 0; sx < samples; sx += 1) {
          const x = (px + (sx + 0.5) / samples) * scale;
          const y = (py + (sy + 0.5) / samples) * scale;
          const colour = sample(x, y);
          if (colour) {
            r += colour[0];
            g += colour[1];
            b += colour[2];
            alpha += 255;
          }
        }
      }
      const total = samples * samples;
      const offset = (py * size + px) * 4;
      rgba[offset] = Math.round(r / total);
      rgba[offset + 1] = Math.round(g / total);
      rgba[offset + 2] = Math.round(b / total);
      rgba[offset + 3] = Math.round(alpha / total);
    }
  }
  return encodePng(size, size, rgba);
}

mkdirSync(OUT_DIR, { recursive: true });
for (const size of [16, 32, 48, 128]) {
  const png = render(size);
  writeFileSync(join(OUT_DIR, `${size}.png`), png);
  console.log(`wrote chrome/icons/${size}.png (${png.length} bytes)`);
}
