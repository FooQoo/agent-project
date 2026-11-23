#!/usr/bin/env node
// Compares freshly captured images and videos with the files committed in the repository, by what they show.
//
//   node e2e/scripts/compare-captures.mjs --captured <dir> [--against <dir>] [--image-tolerance <n>] [--apply] [--json]
//
// <dir> is an AGPJ_MEDIA_OUT folder of the capture scripts (readme/, guide/, videos/). Each file is compared with
// its committed counterpart: readme/ with docs/images/, guide/ with docs/images/guide/, videos/ with
// docs/public/videos/. With --against, <dir> is compared with another AGPJ_MEDIA_OUT folder instead
// (to check that two captures match).
//
// A PNG is hashed over its decoded RGBA pixels, so encoder metadata and compression level do not count as a
// change. An MP4 is decoded by ffmpeg to raw RGBA frames at a fixed rate (1 per second, scaled to 480 x 300) and
// those are hashed the same way. Only files whose visuals differ (or that are new) are reported.
//
// A recorded video is timed by the wall clock, so two captures of the same commit never match frame for frame:
// a transition is caught a few frames apart. When the hashes differ, the frames are therefore compared one by one
// against the nearest second on either side, and the video counts as changed only when many frames differ
// (see VIDEO_* below). A screenshot is exact by default; --image-tolerance <n> lets a pixel differ by up to n in
// any channel, which absorbs anti-aliasing noise at an edge.
//
// Exit code: 0 when nothing differs, 1 when something does, 2 on a usage error or an unreadable file.
// --json prints {"changed":[{"file","status","target"}],"unchanged":N}. --apply copies each changed file over its
// committed counterpart (a changed video also takes its .jpg poster). Needs ffmpeg only when videos are compared.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const layout = { readme: 'docs/images', guide: 'docs/images/guide', videos: 'docs/public/videos' };
const FRAME = { width: 480, height: 300 }; // Video frames are decoded at this size, one per second.
const VIDEO_PIXEL_DELTA = 24; // A video pixel differs when a channel moves by more than this.
const VIDEO_FRAME_SHARE = 0.003; // A video frame differs when more than this share of its pixels differ...
const VIDEO_CHANGED_FRAMES = 0.1; // ...and the video changes when more than this share of its frames differ.

/** Decodes a non-interlaced 8-bit PNG (what Chromium writes) to RGBA bytes. */
export function decodePng(buffer) {
  if (buffer.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG file');
  let offset = 8;
  let header;
  let palette;
  const data = [];
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString('latin1', offset + 4, offset + 8);
    const body = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === 'IHDR') {
      header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4), depth: body[8], color: body[9], interlace: body[12] };
    } else if (type === 'PLTE') palette = body;
    else if (type === 'IDAT') data.push(body);
    else if (type === 'IEND') break;
    offset += length + 12;
  }
  if (!header || header.depth !== 8 || header.interlace !== 0) throw new Error('only 8-bit non-interlaced PNG is supported');
  const channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[header.color];
  if (!channels) throw new Error(`unsupported PNG color type ${header.color}`);
  const { width, height } = header;
  const stride = width * channels;
  const raw = inflateSync(Buffer.concat(data));
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y += 1) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x += 1) {
      const left = x >= channels ? pixels[y * stride + x - channels] : 0;
      const up = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const upLeft = y > 0 && x >= channels ? pixels[(y - 1) * stride + x - channels] : 0;
      let predictor = 0;
      if (filter === 1) predictor = left;
      else if (filter === 2) predictor = up;
      else if (filter === 3) predictor = (left + up) >> 1;
      else if (filter === 4) {
        const p = left + up - upLeft;
        const [pa, pb, pc] = [Math.abs(p - left), Math.abs(p - up), Math.abs(p - upLeft)];
        predictor = pa <= pb && pa <= pc ? left : pb <= pc ? up : upLeft;
      } else if (filter !== 0) throw new Error(`bad PNG filter ${filter}`);
      pixels[y * stride + x] = (line[x] + predictor) & 255;
    }
  }
  const rgba = Buffer.alloc(width * height * 4);
  for (let i = 0; i < width * height; i += 1) {
    const s = i * channels;
    let r; let g; let b; let a = 255;
    if (header.color === 6) [r, g, b, a] = [pixels[s], pixels[s + 1], pixels[s + 2], pixels[s + 3]];
    else if (header.color === 2) [r, g, b] = [pixels[s], pixels[s + 1], pixels[s + 2]];
    else if (header.color === 0) r = g = b = pixels[s];
    else if (header.color === 4) { r = g = b = pixels[s]; a = pixels[s + 1]; } else [r, g, b] = [palette[pixels[s] * 3], palette[pixels[s] * 3 + 1], palette[pixels[s] * 3 + 2]];
    rgba.set([r, g, b, a], i * 4);
  }
  return { width, height, rgba };
}

/** Decodes a video to raw RGBA frames, one per second. */
export function videoFrames(file) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    const child = spawn('ffmpeg', ['-v', 'error', '-i', file, '-vf', `fps=1,scale=${FRAME.width}:${FRAME.height}:flags=bicubic`,
      '-f', 'rawvideo', '-pix_fmt', 'rgba', '-'], { stdio: ['ignore', 'pipe', 'pipe'] });
    let error = '';
    child.stdout.on('data', (chunk) => chunks.push(chunk));
    child.stderr.on('data', (chunk) => { error += chunk; });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code !== 0) return reject(new Error(`ffmpeg failed on ${file}: ${error}`));
      const all = Buffer.concat(chunks);
      const size = FRAME.width * FRAME.height * 4;
      const frames = [];
      for (let at = 0; at + size <= all.length; at += size) frames.push(all.subarray(at, at + size));
      return resolve(frames);
    });
  });
}

const hashFrames = (frames) => {
  const hash = createHash('sha256');
  for (const frame of frames) hash.update(frame);
  return hash.digest('hex');
};

/** The share of pixels that differ between two RGBA buffers of the same size, by more than `delta` in a channel. */
function differingShare(a, b, delta) {
  let count = 0;
  for (let i = 0; i < a.length; i += 4) {
    if (Math.abs(a[i] - b[i]) > delta || Math.abs(a[i + 1] - b[i + 1]) > delta
      || Math.abs(a[i + 2] - b[i + 2]) > delta || Math.abs(a[i + 3] - b[i + 3]) > delta) count += 1;
  }
  return count / (a.length / 4);
}

/** Whether two sets of frames show the same video, allowing for the timing noise of a recording. */
export function sameVideo(a, b) {
  if (hashFrames(a) === hashFrames(b)) return true;
  if (Math.abs(a.length - b.length) > 3) return false;
  let different = 0;
  for (let i = 0; i < a.length; i += 1) {
    const near = [i - 1, i, i + 1].filter((j) => j >= 0 && j < b.length);
    const best = Math.min(...near.map((j) => differingShare(a[i], b[j], VIDEO_PIXEL_DELTA)));
    if (best > VIDEO_FRAME_SHARE) different += 1;
  }
  return different <= a.length * VIDEO_CHANGED_FRAMES;
}

/** Whether two PNG files show the same pixels, each channel within `tolerance`. */
export function samePng(fileA, fileB, tolerance = 0) {
  const a = decodePng(readFileSync(fileA));
  const b = decodePng(readFileSync(fileB));
  if (a.width !== b.width || a.height !== b.height) return false;
  if (tolerance === 0) return a.rgba.equals(b.rgba);
  for (let i = 0; i < a.rgba.length; i += 1) if (Math.abs(a.rgba[i] - b.rgba[i]) > tolerance) return false;
  return true;
}

async function same(fresh, base, options) {
  if (fresh.endsWith('.png')) return samePng(fresh, base, options.imageTolerance);
  return sameVideo(await videoFrames(fresh), await videoFrames(base));
}

function parse(argv) {
  const options = { apply: false, json: false, imageTolerance: 0 };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === '--captured') options.captured = argv[++i];
    else if (argv[i] === '--against') options.against = argv[++i];
    else if (argv[i] === '--image-tolerance') options.imageTolerance = Number(argv[++i]);
    else if (argv[i] === '--apply') options.apply = true;
    else if (argv[i] === '--json') options.json = true;
    else throw new Error(`unknown option ${argv[i]}`);
  }
  if (!Number.isInteger(options.imageTolerance) || options.imageTolerance < 0) throw new Error('--image-tolerance must be a whole number, 0 or more');
  if (!options.captured) throw new Error('--captured <dir> is required');
  if (options.apply && options.against) throw new Error('--apply works on the repository only, not with --against');
  return options;
}

async function main() {
  let options;
  try { options = parse(process.argv.slice(2)); } catch (error) {
    console.error(`${error.message}\nusage: compare-captures.mjs --captured <dir> [--against <dir>] [--image-tolerance <n>] [--apply] [--json]`);
    return 2;
  }
  const captured = path.resolve(options.captured);
  const changed = [];
  let unchanged = 0;
  for (const [group, repoDir] of Object.entries(layout)) {
    const from = path.join(captured, group);
    if (!existsSync(from)) continue;
    const baseDir = options.against ? path.join(path.resolve(options.against), group) : path.join(repo, repoDir);
    for (const name of readdirSync(from).sort()) {
      const fresh = path.join(from, name);
      const base = path.join(baseDir, name);
      if (!/\.(png|mp4)$/.test(name)) continue;
      const status = !existsSync(base) ? 'added' : (await same(fresh, base, options)) ? null : 'changed';
      if (status) changed.push({ file: `${group}/${name}`, status, target: path.relative(repo, base) });
      else unchanged += 1;
      if (status && options.apply) {
        mkdirSync(path.dirname(base), { recursive: true });
        copyFileSync(fresh, base);
        // The poster frame of a changed video travels with it.
        const poster = fresh.replace(/\.mp4$/, '.jpg');
        if (name.endsWith('.mp4') && existsSync(poster)) copyFileSync(poster, base.replace(/\.mp4$/, '.jpg'));
      }
    }
  }
  if (changed.length + unchanged === 0) {
    console.error(`no PNG or MP4 files found under ${captured}`);
    return 2;
  }
  if (options.json) console.log(JSON.stringify({ changed, unchanged }));
  else {
    for (const entry of changed) console.log(`${entry.status}: ${entry.file}`);
    console.log(`${changed.length} differ, ${unchanged} identical${options.apply && changed.length ? '; copied into place' : ''}`);
  }
  return changed.length ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().then((code) => process.exit(code), (error) => { console.error(error.message); process.exit(2); });
}
