#!/usr/bin/env node
// SPDX-License-Identifier: MIT
//
// scripts/generate-vrm-signing-key.mjs
//
// One-time helper: generates an Ed25519 signing key for the VRM model
// manifest. Writes the PEM-encoded private key to
// scripts/.model-signing-key.pem (gitignored) and prints the raw 32-byte
// public key as a hex string. Paste that hex into the
// MODEL_SIGNING_PUBKEY_HEX constant in
// src-tauri/src/security/vrm_protect.rs so the Rust binary can verify
// signatures produced by encrypt-models.mjs.
//
// Usage:
//   node scripts/generate-vrm-signing-key.mjs
//
// Re-running OVERWRITES the existing key (you must re-encrypt all
// models and bump the public-key constant). Do not run this on a built
// release without coordinating a model + binary rebuild.

import { generateKeyPairSync, createPublicKey } from 'node:crypto';
import { promises as fs } from 'node:fs';
import path from 'node:path';

const KEY_PATH = path.join('scripts', '.model-signing-key.pem');

async function main() {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const pem = privateKey.export({ type: 'pkcs8', format: 'pem' });
  await fs.writeFile(KEY_PATH, pem, { mode: 0o600 });

  const pubDer = publicKey.export({ type: 'spki', format: 'der' });
  const pubRaw = pubDer.subarray(pubDer.length - 32);

  console.log(`[generate-vrm-signing-key] wrote private key: ${KEY_PATH}`);
  console.log(`[generate-vrm-signing-key] PUBLIC KEY HEX (32 bytes):`);
  console.log(pubRaw.toString('hex'));
  console.log('');
  console.log('Next steps:');
  console.log('  1. Paste the hex above into MODEL_SIGNING_PUBKEY_HEX in');
  console.log('     src-tauri/src/security/vrm_protect.rs');
  console.log('  2. Run: node scripts/encrypt-models.mjs');
  console.log('  3. Commit the resulting public/models/default/*.vrm.enc,');
  console.log('     manifest.json, manifest.sig, and key.bin');
  console.log('');
  console.log('NEVER commit scripts/.model-signing-key.pem.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

// Suppress the unused warning — createPublicKey kept to document
// the round-trip path for future maintainers reading this script.
void createPublicKey;
