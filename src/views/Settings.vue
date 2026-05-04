<!--
  Settings.vue — sensitivity sliders + gesture on/off toggles.
  Persists to disk via the `update_settings` Tauri command, which forwards
  to settings::save_settings(). Keys on the wire are camelCase; the Rust
  Settings struct uses #[serde(rename_all = "camelCase")] so names match.

  All invoke() calls are guarded with isTauri() so this view renders
  correctly in a plain browser dev session without throwing.
-->
<template>
  <div class="min-h-screen bg-gray-900 text-white p-8">
    <div class="max-w-4xl mx-auto">
      <h1 class="text-3xl font-bold mb-8">Settings</h1>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- Gesture on/off toggles (UI-only state for now; backend doesn't gate on these yet) -->
        <div class="bg-gray-800 rounded-lg p-6">
          <h2 class="text-xl font-semibold mb-4">Gesture Controls</h2>
          <div class="space-y-4">
            <div v-for="gesture in gestures" :key="gesture.id" class="flex items-center justify-between">
              <span class="text-lg">{{ gesture.name }}</span>
              <button
                @click="toggleGesture(gesture.id)"
                :class="gesture.enabled ? 'bg-green-600' : 'bg-gray-700'"
                class="px-4 py-2 rounded-lg transition"
              >
                {{ gesture.enabled ? 'On' : 'Off' }}
              </button>
            </div>
          </div>
        </div>

        <!-- Sensitivity sliders. v-model.number coerces range strings to numbers. -->
        <div class="bg-gray-800 rounded-lg p-6">
          <h2 class="text-xl font-semibold mb-4">Sensitivity</h2>
          <div class="space-y-6">
            <div>
              <label class="block text-sm font-medium mb-2">Scroll Speed: {{ settings.scrollSensitivity }}</label>
              <input
                v-model.number="settings.scrollSensitivity"
                type="range" min="0.1" max="5" step="0.1"
                class="w-full accent-blue-600"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-2">Swipe Threshold: {{ settings.swipeThreshold }}</label>
              <input
                v-model.number="settings.swipeThreshold"
                type="range" min="10" max="200" step="5"
                class="w-full accent-blue-600"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-2">Pinch Threshold: {{ settings.pinchThreshold }}</label>
              <input
                v-model.number="settings.pinchThreshold"
                type="range" min="10" max="100" step="5"
                class="w-full accent-blue-600"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-2">Zoom Threshold: {{ settings.zoomThreshold }}</label>
              <input
                v-model.number="settings.zoomThreshold"
                type="range" min="5" max="100" step="5"
                class="w-full accent-blue-600"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-2">Zoom Time Window (s): {{ settings.zoomTimeWindow }}</label>
              <input
                v-model.number="settings.zoomTimeWindow"
                type="range" min="0.1" max="2" step="0.1"
                class="w-full accent-blue-600"
              />
            </div>
          </div>
        </div>
      </div>

      <div class="mt-8 flex gap-4">
        <button
          @click="saveSettings"
          class="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg transition"
        >
          Save Settings
        </button>
        <button
          @click="resetSettings"
          class="px-6 py-3 bg-gray-700 hover:bg-gray-600 rounded-lg transition"
        >
          Reset to Defaults
        </button>
      </div>

      <p v-if="message" :class="isError ? 'text-red-400' : 'text-green-400'" class="mt-4">{{ message }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'

interface GestureConfig {
  id: string
  name: string
  enabled: boolean
}

// Mirror of the Rust `Settings` struct on the wire (camelCase keys).
// Fields here must match the serde-renamed names in settings/mod.rs.
interface PersistedSettings {
  trackingEnabled: boolean
  scrollSensitivity: number
  swipeThreshold: number
  pinchThreshold: number
  zoomThreshold: number
  zoomTimeWindow: number
}

function defaultSettings(): PersistedSettings {
  return {
    trackingEnabled: false,
    scrollSensitivity: 1.0,
    swipeThreshold: 50,
    pinchThreshold: 30,
    zoomThreshold: 25,
    zoomTimeWindow: 0.5,
  }
}

const settings = reactive<PersistedSettings>(defaultSettings())

// Gesture toggles are currently UI-only — the backend gesture tracker doesn't
// consult them yet. Kept so users can see what's planned.
const gestures = reactive<GestureConfig[]>([
  { id: 'leftClick', name: 'Left Click (Pinch)', enabled: true },
  { id: 'rightClick', name: 'Right Click', enabled: true },
  { id: 'scrollUp', name: 'Scroll Up', enabled: true },
  { id: 'scrollDown', name: 'Scroll Down', enabled: true },
  { id: 'scrollLeft', name: 'Scroll Left', enabled: false },
  { id: 'scrollRight', name: 'Scroll Right', enabled: false },
  { id: 'swipeUp', name: 'Swipe Up', enabled: true },
  { id: 'swipeDown', name: 'Swipe Down', enabled: true },
  { id: 'swipeLeft', name: 'Swipe Left', enabled: false },
  { id: 'swipeRight', name: 'Swipe Right', enabled: false },
  { id: 'zoomIn', name: 'Zoom In', enabled: true },
  { id: 'zoomOut', name: 'Zoom Out', enabled: true },
  { id: 'appSwitch', name: 'App Switch', enabled: true },
  { id: 'screenshot', name: 'Screenshot', enabled: true },
])

const message = ref('')
const isError = ref(false)

function applyLoaded(loaded: Partial<PersistedSettings>) {
  // Only copy keys we know about — defends against future schema drift.
  const keys: (keyof PersistedSettings)[] = [
    'trackingEnabled', 'scrollSensitivity', 'swipeThreshold',
    'pinchThreshold', 'zoomThreshold', 'zoomTimeWindow',
  ]
  for (const k of keys) {
    if (loaded[k] !== undefined) {
      ;(settings as any)[k] = loaded[k]
    }
  }
}

async function loadSettings() {
  // guard: invoke throws if window.__TAURI_INTERNALS__ is undefined (plain browser)
  if (!isTauri()) return

  try {
    // get_settings returns a JSON string (serialized by serde_json::to_string).
    const raw = await invoke<string>('get_settings')
    const parsed = JSON.parse(raw) as Partial<PersistedSettings>
    applyLoaded(parsed)
  } catch (e) {
    console.error('[Settings] Failed to load settings:', e)
  }
}

async function saveSettings() {
  if (!isTauri()) {
    isError.value = false
    message.value = 'Run the app via "npm run tauri:dev" to save settings.'
    setTimeout(() => (message.value = ''), 3000)
    return
  }
  try {
    // Snapshot the reactive object; Tauri v2 command parameter names are
    // camelCase (Rust snake_case `settings_json` → wire name `settingsJson`).
    const payload = JSON.stringify({ ...settings })
    await invoke<string>('update_settings', { settingsJson: payload })
    isError.value = false
    message.value = 'Settings saved!'
    setTimeout(() => (message.value = ''), 3000)
  } catch (e: any) {
    isError.value = true
    message.value = 'Error: ' + (e?.message || e)
  }
}

function resetSettings() {
  Object.assign(settings, defaultSettings())
  for (const g of gestures) g.enabled = true
}

function toggleGesture(id: string) {
  const gesture = gestures.find((g) => g.id === id)
  if (gesture) gesture.enabled = !gesture.enabled
}

onMounted(() => {
  loadSettings()
})
</script>
