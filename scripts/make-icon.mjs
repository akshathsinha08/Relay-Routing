import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";

const SIZE = 1024;

function crc32(buf) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

function encodePNG(width, height, rgba) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0;
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const smooth = (d, edge) => Math.min(1, Math.max(0, (edge - d) / 2.5));
const mix = (a, b, t) => a + (b - a) * t;

const pixels = Buffer.alloc(SIZE * SIZE * 4);
const c = SIZE / 2;
const radius = SIZE * 0.225; // rounded-square corner radius
const half = SIZE * 0.5 - SIZE * 0.07;
const ringOuter = SIZE * 0.265;
const ringInner = SIZE * 0.185;

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const px = x + 0.5;
    const py = y + 0.5;
    const qx = Math.abs(px - c) - half + radius;
    const qy = Math.abs(py - c) - half + radius;
    const dSquare = Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - radius;
    const bgAlpha = smooth(Math.abs(dSquare), 1.2);

    const t = (px / SIZE) * 0.6 + (py / SIZE) * 0.4;
    let r = mix(32, 79, t);
    let g = mix(36, 70, t);
    let b = mix(52, 229, t);

    // Glow
    const glow = Math.max(0, 1 - Math.hypot(px - c * 0.72, py - c * 0.42) / (SIZE * 0.5));
    r = Math.min(255, r + glow * 18);
    g = Math.min(255, g + glow * 12);
    b = Math.min(255, b + glow * 26);

    // Stylized C: ring with a right-side opening.
    const dist = Math.hypot(px - c, py - c);
    const angle = Math.atan2(py - c, px - c); // -pi..pi
    const opening = Math.abs(angle) < 0.5; // right side gap
    const ring = smooth(Math.abs(dist - (ringOuter + ringInner) / 2), (ringOuter - ringInner) / 2) * (opening ? 0 : 1);
    const aa = smooth(Math.abs(dist - (ringOuter + ringInner) / 2) - (ringOuter - ringInner) / 2, 1.2);
    const ringMask = (1 - aa) * (opening ? 0 : 1);

    // Small accent dot at the opening
    const dot = 1 - smooth(Math.hypot(px - (c + ringOuter * 0.96), py - c), SIZE * 0.035);

    const white = Math.max(ringMask, dot);
    r = mix(r, 247, white);
    g = mix(g, 248, white);
    b = mix(b, 255, white);

    const alpha = bgAlpha;
    const i = (y * SIZE + x) * 4;
    pixels[i] = Math.round(r);
    pixels[i + 1] = Math.round(g);
    pixels[i + 2] = Math.round(b);
    pixels[i + 3] = Math.round(alpha * 255);
    void ring;
  }
}

const out = join(dirname(new URL(import.meta.url).pathname), "..", "src-tauri", "icons");
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "source.png"), encodePNG(SIZE, SIZE, pixels));
console.log("wrote", join(out, "source.png"));
