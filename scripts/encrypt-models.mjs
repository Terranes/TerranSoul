#!/usr/bin/env node
// SPDX-License-Identifier: MIT
//
// scripts/encrypt-models.mjs
//
// Build-time VRM encryption pipeline (Phase 7, Chunk 101).
//
// What this does:
//   * Reads plaintext .vrm files from a SOURCE directory
//   * AES-256-GCM encrypts each one (random 12-byte nonce per file)
//   * Writes <id>.vrm.enc to public/models/default/
//   * Writes a per-build random 32-byte AES key sidecar
//   * Writes manifest.json with sha256(plaintext), sha256(ciphertext),
//     nonce, sizes, ids, etc.
//   * Ed25519-signs manifest.json with the model-signing private key
//     (resolved from MODEL_SIGNING_PRIVATE_KEY env or
//     scripts/.model-signing-key.pem) and writes manifest.sig.
//
// What this does NOT do:
//   * It is NOT cryptographic protection in the Hollywood-DRM sense.
//   * In an open-source Tauri app the AES key SIDECAR ships with the
//     binary; anyone with the install can decrypt. See SECURITY-MODEL.md.
//   * This pipeline raises the bar against casual rip / dataset crawl,
//     and prevents network/devtools interception of the raw .vrm bytes
//     when combined with the Rust load_vrm_secure command and the strict
//     CSP. That is the honest extent of the protection.
//
// Usage:
//   MODELS_SOURCE=./models-source node scripts/encrypt-models.mjs
//
// MODELS_SOURCE defaults to ./models-source. Each subdirectory in
// MODELS_SOURCE that is named after a default-models.ts id and contains
// <id>.vrm will be encrypted. Layout:
//   models-source/annabelle.vrm
//   models-source/m58.vrm
//
// Env vars:
//   MODELS_SOURCE              path to plaintext .vrm input dir
//   MODELS_OUT                 path to encrypted output dir
//                              (default: public/models/default)
//   MODEL_SIGNING_PRIVATE_KEY  PEM-encoded Ed25519 private key (CI secret)
//   MODEL_SIGNING_KEY_FILE     fallback: path to the PEM file on disk
//                              (default: scripts/.model-signing-key.pem)
//
// Exit codes:
//   0 success
//   1 missing input / missing key / IO error

import { createHash, randomBytes, createCipheriv, generateKeyPairSync,
         createPrivateKey, sign as cryptoSign, createPublicKey } from 'node:crypto';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const MODELS_SOURCE = process.env.MODELS_SOURCE || 'models-source';
const MODELS_OUT    = process.env.MODELS_OUT    || path.join('public', 'models', 'default');
const KEY_FILE      = process.env.MODEL_SIGNING_KEY_FILE
                      || path.join('scripts', '.model-signing-key.pem');

// Known default model ids — must stay in sync with src/config/default-models.ts.
// The script only encrypts known ids so a typo in MODELS_SOURCE doesn't
// silently ship an unintended file.
const DEFAULT_MODEL_IDS = ['annabelle', 'm58'];

async function fileExists(p) {
  try { await fs.access(p); return true; } catch { return false; }
}

async function loadPrivateKey() {
  const envPem = process.env.MODEL_SIGNING_PRIVATE_KEY;
  if (envPem && envPem.trim().length > 0) {
    return createPrivateKey({ key: envPem, format: 'pem' });
  }
  if (await fileExists(KEY_FILE)) {
    const pem = await fs.readFile(KEY_FILE, 'utf8');
    return createPrivateKey({ key: pem, format: 'pem' });
  }
  throw new Error(
    `No model-signing key found. Provide MODEL_SIGNING_PRIVATE_KEY env ` +
    `or place a PEM at ${KEY_FILE}. Generate one with: ` +
    `node scripts/generate-vrm-signing-key.mjs`
  );
}

function sha256Hex(buf) {
  return createHash('sha256').update(buf).digest('hex');
}

function aesGcmEncrypt(plaintext, key, nonce) {
  const cipher = createCipheriv('aes-256-gcm', key, nonce);
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const tag = cipher.getAuthTag(); // 16 bytes
  // Wire format: nonce(12) || ciphertext || tag(16). The Rust loader
  // splits these back apart in the same order.
  return Buffer.concat([nonce, ciphertext, tag]);
}

async function main() {
  const sourceExists = await fileExists(MODELS_SOURCE);
  if (!sourceExists) {
    console.error(`[encrypt-models] MODELS_SOURCE not found: ${MODELS_SOURCE}`);
    console.error('[encrypt-models] Place plaintext VRMs (e.g. annabelle.vrm) under that directory.');
    process.exit(1);
  }

  await fs.mkdir(MODELS_OUT, { recursive: true });

  // Per-build random AES-256 key.
  const aesKey = randomBytes(32);
  const keyOut = path.join(MODELS_OUT, 'key.bin');
  await fs.writeFile(keyOut, aesKey, { mode: 0o644 });

  const entries = [];
  for (const id of DEFAULT_MODEL_IDS) {
    const plain = path.join(MODELS_SOURCE, `${id}.vrm`);
    if (!await fileExists(plain)) {
      console.warn(`[encrypt-models] skip ${id}: ${plain} not found`);
      continue;
    }
    const plaintext = await fs.readFile(plain);
    const nonce = randomBytes(12);
    const wrapped = aesGcmEncrypt(plaintext, aesKey, nonce);
    const outPath = path.join(MODELS_OUT, `${id}.vrm.enc`);
    await fs.writeFile(outPath, wrapped);
    entries.push({
      id,
      file: `${id}.vrm.enc`,
      nonce_hex: nonce.toString('hex'),
      sha256_plain: sha256Hex(plaintext),
      sha256_enc: sha256Hex(wrapped),
      size_plain: plaintext.length,
      size_enc: wrapped.length,
    });
    console.log(`[encrypt-models] encrypted ${id} (${plaintext.length} → ${wrapped.length} bytes)`);
  }

  if (entries.length === 0) {
    console.error('[encrypt-models] no models encrypted — aborting (would ship empty manifest)');
    process.exit(1);
  }

  const privateKey = await loadPrivateKey();
  const publicKey = createPublicKey(privateKey);
  const pubDer = publicKey.export({ type: 'spki', format: 'der' });
  // Ed25519 SPKI = 12-byte ASN.1 prefix + 32-byte raw key. We strip the
  // prefix so the Rust binary can compare against a 32-byte constant.
  const pubRaw = pubDer.subarray(pubDer.length - 32);

  const manifest = {
    schema_version: 1,
    generated_at: new Date().toISOString(),
    cipher: 'aes-256-gcm',
    key_file: 'key.bin',
    key_sha256: sha256Hex(aesKey),
    signing_pubkey_hex: pubRaw.toString('hex'),
    entries,
  };
  const manifestBytes = Buffer.from(JSON.stringify(manifest, null, 2));
  const manifestPath = path.join(MODELS_OUT, 'manifest.json');
  await fs.writeFile(manifestPath, manifestBytes);

  // Ed25519 signs raw bytes (no hash prefix). Sign the canonical manifest.
  const sig = cryptoSign(null, manifestBytes, privateKey);
  await fs.writeFile(path.join(MODELS_OUT, 'manifest.sig'), sig);

  console.log(`[encrypt-models] wrote manifest with ${entries.length} entries`);
  console.log(`[encrypt-models] manifest signing pubkey (hex): ${pubRaw.toString('hex')}`);
  console.log('[encrypt-models] make sure src-tauri/src/security/vrm_protect.rs has the same pubkey constant.');
}

main().catch((err) => {
  console.error('[encrypt-models] FAILED:', err.message);
  process.exit(1);
});
