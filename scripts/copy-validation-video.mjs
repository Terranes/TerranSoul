#!/usr/bin/env node
/**
 * copy-validation-video.mjs
 *
 * Finds the video recorded by the self-validation Playwright test and copies it
 * to `recording/validation.webm`, replacing any previous recording.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');
const resultsDir = path.join(root, 'test-results', 'validation');
const recordingDir = path.join(root, 'recording');

function findVideos(dir) {
  const videos = [];
  if (!fs.existsSync(dir)) return videos;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      videos.push(...findVideos(full));
    } else if (entry.name.endsWith('.webm')) {
      videos.push(full);
    }
  }
  return videos;
}

const videos = findVideos(resultsDir);

if (videos.length === 0) {
  console.error('❌ No validation video found in', resultsDir);
  process.exit(1);
}

// Pick the most recently modified video
videos.sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
const source = videos[0];

// Clear old recordings
if (fs.existsSync(recordingDir)) {
  for (const f of fs.readdirSync(recordingDir)) {
    const full = path.join(recordingDir, f);
    if (f !== '.gitkeep' && fs.statSync(full).isFile()) {
      fs.unlinkSync(full);
    }
  }
} else {
  fs.mkdirSync(recordingDir, { recursive: true });
}

const dest = path.join(recordingDir, 'validation.webm');
fs.copyFileSync(source, dest);
console.log(`✅ Validation video saved to ${path.relative(root, dest)} (${(fs.statSync(dest).size / 1024 / 1024).toFixed(1)} MB)`);
