<template>
  <div class="audio-controls-panel-overlay" @click.stop.self="$emit('close')">
    <div class="audio-controls-panel" @click.stop>
      <div class="panel-header">
        <h3>🎛️ Audio Controls</h3>
        <button class="close-btn" @click="$emit('close')" aria-label="Close">&times;</button>
      </div>

      <div class="panel-body">
        <!-- System Volume -->
        <div class="control-section">
          <div class="section-header">
            <h4>🔊 System Volume</h4>
            <button 
              class="mute-btn" 
              :class="{ muted: systemMuted }"
              @click="toggleSystemMute"
              :title="systemMuted ? 'Unmute' : 'Mute'"
            >
              {{ systemMuted ? '🔇' : '🔊' }}
            </button>
          </div>
          <div class="volume-control">
            <span class="volume-icon">🔈</span>
            <input
              type="range"
              class="volume-slider"
              min="0"
              max="100"
              :value="Math.round(systemVolume * 100)"
              :disabled="systemMuted"
              @input="handleSystemVolumeChange"
            />
            <span class="volume-icon">🔊</span>
            <span class="volume-display">{{ Math.round(systemVolume * 100) }}%</span>
          </div>
        </div>

        <!-- Background Music -->
        <div class="control-section">
          <div class="section-header">
            <h4>🎵 Background Music</h4>
            <button 
              class="mute-btn" 
              :class="{ muted: bgmMuted }"
              @click="toggleBgmMute"
              :title="bgmMuted ? 'Unmute BGM' : 'Mute BGM'"
            >
              {{ bgmMuted ? '🔇' : '🎵' }}
            </button>
          </div>
          <div class="volume-control">
            <span class="volume-icon">🔈</span>
            <input
              type="range"
              class="volume-slider"
              min="0"
              max="100"
              :value="Math.round(bgmVolume * 100)"
              :disabled="bgmMuted"
              @input="handleBgmVolumeChange"
            />
            <span class="volume-icon">🔊</span>
            <span class="volume-display">{{ Math.round(bgmVolume * 100) }}%</span>
          </div>
          <div class="bgm-track-selector">
            <label for="bgm-track-select">Track:</label>
            <select
              id="bgm-track-select"
              class="track-select"
              :value="bgmTrackId"
              @change="handleTrackChange"
            >
              <option v-for="track in bgmTracks" :key="track.id" :value="track.id">
                {{ track.name }}
              </option>
            </select>
          </div>
        </div>

        <!-- Microphone Input -->
        <div class="control-section">
          <div class="section-header">
            <h4>🎤 Microphone</h4>
            <button 
              class="mute-btn" 
              :class="{ muted: micMuted }"
              @click="toggleMicMute"
              :title="micMuted ? 'Unmute Microphone' : 'Mute Microphone'"
            >
              {{ micMuted ? '🎤' : '🎙️' }}
            </button>
          </div>
          <div class="mic-device-selector">
            <label for="mic-device-select">Device:</label>
            <select
              id="mic-device-select"
              class="device-select"
              :value="selectedMicDevice"
              @change="handleMicDeviceChange"
              :disabled="!micDevices.length"
            >
              <option value="">Default System Microphone</option>
              <option v-for="device in micDevices" :key="device.deviceId" :value="device.deviceId">
                {{ device.label || `Microphone ${device.deviceId.slice(0, 8)}...` }}
              </option>
            </select>
          </div>
          <div class="mic-level-display">
            <span class="mic-level-label">Level:</span>
            <div class="mic-level-bar">
              <div 
                class="mic-level-fill" 
                :style="{ width: `${micLevel * 100}%` }"
              ></div>
            </div>
            <span class="mic-level-text">{{ Math.round(micLevel * 100) }}%</span>
          </div>
        </div>

        <!-- Speaker Output -->
        <div class="control-section">
          <div class="section-header">
            <h4>🔈 Speaker Output</h4>
          </div>
          <div class="speaker-device-selector">
            <label for="speaker-device-select">Device:</label>
            <select
              id="speaker-device-select"
              class="device-select"
              :value="selectedSpeakerDevice"
              @change="handleSpeakerDeviceChange"
              :disabled="!speakerDevices.length"
            >
              <option value="">Default System Speaker</option>
              <option v-for="device in speakerDevices" :key="device.deviceId" :value="device.deviceId">
                {{ device.label || `Speaker ${device.deviceId.slice(0, 8)}...` }}
              </option>
            </select>
          </div>
        </div>

        <!-- Audio Test -->
        <div class="control-section">
          <div class="section-header">
            <h4>🧪 Audio Test</h4>
          </div>
          <div class="test-controls">
            <button class="test-btn" @click="testSpeakers" :disabled="testingAudio">
              {{ testingAudio ? '⏸️ Testing...' : '▶️ Test Speakers' }}
            </button>
            <button 
              class="test-btn" 
              @click="testMicrophone" 
              :disabled="testingMic || !selectedMicDevice"
            >
              {{ testingMic ? '⏸️ Recording...' : '🎤 Test Microphone' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { BGM_TRACKS } from '../composables/useBgmPlayer';

// TODO: Implement props usage
// const _props = defineProps<{
//   systemVolume: number;
//   bgmVolume: number;
//   bgmTrackId: string;
// }>();

const emit = defineEmits<{
  close: [];
  'update:systemVolume': [volume: number];
  'update:bgmVolume': [volume: number];
  'update:bgmTrackId': [trackId: string];
  'update:bgmEnabled': [enabled: boolean];
}>();

// Audio state
const systemVolume = ref(100);
const bgmVolume = ref(75);
const bgmTrackId = ref('ambient-calm');
const systemMuted = ref(false);
const bgmMuted = ref(false);
const micMuted = ref(false);
const micLevel = ref(0);
const testingAudio = ref(false);
const testingMic = ref(false);

// Device lists
const micDevices = ref<MediaDeviceInfo[]>([]);
const speakerDevices = ref<MediaDeviceInfo[]>([]);
const selectedMicDevice = ref('');
const selectedSpeakerDevice = ref('');

// BGM tracks
const bgmTracks = BGM_TRACKS;

// Audio monitoring
let audioContext: AudioContext | null = null;
let micStream: MediaStream | null = null;
let micAnalyser: AnalyserNode | null = null;
let micMonitoringInterval: number | null = null;

async function loadAudioDevices() {
  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    micDevices.value = devices.filter(device => device.kind === 'audioinput');
    speakerDevices.value = devices.filter(device => device.kind === 'audiooutput');
  } catch (error) {
    console.warn('Failed to enumerate audio devices:', error);
  }
}

function handleSystemVolumeChange(e: Event) {
  const volume = parseInt((e.target as HTMLInputElement).value) / 100;
  emit('update:systemVolume', volume);
}

function handleBgmVolumeChange(e: Event) {
  const volume = parseInt((e.target as HTMLInputElement).value) / 100;
  emit('update:bgmVolume', volume);
}

function handleTrackChange(e: Event) {
  const trackId = (e.target as HTMLSelectElement).value;
  emit('update:bgmTrackId', trackId);
}

function toggleSystemMute() {
  systemMuted.value = !systemMuted.value;
  if (systemMuted.value) {
    emit('update:systemVolume', 0);
  } else {
    emit('update:systemVolume', 0.8); // Restore to reasonable level
  }
}

function toggleBgmMute() {
  bgmMuted.value = !bgmMuted.value;
  if (bgmMuted.value) {
    emit('update:bgmVolume', 0);
    emit('update:bgmEnabled', false);
  } else {
    emit('update:bgmVolume', 0.15); // Restore to default level
    emit('update:bgmEnabled', true);
  }
}

function toggleMicMute() {
  micMuted.value = !micMuted.value;
  if (micMuted.value) {
    stopMicMonitoring();
  } else {
    startMicMonitoring();
  }
}

async function handleMicDeviceChange(e: Event) {
  const deviceId = (e.target as HTMLSelectElement).value;
  selectedMicDevice.value = deviceId;
  
  // Restart mic monitoring with new device
  if (!micMuted.value) {
    await stopMicMonitoring();
    await startMicMonitoring();
  }
}

function handleSpeakerDeviceChange(e: Event) {
  const deviceId = (e.target as HTMLSelectElement).value;
  selectedSpeakerDevice.value = deviceId;
  // Note: Changing speaker device requires browser API support (setSinkId)
}

async function startMicMonitoring() {
  try {
    const constraints: MediaStreamConstraints = {
      audio: selectedMicDevice.value ? { deviceId: selectedMicDevice.value } : true
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
    
    function updateMicLevel() {
      if (micAnalyser) {
        micAnalyser.getByteFrequencyData(dataArray);
        const average = dataArray.reduce((a, b) => a + b) / dataArray.length;
        micLevel.value = Math.min(1, average / 128);
      }
    }
    
    micMonitoringInterval = window.setInterval(updateMicLevel, 100);
    
  } catch (error) {
    console.warn('Failed to start microphone monitoring:', error);
  }
}

async function stopMicMonitoring() {
  if (micMonitoringInterval) {
    clearInterval(micMonitoringInterval);
    micMonitoringInterval = null;
  }
  
  if (micStream) {
    micStream.getTracks().forEach(track => track.stop());
    micStream = null;
  }
  
  micAnalyser = null;
  micLevel.value = 0;
}

async function testSpeakers() {
  if (testingAudio.value) return;
  
  testingAudio.value = true;
  try {
    const context = audioContext || new AudioContext();
    const oscillator = context.createOscillator();
    const gainNode = context.createGain();
    
    oscillator.connect(gainNode);
    gainNode.connect(context.destination);
    
    oscillator.frequency.value = 440; // A4 note
    gainNode.gain.value = 0.1;
    
    oscillator.start();
    
    setTimeout(() => {
      oscillator.stop();
      testingAudio.value = false;
    }, 1000);
    
  } catch (error) {
    console.warn('Speaker test failed:', error);
    testingAudio.value = false;
  }
}

async function testMicrophone() {
  if (testingMic.value || !selectedMicDevice.value) return;
  
  testingMic.value = true;
  
  try {
    // Record for 3 seconds then play back
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: { deviceId: selectedMicDevice.value }
    });
    
    const mediaRecorder = new MediaRecorder(stream);
    const chunks: BlobPart[] = [];
    
    mediaRecorder.ondataavailable = (e) => {
      chunks.push(e.data);
    };
    
    mediaRecorder.onstop = async () => {
      const blob = new Blob(chunks, { type: 'audio/wav' });
      const audio = new Audio(URL.createObjectURL(blob));
      await audio.play();
      testingMic.value = false;
    };
    
    mediaRecorder.start();
    
    setTimeout(() => {
      mediaRecorder.stop();
      stream.getTracks().forEach(track => track.stop());
    }, 3000);
    
  } catch (error) {
    console.warn('Microphone test failed:', error);
    testingMic.value = false;
  }
}

onMounted(async () => {
  await loadAudioDevices();
  
  // Request microphone permission for device enumeration
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    stream.getTracks().forEach(track => track.stop());
    await loadAudioDevices(); // Reload to get device labels
  } catch {
    // Permission denied, that's ok
  }
  
  // Start mic monitoring if not muted
  if (!micMuted.value) {
    await startMicMonitoring();
  }
});

onUnmounted(() => {
  stopMicMonitoring();
  if (audioContext) {
    audioContext.close();
    audioContext = null;
  }
});
</script>

<style scoped>
.audio-controls-panel-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
}

.audio-controls-panel {
  width: min(520px, 90vw);
  max-height: 85vh;
  background: rgba(15, 20, 35, 0.95);
  border: 1px solid rgba(124, 111, 255, 0.3);
  border-radius: 12px;
  overflow: hidden;
  backdrop-filter: blur(20px);
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.7);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(124, 111, 255, 0.2);
}

.panel-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: #e2e8f0;
}

.close-btn {
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 1.5rem;
  padding: 4px;
  border-radius: 4px;
  transition: color 0.2s ease, background 0.2s ease;
}

.close-btn:hover {
  color: #e2e8f0;
  background: rgba(255, 255, 255, 0.1);
}

.panel-body {
  padding: 20px;
  max-height: calc(85vh - 80px);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.control-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-header h4 {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 600;
  color: #a78bfa;
}

.mute-btn {
  background: none;
  border: 1px solid rgba(124, 111, 255, 0.3);
  border-radius: 8px;
  padding: 8px 12px;
  color: #a78bfa;
  cursor: pointer;
  font-size: 1rem;
  transition: all 0.2s ease;
}

.mute-btn:hover {
  background: rgba(124, 111, 255, 0.1);
  border-color: rgba(124, 111, 255, 0.5);
}

.mute-btn.muted {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.5);
  color: #fca5a5;
}

.volume-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.volume-icon {
  font-size: 0.9rem;
  opacity: 0.7;
}

.volume-slider {
  flex: 1;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 3px;
  outline: none;
  cursor: pointer;
  transition: background 0.2s ease;
}

.volume-slider:hover {
  background: rgba(255, 255, 255, 0.3);
}

.volume-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 18px;
  height: 18px;
  background: #a78bfa;
  border-radius: 50%;
  cursor: pointer;
  transition: background 0.2s ease, transform 0.1s ease;
}

.volume-slider::-webkit-slider-thumb:hover {
  background: #c4b5fd;
  transform: scale(1.1);
}

.volume-slider::-moz-range-thumb {
  width: 18px;
  height: 18px;
  background: #a78bfa;
  border-radius: 50%;
  cursor: pointer;
  border: none;
  transition: background 0.2s ease, transform 0.1s ease;
}

.volume-slider::-moz-range-thumb:hover {
  background: #c4b5fd;
  transform: scale(1.1);
}

.volume-display {
  font-size: 0.8rem;
  color: #94a3b8;
  font-family: 'SF Mono', 'Monaco', 'Cascadia Code', monospace;
  min-width: 40px;
  text-align: right;
}

.bgm-track-selector,
.mic-device-selector,
.speaker-device-selector {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bgm-track-selector label,
.mic-device-selector label,
.speaker-device-selector label {
  font-size: 0.8rem;
  color: #94a3b8;
  min-width: 60px;
}

.track-select,
.device-select {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid rgba(124, 111, 255, 0.3);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.05);
  color: #e2e8f0;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.track-select:hover,
.device-select:hover {
  border-color: rgba(124, 111, 255, 0.5);
  background: rgba(255, 255, 255, 0.08);
}

.track-select:focus,
.device-select:focus {
  outline: none;
  border-color: #a78bfa;
  box-shadow: 0 0 0 2px rgba(124, 111, 255, 0.2);
}

.mic-level-display {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mic-level-label {
  font-size: 0.8rem;
  color: #94a3b8;
  min-width: 60px;
}

.mic-level-bar {
  flex: 1;
  height: 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  overflow: hidden;
}

.mic-level-fill {
  height: 100%;
  background: linear-gradient(90deg, #10b981, #fbbf24, #ef4444);
  border-radius: 4px;
  transition: width 0.1s ease;
}

.mic-level-text {
  font-size: 0.8rem;
  color: #94a3b8;
  font-family: 'SF Mono', 'Monaco', 'Cascadia Code', monospace;
  min-width: 40px;
  text-align: right;
}

.test-controls {
  display: flex;
  gap: 12px;
}

.test-btn {
  flex: 1;
  padding: 10px 16px;
  border: 1px solid rgba(124, 111, 255, 0.3);
  border-radius: 8px;
  background: rgba(124, 111, 255, 0.1);
  color: #a78bfa;
  cursor: pointer;
  font-size: 0.8rem;
  font-weight: 600;
  transition: all 0.2s ease;
}

.test-btn:hover:not(:disabled) {
  background: rgba(124, 111, 255, 0.2);
  border-color: rgba(124, 111, 255, 0.5);
}

.test-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Mobile adjustments */
@media (max-width: 640px) {
  .audio-controls-panel {
    width: 95vw;
    max-height: 90vh;
  }
  
  .panel-header,
  .panel-body {
    padding: 12px 16px;
  }
  
  .volume-control,
  .bgm-track-selector,
  .mic-device-selector,
  .speaker-device-selector,
  .mic-level-display {
    gap: 8px;
  }
  
  .test-controls {
    flex-direction: column;
  }
}
</style>