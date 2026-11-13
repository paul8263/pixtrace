import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const outputDir = resolve(process.argv[2] ?? "sample-workspace");
const count = Number(process.argv[3] ?? 36);
const small = process.argv[4] === "small";
const tiny = process.argv[4] === "tiny";

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
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

function textChunk(keyword, text) {
  const data = Buffer.concat([
    Buffer.from(keyword, "latin1"),
    Buffer.from([0]),
    Buffer.from(text, "utf8"),
  ]);
  return chunk("tEXt", data);
}

function makePng(width, height, hue, texts) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const stride = width * 3 + 1;
  const raw = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y += 1) {
    const offset = y * stride;
    raw[offset] = 0;
    for (let x = 0; x < width; x += 1) {
      const px = offset + 1 + x * 3;
      const wave = Math.sin((x / width) * Math.PI * 2 + hue) * 0.5 + 0.5;
      const shade = 0.25 + 0.75 * (y / height);
      raw[px] = Math.round(255 * wave * shade);
      raw[px + 1] = Math.round(255 * (1 - wave) * shade);
      raw[px + 2] = Math.round(255 * (0.35 + 0.5 * Math.abs(Math.sin(hue + x / 40))));
    }
  }

  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    ...texts.map(([keyword, value]) => textChunk(keyword, value)),
    chunk("IDAT", deflateSync(raw, { level: 6 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function sd15(seed) {
  return {
    prompt: {
      "3": {
        class_type: "KSampler",
        inputs: {
          seed,
          steps: 28,
          cfg: 7,
          sampler_name: "euler",
          scheduler: "normal",
          denoise: 1,
          model: ["4", 0],
          positive: ["6", 0],
          negative: ["7", 0],
          latent_image: ["5", 0],
        },
      },
      "4": {
        class_type: "CheckpointLoaderSimple",
        inputs: { ckpt_name: "sd_v1-5-pruned.safetensors" },
      },
      "5": {
        class_type: "EmptyLatentImage",
        inputs: { width: 512, height: 768, batch_size: 1 },
      },
      "6": {
        class_type: "CLIPTextEncode",
        inputs: { text: "masterpiece, best quality, 1girl, silver hair, moonlit garden", clip: ["4", 1] },
      },
      "7": {
        class_type: "CLIPTextEncode",
        inputs: { text: "lowres, bad anatomy, watermark, text", clip: ["4", 1] },
      },
    },
  };
}

function sdxl(seed) {
  return {
    prompt: {
      "1": {
        class_type: "KSampler",
        inputs: {
          seed,
          steps: 32,
          cfg: 8,
          sampler_name: "dpmpp_2m",
          scheduler: "karras",
          denoise: 1,
          positive: ["4", 0],
          negative: ["5", 0],
        },
      },
      "2": {
        class_type: "CheckpointLoaderSimple",
        inputs: { ckpt_name: "sd_xl_base_1.0.safetensors" },
      },
      "4": {
        class_type: "CLIPTextEncodeSDXL",
        inputs: {
          text_g: "cinematic photo, ancient city at dawn, volumetric light",
          text_l: "detailed stone architecture, warm sunrise, mist",
          width: 1024,
          height: 1024,
        },
      },
      "5": { class_type: "CLIPTextEncode", inputs: { text: "blurry, deformed" } },
      "6": {
        class_type: "LoraLoader",
        inputs: { lora_name: "detail_tweaker_xl.safetensors", strength_model: 0.6 },
      },
    },
  };
}

function flux(seed) {
  return {
    prompt: {
      "1": {
        class_type: "UNETLoader",
        inputs: { unet_name: "flux1-dev.safetensors", weight_dtype: "default" },
      },
      "2": {
        class_type: "CLIPTextEncode",
        inputs: { text: "a paper crane resting on a rainy window, soft focus" },
      },
      "3": { class_type: "BasicGuider", inputs: { conditioning: ["9", 0], model: ["1", 0] } },
      "4": {
        class_type: "SamplerCustomAdvanced",
        inputs: {
          noise: ["5", 0],
          guider: ["3", 0],
          sampler: ["6", 0],
          sigmas: ["7", 0],
          latent_image: ["8", 0],
        },
      },
      "5": { class_type: "RandomNoise", inputs: { noise_seed: seed } },
      "6": { class_type: "KSamplerSelect", inputs: { sampler_name: "euler" } },
      "7": {
        class_type: "BasicScheduler",
        inputs: { scheduler: "simple", steps: 20, denoise: 1, model: ["1", 0] },
      },
      "8": { class_type: "EmptySD3LatentImage", inputs: { width: 1024, height: 768 } },
      "9": { class_type: "FluxGuidance", inputs: { guidance: 3.5, conditioning: ["2", 0] } },
    },
  };
}

const styles = [
  { name: "sd15", build: sd15, size: [512, 768], hue: 0.4 },
  { name: "sdxl", build: sdxl, size: [1024, 1024], hue: 2.1 },
  { name: "flux", build: flux, size: [1024, 768], hue: 4.2 },
];

mkdirSync(outputDir, { recursive: true });

for (let index = 0; index < count; index += 1) {
  const style = styles[index % styles.length];
  const seed = 1000 + index * 17;
  const { prompt } = style.build(seed);
  const [width, height] = tiny ? [48, 32] : small ? [96, 64] : style.size;
  const png = makePng(width, height, style.hue + index * 0.15, [
    ["prompt", JSON.stringify(prompt)],
  ]);
  const name = `${style.name}-${String(index + 1).padStart(3, "0")}-seed${seed}.png`;
  writeFileSync(join(outputDir, name), png);
}

const plain = makePng(256, 256, 1.3, []);
writeFileSync(join(outputDir, "no-metadata.png"), plain);

const largeText = JSON.stringify({
  "1": {
    class_type: "CLIPTextEncode",
    inputs: { text: "very long prompt ".repeat(90_000) },
  },
});
writeFileSync(
  join(outputDir, "large-json.png"),
  makePng(512, 512, 3.2, [["prompt", largeText]]),
);

const broken = makePng(512, 512, 5.1, [["prompt", JSON.stringify(flux(7).prompt)]]);
writeFileSync(join(outputDir, "broken.png"), broken.subarray(0, broken.length - 40));

console.log(`generated ${count + 3} files in ${outputDir}`);
