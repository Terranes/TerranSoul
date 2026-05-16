<template>
  <PanelShell
    variant="overlay-fixed"
    title="🎛️ Audio Controls"
    test-id="audio-controls-panel"
    card-class="audio-controls-panel__card"
    :on-close="handleClose"
    @close="handleClose"
  >
    <!-- ── System Volume ─────────────────────────────────────────────── -->
    <section
      class="ac-section"
      data-testid="audio-system-section"
    >
      <header class="ac-section-head">
        <h4>🔊 System Volume</h4>
        <button
          class="ac-icon-btn"
          :class="{ 'ac-icon-btn--muted': systemMuted }"
          :title="systemMuted ? 'Unmute' : 'Mute'"
          type="button"
          data-testid="audio-system-mute"
          @click="toggleSystemMute"
        >
          {{ systemMuted ? '🔇' : '🔊' }}
        </button>
      </header>
      <div class="ac-slider-row">
        <span class="ac-slider-icon">🔈</span>
        <input
          type="range"
          class="ac-slider"
          min="0"
          max="100"
          :value="Math.round(systemVolume * 100)"
          :disabled="systemMuted"
          aria-label="System volume"
          @input="handleSystemVolumeChange"
        >
        <span class="ac-slider-icon">🔊</span>
        <span class="ac-slider-value">{{ Math.round(systemVolume * 100) }}%</span>
      </div>
    </section>

    <!-- ── Background Music ─────────────────────────────────────────── -->
    <section
      class="ac-section"
      data-testid="audio-bgm-section"
    >
      <header class="ac-section-head">
        <h4>🎵 Background Music</h4>
        <button
          class="ac-icon-btn"
          :class="{ 'ac-icon-btn--muted': bgmMuted }"
          :title="bgmMuted ? 'Unmute BGM' : 'Mute BGM'"
          type="button"
          data-testid="audio-bgm-mute"
          @click="toggleBgmMute"
        >
          {{ bgmMuted ? '🔇' : '🎵' }}
        </button>
      </header>
      <div class="ac-slider-row">
        <span class="ac-slider-icon">🔈</span>
        <input
          type="range"
          class="ac-slider"
          min="0"
          max="100"
          :value="Math.round(bgmVolume * 100)"
          :disabled="bgmMuted"
          aria-label="Background music volume"
          @input="handleBgmVolumeChange"
        >
        <span class="ac-slider-icon">🔊</span>
        <span class="ac-slider-value">{{ Math.round(bgmVolume * 100) }}%</span>
      </div>
      <label
        for="ac-bgm-track"
        class="ac-field"
      >
        <span class="ac-field-label">Track</span>
        <select
          id="ac-bgm-track"
          class="ac-select"
          :value="bgmTrackId"
          data-testid="audio-bgm-track"
          @change="handleTrackChange"
        >
          <option
            v-for="track in bgmTracks"
            :key="track.id"
            :value="track.id"
          >
            {{ track.name }}
          </option>
        </select>
      </label>
    </section>

    <!-- ── Microphone Input ─────────────────────────────────────────── -->
    <section
      class="ac-section"
      data-testid="audio-mic-section"
    >
      <header class="ac-section-head">
        <h4>🎤 Microphone</h4>
        <button
          class="ac-icon-btn"
          :class="{ 'ac-icon-btn--muted': micMuted }"
          :title="micMuted ? 'Resume monitoring' : 'Pause monitoring'"
          type="button"
          data-testid="audio-mic-mute"
          @click="toggleMicMute"
        >
          {{ micMuted ? '🎤' : '🎙️' }}
        </button>
      </header>
      <label
        for="ac-mic-device"
        class="ac-field"
      >
        <span class="ac-field-label">Device</span>
        <select
          id="ac-mic-device"
          class="ac-select"
          :value="selectedMicDevice"
          :disabled="!micDevices.length"
          data-testid="audio-mic-device"
          @change="handleMicDeviceChange"
        >
          <option value="">
            Default System Microphone
          </option>
          <option
            v-for="device in micDevices"
            :key="device.deviceId"
            :value="device.deviceId"
          >
            {{ device.label || `Microphone ${device.deviceId.slice(0, 8)}…` }}
          </option>
        </select>
      </label>
      <div class="ac-meter-row">
        <span class="ac-meter-label">Level</span>
        <div
          class="ac-meter-track"
          role="meter"
          :aria-valuenow="Math.round(micLevel * 100)"
          aria-valuemin="0"
          aria-valuemax="100"
        >
          <div
            class="ac-meter-fill"
            :style="{ width: `${micLevel * 100}%` }"
          />
        </div>
        <span class="ac-meter-value">{{ Math.round(micLevel * 100) }}%</span>
      </div>
      <button
        class="ac-test-btn"
        :disabled="testingMic"
        type="button"
        data-testid="audio-mic-test"
        @click="testMicrophone"
      >
        {{ testingMic ? '⏺ Recording…' : '🎤 Test microphone (3s record + playback)' }}
      </button>
    </section>

    <!-- ── Speaker Output ───────────────────────────────────────────── -->
    <section
      class="ac-section"
      data-testid="audio-speaker-section"
    >
      <header class="ac-section-head">
        <h4>🔈 Speaker Output</h4>
      </header>
      <label
        for="ac-speaker-device"
        class="ac-field"
      >
        <span class="ac-field-label">Device</span>
        <select
          id="ac-speaker-device"
          class="ac-select"
          :value="selectedSpeakerDevice"
          :disabled="!speakerDevices.length"
          data-testid="audio-speaker-device"
          @change="handleSpeakerDeviceChange"
        >
          <option value="">
            Default System Speaker
          </option>
          <option
            v-for="device in speakerDevices"
            :key="device.deviceId"
            :value="device.deviceId"
          >
            {{ device.label || `Speaker ${device.deviceId.slice(0, 8)}…` }}
          </option>
        </select>
      </label>
      <p
        v-if="!supportsSinkId"
        class="ac-hint"
      >
        ⚠️ Your browser cannot route to a specific output device. The system
        default will be used.
      </p>
      <button
        class="ac-test-btn"
        :disabled="testingSpeakers"
        type="button"
        data-testid="audio-speaker-test"
        @click="testSpeakers"
      >
        {{ testingSpeakers ? '▶️ Playing…' : '▶️ Test speakers (440 Hz tone)' }}
      </button>
    </section>

    <!-- ── Voice (TTS + ASR) link-out ───────────────────────────────── -->
    <section
      class="ac-section ac-section--link"
      data-testid="audio-voice-link-section"
    >
      <header class="ac-section-head">
        <h4>🗣️ Voice (TTS + ASR)</h4>
      </header>
      <p class="ac-hint">
        Voice provider selection, per-provider 🔊 Test buttons, and API keys
        live in the dedicated Voice Setup view.
      </p>
      <button
        class="ac-link-btn"
        type="button"
        data-testid="audio-open-voice-setup"
        @click="openVoiceSetup"
      >
        Configure voice providers →
      </button>
    </section>
  </PanelShell>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { BGM_TRACKS } from '../composables/useBgmPlayer';
import PanelShell from './ui/PanelShell.vue';

/*
 * AudioControlsPanel — system-level audio controls (volume, BGM, mic/speaker
 * device pickers + test buttons). Voice provider selection (TTS/ASR) lives
 * in `src/views/VoiceSetupView.vue`; this panel surfaces a link-out button
 * via the `navigate` emit so the user has one canonical place for provider
 * config.
 *
 * Migrated to `PanelShell` chrome (UI-AUDIO-PANEL-1) so it matches every
 * other overlay panel (SystemInfoPanel, ModelPanel) in close-button
 * placement, backdrop blur, and keyboard semantics.
 */

const emit = defineEmits<{
  close: [];
  navigate: [target: string];
  'update:systemVolume': [volume: number];
  'update:bgmVolume': [volume: number];
  'update:bgmTrackId': [trackId: string];
  'update:bgmEnabled': [enabled: boolean];
}>();

// ── Audio state (local refs; parent applies updates via emitted events) ──
const systemVolume = ref(1.0);
const bgmVolume = ref(0.15);
const bgmTrackId = ref('prelude');
const systemMuted = ref(false);
const bgmMuted = ref(false);
const micMuted = ref(false);
const micLevel = ref(0);
const testingSpeakers = ref(false);
const testingMic = ref(false);

const micDevices = ref<MediaDeviceInfo[]>([]);
const speakerDevices = ref<MediaDeviceInfo[]>([]);
const selectedMicDevice = ref('');
const selectedSpeakerDevice = ref('');

const bgmTracks = BGM_TRACKS;

const supportsSinkId = computed(() => {
  if (typeof HTMLAudioElement === 'undefined') return false;
  return 'setSinkId' in HTMLAudioElement.prototype;
});

// ── Audio monitoring plumbing ──────────────────────────────────────────
let audioContext: AudioContext | null = null;
let micStream: MediaStream | null = null;
let micAnalyser: AnalyserNode | null = null;
let micMonitoringInterval: number | null = null;

async function loadAudioDevices() {
  if (typeof navigator === 'undefined' || !navigator.mediaDevices?.enumerateDevices) {
    return;
  }
  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    micDevices.value = devices.filter((d) => d.kind === 'audioinput');
    speakerDevices.value = devices.filter((d) => d.kind === 'audiooutput');
  } catch (error) {
    console.warn('Failed to enumerate audio devices:', error);
  }
}

function handleSystemVolumeChange(e: Event) {
  const volume = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  systemVolume.value = volume;
  emit('update:systemVolume', volume);
}

function handleBgmVolumeChange(e: Event) {
  const volume = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  bgmVolume.value = volume;
  emit('update:bgmVolume', volume);
}

function handleTrackChange(e: Event) {
  const trackId = (e.target as HTMLSelectElement).value;
  bgmTrackId.value = trackId;
  emit('update:bgmTrackId', trackId);
}

function toggleSystemMute() {
  systemMuted.value = !systemMuted.value;
  emit('update:systemVolume', systemMuted.value ? 0 : Math.max(systemVolume.value, 0.8));
}

function toggleBgmMute() {
  bgmMuted.value = !bgmMuted.value;
  if (bgmMuted.value) {
    emit('update:bgmVolume', 0);
    emit('update:bgmEnabled', false);
  } else {
    emit('update:bgmVolume', Math.max(bgmVolume.value, 0.15));
    emit('update:bgmEnabled', true);
  }
}

function toggleMicMute() {
  micMuted.value = !micMuted.value;
  if (micMuted.value) {
    void stopMicMonitoring();
  } else {
    void startMicMonitoring();
  }
}

async function handleMicDeviceChange(e: Event) {
  const deviceId = (e.target as HTMLSelectElement).value;
  selectedMicDevice.value = deviceId;
  if (!micMuted.value) {
    await stopMicMonitoring();
    await startMicMonitoring();
  }
}

function handleSpeakerDeviceChange(e: Event) {
  const deviceId = (e.target as HTMLSelectElement).value;
  selectedSpeakerDevice.value = deviceId;
  // Actually routing audio to the chosen sink requires `audioElement.setSinkId`,
  // which is gated on Chromium and a few other engines. We surface the hint
  // via `supportsSinkId` instead of silently failing.
}

async function startMicMonitoring() {
  if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
    return;
  }
  try {
    const constraints: MediaStreamConstraints = {
      audio: selectedMicDevice.value ? { deviceId: selectedMicDevice.value } : true,
    };
    micStream = await navigator.mediaDevices.getUserMedia(constraints);

    if (!audioContext) {
      audioContext = new AudioContext();
    }

    micAnalyser = audioContext.createAnalyser();
    micAnalyser.fftSize = 2048;

    const source = audioContext.createMediaStreamSource(micStream);
    source.connect(micAnalyser);

    const dataArray = new Uint8Array(micAnalyser.frequencyBinCount);

    micMonitoringInterval = window.setInterval(() => {
      if (!micAnalyser) return;
      micAnalyser.getByteFrequencyData(dataArray);
      let sum = 0;
      for (let i = 0; i < dataArray.length; i++) sum += dataArray[i];
      const average = sum / dataArray.length;
      micLevel.value = Math.min(1, average / 128);
    }, 100);
  } catch (error) {
    console.warn('Failed to start microphone monitoring:', error);
  }
}

async function stopMicMonitoring() {
  if (micMonitoringInterval !== null) {
    clearInterval(micMonitoringInterval);
    micMonitoringInterval = null;
  }
  if (micStream) {
    micStream.getTracks().forEach((track) => track.stop());
    micStream = null;
  }
  micAnalyser = null;
  micLevel.value = 0;
}

async function testSpeakers() {
  if (testingSpeakers.value) return;
  testingSpeakers.value = true;
  try {
    const context = audioContext ?? new AudioContext();
    audioContext = context;
    const oscillator = context.createOscillator();
    const gainNode = context.createGain();
    oscillator.connect(gainNode);
    gainNode.connect(context.destination);
    oscillator.frequency.value = 440;
    gainNode.gain.value = 0.1;
    oscillator.start();
    setTimeout(() => {
      oscillator.stop();
      testingSpeakers.value = false;
    }, 1000);
  } catch (error) {
    console.warn('Speaker test failed:', error);
    testingSpeakers.value = false;
  }
}

async function testMicrophone() {
  if (testingMic.value) return;
  if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) return;
  testingMic.value = true;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: selectedMicDevice.value ? { deviceId: selectedMicDevice.value } : true,
    });
    const mediaRecorder = new MediaRecorder(stream);
    const chunks: BlobPart[] = [];

    mediaRecorder.ondataavailable = (e) => {
      chunks.push(e.data);
    };

    mediaRecorder.onstop = async () => {
      const blob = new Blob(chunks, { type: 'audio/webm' });
      const audio = new Audio(URL.createObjectURL(blob));
      try {
        await audio.play();
      } catch (err) {
        console.warn('Playback failed:', err);
      }
      testingMic.value = false;
    };

    mediaRecorder.start();
    setTimeout(() => {
      mediaRecorder.stop();
      stream.getTracks().forEach((track) => track.stop());
    }, 3000);
  } catch (error) {
    console.warn('Microphone test failed:', error);
    testingMic.value = false;
  }
}

function openVoiceSetup() {
  emit('navigate', 'voice');
  emit('close');
}

function handleClose() {
  emit('close');
}

onMounted(async () => {
  await loadAudioDevices();
  // Request mic permission once to populate device labels (browsers hide
  // labels until permission is granted). Failure is non-fatal.
  if (typeof navigator !== 'undefined' && navigator.mediaDevices?.getUserMedia) {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      stream.getTracks().forEach((t) => t.stop());
      await loadAudioDevices();
    } catch {
      // permission denied — keep going with anonymous device IDs
    }
  }
  if (!micMuted.value) {
    await startMicMonitoring();
  }
});

onUnmounted(() => {
  void stopMicMonitoring();
  if (audioContext) {
    void audioContext.close();
    audioContext = null;
  }
});
</script>

<style scoped>
:deep(.audio-controls-panel__card) {
  width: min(560px, 100%);
  max-height: min(86vh, calc(100dvh - 32px));
}

.ac-section {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm);
  padding: var(--ts-space-md) var(--ts-space-lg);
  border-bottom: 1px solid var(--ts-border);
}

.ac-section:last-child {
  border-bottom: none;
}

.ac-section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-sm);
}

.ac-section-head h4 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ts-text-primary);
}

.ac-icon-btn {
  appearance: none;
  background: transparent;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  color: var(--ts-text-primary);
  font-size: 1rem;
  line-height: 1;
  padding: 4px 10px;
  cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast);
}

.ac-icon-btn:hover {
  background: var(--ts-bg-hover);
  border-color: var(--ts-accent);
}

.ac-icon-btn--muted {
  background: color-mix(in srgb, var(--ts-accent-warm, #e09b3d) 18%, transparent);
  border-color: var(--ts-accent-warm, #e09b3d);
}

.ac-slider-row {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.ac-slider {
  flex: 1 1 auto;
  min-width: 0;
}

.ac-slider-icon {
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
}

.ac-slider-value {
  font-variant-numeric: tabular-nums;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  min-width: 3em;
  text-align: right;
}

.ac-field {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.ac-field-label {
  flex: 0 0 auto;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  min-width: 4em;
}

.ac-select {
  flex: 1 1 auto;
  min-width: 0;
  appearance: none;
  background: var(--ts-bg-elevated);
  color: var(--ts-text-primary);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  padding: 6px 10px;
  font-size: 0.9rem;
  cursor: pointer;
}

.ac-select:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.ac-meter-row {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.ac-meter-label {
  flex: 0 0 auto;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  min-width: 4em;
}

.ac-meter-track {
  flex: 1 1 auto;
  height: 8px;
  background: var(--ts-bg-elevated);
  border-radius: var(--ts-radius-pill);
  overflow: hidden;
  border: 1px solid var(--ts-border);
}

.ac-meter-fill {
  height: 100%;
  background: linear-gradient(
    to right,
    var(--ts-accent) 0%,
    var(--ts-accent-warm, #e09b3d) 100%
  );
  transition: width 80ms linear;
}

.ac-meter-value {
  font-variant-numeric: tabular-nums;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  min-width: 3em;
  text-align: right;
}

.ac-test-btn,
.ac-link-btn {
  appearance: none;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border);
  color: var(--ts-text-primary);
  border-radius: var(--ts-radius-md);
  padding: 8px 14px;
  font-size: 0.9rem;
  cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast);
  text-align: center;
}

.ac-test-btn:hover:not(:disabled),
.ac-link-btn:hover {
  background: var(--ts-bg-hover);
  border-color: var(--ts-accent);
  color: var(--ts-accent);
}

.ac-test-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.ac-link-btn {
  font-weight: 600;
}

.ac-hint {
  margin: 0;
  font-size: 0.8rem;
  line-height: 1.4;
  color: var(--ts-text-secondary);
}

.ac-section--link {
  background: color-mix(in srgb, var(--ts-accent) 6%, transparent);
}

@media (max-width: 640px) {
  .ac-section {
    padding: var(--ts-space-sm) var(--ts-space-md);
  }
  .ac-field,
  .ac-meter-row {
    flex-wrap: wrap;
  }
  .ac-field-label,
  .ac-meter-label {
    flex: 1 1 100%;
  }
}
</style>
