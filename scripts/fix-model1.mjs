/**
 * Fix Model1.vrm — patches the GLB JSON to correct the humanoid bone mapping.
 * 
 * The VRM 1.x bone assignments are shifted by one level in the skeleton:
 *   - head was pointing to NeckTwist01 instead of Head
 *   - leftHand was pointing to L_Forearm instead of L_Hand
 *   - etc.
 * 
 * This script fixes the mapping without touching root transforms or bind matrices.
 */
import { readFileSync, writeFileSync } from 'fs';

const INPUT = 'public/models/default/Model1.vrm';
const OUTPUT = 'public/models/default/Model1.vrm';

const buf = readFileSync(INPUT);

// Parse GLB header
const version = buf.readUInt32LE(4);

// Chunk 0 (JSON)
const chunk0Len = buf.readUInt32LE(12);
const jsonStr = buf.toString('utf-8', 20, 20 + chunk0Len);
const json = JSON.parse(jsonStr);

// Chunk 1 (binary)
const chunk1Offset = 20 + chunk0Len;
const chunk1Len = buf.readUInt32LE(chunk1Offset);
const binChunk = buf.subarray(chunk1Offset + 8, chunk1Offset + 8 + chunk1Len);

// ──────────────────────────────────────────────────
// Fix: Correct humanoid bone mapping
// Node index → name:
//   0: L_Foot, 3: L_Calf, 6: L_Thigh
//   9: R_Foot, 12: R_Calf, 13: R_Thigh
//   14: Pelvis, 15: Head, 17: NeckTwist01
//   20: L_Hand, 21: L_Forearm, 24: L_Upperarm, 25: L_Clavicle
//   30: R_Hand, 31: R_Forearm, 32: R_Upperarm, 33: R_Clavicle
//   34: Spine02, 35: Spine01, 36: Waist, 37: Hip
// ──────────────────────────────────────────────────
const correctBones = {
  hips:          { node: 37 },  // Hip
  spine:         { node: 36 },  // Waist
  chest:         { node: 35 },  // Spine01
  upperChest:    { node: 34 },  // Spine02
  neck:          { node: 17 },  // NeckTwist01
  head:          { node: 15 },  // Head
  leftShoulder:  { node: 25 },  // L_Clavicle
  leftUpperArm:  { node: 24 },  // L_Upperarm
  leftLowerArm:  { node: 21 },  // L_Forearm
  leftHand:      { node: 20 },  // L_Hand
  rightShoulder: { node: 33 },  // R_Clavicle
  rightUpperArm: { node: 32 },  // R_Upperarm
  rightLowerArm: { node: 31 },  // R_Forearm
  rightHand:     { node: 30 },  // R_Hand
  leftUpperLeg:  { node: 6  },  // L_Thigh
  leftLowerLeg:  { node: 3  },  // L_Calf
  leftFoot:      { node: 0  },  // L_Foot
  rightUpperLeg: { node: 13 },  // R_Thigh
  rightLowerLeg: { node: 12 },  // R_Calf
  rightFoot:     { node: 9  },  // R_Foot
};

const vrmExt = json.extensions.VRMC_vrm;
const oldBones = vrmExt.humanoid.humanBones;
console.log('Old bone mapping:');
for (const [name, info] of Object.entries(oldBones)) {
  console.log(`  ${name} -> [${info.node}] ${json.nodes[info.node].name}`);
}

vrmExt.humanoid.humanBones = correctBones;

console.log('\nNew bone mapping:');
for (const [name, info] of Object.entries(correctBones)) {
  console.log(`  ${name} -> [${info.node}] ${json.nodes[info.node].name}`);
}

// ──────────────────────────────────────────────────
// Rebuild GLB
// ──────────────────────────────────────────────────
let newJsonStr = JSON.stringify(json);
while (newJsonStr.length % 4 !== 0) newJsonStr += ' ';
const newJsonBuf = Buffer.from(newJsonStr, 'utf-8');

const totalLen = 12 + 8 + newJsonBuf.length + 8 + binChunk.length;
const out = Buffer.alloc(totalLen);
let offset = 0;

out.write('glTF', offset); offset += 4;
out.writeUInt32LE(version, offset); offset += 4;
out.writeUInt32LE(totalLen, offset); offset += 4;

out.writeUInt32LE(newJsonBuf.length, offset); offset += 4;
out.writeUInt32LE(0x4E4F534A, offset); offset += 4;
newJsonBuf.copy(out, offset); offset += newJsonBuf.length;

out.writeUInt32LE(binChunk.length, offset); offset += 4;
out.writeUInt32LE(0x004E4942, offset); offset += 4;
binChunk.copy(out, offset); offset += binChunk.length;

writeFileSync(OUTPUT, out);
console.log('\nWrote fixed VRM:', OUTPUT, '(' + (out.length / 1024 / 1024).toFixed(1) + ' MB)');
