/**
 * Per-model camera position store.
 *
 * Persists camera orbit (azimuth + distance) independently for each 3D model
 * via Tauri IPC, so the user's preferred viewing angle is restored on restart.
 */

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface ModelCameraPosition {
  azimuth: number;
  distance: number;
}

// ── State ─────────────────────────────────────────────────────────────────────

const positions = ref<Map<string, ModelCameraPosition>>(new Map());

// ── Composable ────────────────────────────────────────────────────────────────

export function useModelCameraStore() {
  /** Load all per-model camera positions from the backend. */
  async function loadAll(): Promise<void> {
    try {
      const data = await invoke<Record<string, ModelCameraPosition>>(
        'get_model_camera_positions',
      );
      const map = new Map<string, ModelCameraPosition>();
      for (const [modelId, pos] of Object.entries(data)) {
        map.set(modelId, pos);
      }
      positions.value = map;
    } catch {
      // Tauri unavailable — keep whatever is in memory
    }
  }

  /** Get the saved camera position for a model, or null if none exists. */
  function getCameraForModel(modelId: string): ModelCameraPosition | null {
    return positions.value.get(modelId) ?? null;
  }

  /** Save a camera position for a specific model and persist via Tauri IPC. */
  async function saveCameraForModel(
    modelId: string,
    azimuth: number,
    distance: number,
  ): Promise<void> {
    const pos: ModelCameraPosition = { azimuth, distance };
    const updated = new Map(positions.value);
    updated.set(modelId, pos);
    positions.value = updated;
    try {
      await invoke('save_model_camera_position', {
        modelId,
        azimuth,
        distance,
      });
    } catch {
      // Tauri unavailable — position lives in memory only
    }
  }

  return {
    positions,
    loadAll,
    getCameraForModel,
    saveCameraForModel,
  };
}
