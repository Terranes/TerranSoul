import { existsSync, readFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const args = new Set(process.argv.slice(2));
const iosConfigPath = path.join(repoRoot, 'src-tauri', 'tauri.ios.conf.json');
const iosInfoPlistPath = path.join(repoRoot, 'src-tauri', 'Info.ios.plist');
const mobileCapabilityPath = path.join(repoRoot, 'src-tauri', 'capabilities', 'mobile.json');
const rustEntryPath = path.join(repoRoot, 'src-tauri', 'src', 'lib.rs');
const appleProjectPath = path.join(repoRoot, 'src-tauri', 'gen', 'apple');

function fail(message) {
  console.error(`[ios-check] ${message}`);
  process.exit(1);
}

function info(message) {
  console.log(`[ios-check] ${message}`);
}

function warn(message) {
  console.warn(`[ios-check] ${message}`);
}

function readJson(filePath) {
  try {
    return JSON.parse(readFileSync(filePath, 'utf8'));
  } catch (error) {
    fail(`failed to parse ${path.relative(repoRoot, filePath)}: ${error.message}`);
  }
}

function commandOk(command, commandArgs) {
  const result = spawnSync(command, commandArgs, {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: false,
  });
  return {
    ok: result.status === 0,
    stdout: result.stdout?.trim() ?? '',
    stderr: result.stderr?.trim() ?? '',
  };
}

if (!existsSync(iosConfigPath)) {
  fail('src-tauri/tauri.ios.conf.json is missing.');
}

const iosConfig = readJson(iosConfigPath);
const minimumSystemVersion = iosConfig.bundle?.iOS?.minimumSystemVersion;
if (typeof minimumSystemVersion !== 'string' || minimumSystemVersion.length === 0) {
  fail('bundle.iOS.minimumSystemVersion must be set in tauri.ios.conf.json.');
}

const mainWindow = iosConfig.app?.windows?.find((windowConfig) => windowConfig.label === 'main');
if (!mainWindow) {
  fail('tauri.ios.conf.json must define the main window override.');
}
if (mainWindow.transparent !== false || mainWindow.disableInputAccessoryView !== true) {
  fail('iOS main window must be opaque and disable the input accessory view.');
}

const rustEntry = readFileSync(rustEntryPath, 'utf8');
if (!rustEntry.includes('tauri::mobile_entry_point')) {
  fail('src-tauri/src/lib.rs must keep #[cfg_attr(mobile, tauri::mobile_entry_point)].');
}
if (!rustEntry.includes('tauri_plugin_barcode_scanner::init()')) {
  fail('src-tauri/src/lib.rs must register tauri-plugin-barcode-scanner.');
}

if (!existsSync(iosInfoPlistPath)) {
  fail('src-tauri/Info.ios.plist is missing; barcode scanning requires NSCameraUsageDescription on iOS.');
}
const iosInfoPlist = readFileSync(iosInfoPlistPath, 'utf8');
if (!iosInfoPlist.includes('NSCameraUsageDescription')) {
  fail('src-tauri/Info.ios.plist must define NSCameraUsageDescription for QR scanning.');
}

if (!existsSync(mobileCapabilityPath)) {
  fail('src-tauri/capabilities/mobile.json is missing.');
}
const mobileCapability = readJson(mobileCapabilityPath);
if (!Array.isArray(mobileCapability.permissions)
  || !mobileCapability.permissions.includes('barcode-scanner:allow-scan')) {
  fail('mobile capability must allow barcode-scanner:allow-scan.');
}

info(`iOS config valid, minimum system version ${minimumSystemVersion}.`);

if (os.platform() !== 'darwin') {
  const mode = args.has('--init') ? 'init' : 'check';
  info(`${mode} skipped on ${os.platform()}; Xcode-based iOS project generation requires macOS.`);
  if (args.has('--require-macos')) {
    fail('--require-macos was set on a non-macOS host.');
  }
  process.exit(0);
}

const tauriVersion = commandOk('npx', ['tauri', '--version']);
if (!tauriVersion.ok) {
  fail(`Tauri CLI is unavailable: ${tauriVersion.stderr || tauriVersion.stdout}`);
}
info(tauriVersion.stdout || 'Tauri CLI available.');

const xcode = commandOk('xcodebuild', ['-version']);
if (!xcode.ok) {
  fail(`xcodebuild is unavailable: ${xcode.stderr || xcode.stdout}`);
}
info(xcode.stdout.split('\n')[0] ?? 'Xcode available.');

const simctl = commandOk('xcrun', ['simctl', 'help']);
if (!simctl.ok) {
  fail(`xcrun simctl is unavailable: ${simctl.stderr || simctl.stdout}`);
}

if (!process.env.APPLE_DEVELOPMENT_TEAM) {
  warn('APPLE_DEVELOPMENT_TEAM is not set; signed device builds will need it or bundle.iOS.developmentTeam.');
}

if (args.has('--init')) {
  const result = spawnSync('npx', ['tauri', 'ios', 'init'], {
    cwd: repoRoot,
    stdio: 'inherit',
    shell: false,
  });
  if (result.status !== 0) {
    fail('npx tauri ios init failed.');
  }
  info('Tauri iOS project initialized.');
  process.exit(0);
}

if (!existsSync(appleProjectPath)) {
  warn('src-tauri/gen/apple is not present yet. Run npm run tauri:ios:init on macOS to generate it.');
}

info('macOS iOS smoke check passed.');