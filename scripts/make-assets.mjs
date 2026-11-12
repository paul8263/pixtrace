import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function crc32(buffer) {
  let crc = 0xffffffff;
  for (let i = 0; i < buffer.length; i++) {
    let value = (crc ^ buffer[i]) & 0xff;
    for (let bit = 0; bit < 8; bit++) {
      value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
    }
    crc = (crc >>> 8) ^ value;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const typeBuffer = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])));
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function png(width, height, pixel) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;

  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    const row = y * (width * 4 + 1);
    raw[row] = 0;
    for (let x = 0; x < width; x++) {
      const [r, g, b, a] = pixel(x, y, width, height);
      const offset = row + 1 + x * 4;
      raw[offset] = r;
      raw[offset + 1] = g;
      raw[offset + 2] = b;
      raw[offset + 3] = a;
    }
  }

  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const background = [16, 17, 20];
const accent = [124, 196, 255];

const icon = png(512, 512, (x, y, width, height) => {
  const dx = x - width / 2;
  const dy = y - height / 2;
  const distance = Math.sqrt(dx * dx + dy * dy);
  if (distance > 190) return [...background, 255];
  if (distance > 135 || distance < 62) return [...accent, 255];
  return [24, 26, 31, 255];
});

const protocolTest = png(320, 200, (x, y, width, height) => {
  if (y > height - 34) return [...accent, 255];
  const checker = (Math.floor(x / 20) + Math.floor(y / 20)) % 2 === 0;
  return checker ? [30, 32, 38, 255] : [22, 24, 28, 255];
});

writeFileSync(join(root, "app-icon.png"), icon);
const assetsDir = join(root, "src-tauri", "assets");
mkdirSync(assetsDir, { recursive: true });
writeFileSync(join(assetsDir, "protocol-test.png"), protocolTest);

console.log("app-icon.png and src-tauri/assets/protocol-test.png written");
