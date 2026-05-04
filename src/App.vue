<!--
  App.vue — root shell.
  Hosts the tab navigation and the Start / Stop Tracking control.
  Subscribes to:
    - "tracking-status"  (state, message, hint, gesture?, confidence?)
    - "gesture-detected" (gesture name string) — for the gesture log
-->
<template>
  <div class="min-h-screen bg-gray-900 text-white">
    <!-- Top navigation bar -->
    <nav class="bg-gray-800 border-b border-gray-700">
      <div class="max-w-6xl mx-auto px-4 py-3 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center text-sm font-bold">SC</div>
          <h1 class="text-xl font-bold">ScreenCursor</h1>
        </div>
        <div class="flex gap-2">
          <button
            v-for="view in views"
            :key="view.id"
            @click="currentView = view.id"
            :class="currentView === view.id ? 'bg-blue-600 text-white' : 'bg-gray-700 hover:bg-gray-600 text-gray-300'"
            class="px-4 py-2 rounded-lg transition text-sm font-medium"
          >{{ view.label }}</button>
        </div>
      </div>
    </nav>

    <!-- Browser-mode banner -->
    <div v-if="!isInsideTauri" class="bg-amber-700/80 text-amber-100 text-sm text-center py-2 px-4 border-b border-amber-600">
      ⚠️ Browser mode — Tauri backend unavailable. Run <code class="bg-amber-800 px-1 rounded">npm run tauri:dev</code> for full functionality.
    </div>

    <!-- ═══════════════ MAIN VIEW ═══════════════ -->
    <div v-if="currentView === 'main'" class="max-w-2xl mx-auto px-4 py-10 space-y-6">

      <!-- Start / Stop button -->
      <div class="text-center">
        <p class="text-gray-400 mb-6 text-sm">Hand gesture mouse control via computer vision</p>
        <button
          @click="toggleTracking"
          :class="tracking
            ? 'bg-red-600 hover:bg-red-700 ring-red-500'
            : 'bg-blue-600 hover:bg-blue-700 ring-blue-500'"
          class="px-10 py-4 text-lg font-semibold rounded-xl transition-all duration-200 ring-2 ring-offset-2 ring-offset-gray-900 active:scale-95"
        >
          <span v-if="!tracking">▶ Start Tracking</span>
          <span v-else class="flex items-center gap-2 justify-center">
            <span class="w-2 h-2 rounded-full bg-red-300 animate-pulse inline-block"></span>
            Stop Tracking
          </span>
        </button>
        <p v-if="toggleMessage" :class="toggleError ? 'text-red-400' : 'text-green-400'" class="mt-3 text-sm">
          {{ toggleMessage }}
        </p>
      </div>

      <!-- ── Status banner (only while tracking) ── -->
      <transition name="fade">
        <div v-if="tracking" :class="statusBannerClass" class="rounded-xl p-4 flex items-start gap-4 transition-colors duration-300">
          <div class="text-2xl mt-0.5 flex-shrink-0">{{ statusIcon }}</div>
          <div class="flex-1 min-w-0">
            <p class="font-semibold text-sm">{{ trackingStatus.message || 'Initialising…' }}</p>
            <p class="text-xs mt-0.5 opacity-80">{{ trackingStatus.hint }}</p>
            <div v-if="trackingStatus.confidence != null" class="mt-2">
              <div class="flex items-center gap-2">
                <span class="text-xs opacity-60">Confidence</span>
                <div class="flex-1 h-1.5 bg-black/30 rounded-full overflow-hidden">
                  <div
                    class="h-full rounded-full transition-all duration-300"
                    :class="trackingStatus.confidence > 0.7 ? 'bg-green-400' : 'bg-yellow-400'"
                    :style="{ width: (trackingStatus.confidence * 100).toFixed(0) + '%' }"
                  ></div>
                </div>
                <span class="text-xs opacity-60 tabular-nums">{{ (trackingStatus.confidence * 100).toFixed(0) }}%</span>
              </div>
            </div>
          </div>
        </div>
      </transition>

      <!-- ── Gesture log (only while tracking) ── -->
      <transition name="fade">
        <div v-if="tracking && gestureLog.length > 0" class="bg-gray-800 rounded-xl p-4">
          <h2 class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">Recent Gestures</h2>
          <ul class="space-y-1.5">
            <li
              v-for="(entry, i) in gestureLog"
              :key="entry.id"
              :class="i === 0 ? 'opacity-100' : i === 1 ? 'opacity-70' : i === 2 ? 'opacity-50' : 'opacity-30'"
              class="flex items-center gap-3 text-sm transition-opacity"
            >
              <span class="text-lg flex-shrink-0">{{ gestureIcon(entry.name) }}</span>
              <span class="font-medium flex-1">{{ entry.name }}</span>
              <span class="text-xs text-gray-500 tabular-nums">{{ entry.time }}</span>
            </li>
          </ul>
        </div>
      </transition>

      <!-- ── Idle hint (not tracking) ── -->
      <div v-if="!tracking" class="bg-gray-800/60 rounded-xl p-5 text-sm text-gray-400 space-y-2 border border-gray-700/50">
        <p class="font-semibold text-gray-300">Quick-start guide</p>
        <ul class="list-disc list-inside space-y-1">
          <li>Click <strong class="text-white">Start Tracking</strong> — position your hand 30–60 cm from the camera</li>
          <li><strong class="text-white">Pinch</strong> thumb + index → Left Click</li>
          <li><strong class="text-white">Pinch</strong> thumb + middle → Right Click</li>
          <li><strong class="text-white">Move hand</strong> slowly → Scroll</li>
          <li><strong class="text-white">Spread / pinch</strong> fingers → Zoom In / Out</li>
        </ul>
      </div>
    </div>

    <!-- Other views -->
    <SettingsView    v-if="currentView === 'settings'"    />
    <CalibrationView v-if="currentView === 'calibration'" />
    <AboutView       v-if="currentView === 'about'"       />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import SettingsView    from './views/Settings.vue'
import CalibrationView from './views/Calibration.vue'
import AboutView       from './views/About.vue'

// ── Nav views ──────────────────────────────────────────────────────────────
const views = [
  { id: 'main',        label: 'Main' },
  { id: 'settings',    label: 'Settings' },
  { id: 'calibration', label: 'Calibration' },
  { id: 'about',       label: 'About' },
] as const
type ViewId = typeof views[number]['id']
const currentView = ref<ViewId>('main')

// ── Tracking state ──────────────────────────────────────────────────────────
const tracking      = ref(false)
const toggleMessage = ref('')
const toggleError   = ref(false)

// ── Tauri guard ─────────────────────────────────────────────────────────────
const isInsideTauri = isTauri()

// ── Tracking status (from "tracking-status" event) ──────────────────────────
interface StatusPayload {
  state:      string
  message:    string
  hint:       string
  gesture:    string | null
  confidence: number | null
}
const trackingStatus = ref<StatusPayload>({
  state: '', message: '', hint: '', gesture: null, confidence: null,
})

// ── Gesture log ─────────────────────────────────────────────────────────────
interface GestureEntry { id: number; name: string; time: string }
const gestureLog = ref<GestureEntry[]>([])
let gestureSeq = 0

function pushGesture(name: string) {
  const now = new Date()
  const time = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  gestureLog.value.unshift({ id: gestureSeq++, name, time })
  if (gestureLog.value.length > 8) gestureLog.value.pop()
}

// ── Status banner helpers ────────────────────────────────────────────────────
const statusBannerClass = computed(() => {
  switch (trackingStatus.value.state) {
    case 'no_hand':      return 'bg-gray-700/80 text-gray-200 border border-gray-600'
    case 'too_close':    return 'bg-amber-800/80 text-amber-100 border border-amber-600'
    case 'too_far':      return 'bg-amber-800/80 text-amber-100 border border-amber-600'
    case 'hand_detected': return 'bg-green-800/70 text-green-100 border border-green-600'
    case 'gesture':      return 'bg-blue-700/80 text-blue-100 border border-blue-500'
    default:             return 'bg-gray-700/80 text-gray-200 border border-gray-600'
  }
})

const statusIcon = computed(() => {
  switch (trackingStatus.value.state) {
    case 'no_hand':       return '👀'
    case 'too_close':     return '↔️'
    case 'too_far':       return '🔍'
    case 'hand_detected': return '✋'
    case 'gesture':       return '⚡'
    default:              return '⏳'
  }
})

function gestureIcon(name: string): string {
  const map: Record<string, string> = {
    'Left Click':   '🖱️',
    'Right Click':  '🖱️',
    'Scroll Up':    '⬆️',
    'Scroll Down':  '⬇️',
    'Scroll Left':  '⬅️',
    'Scroll Right': '➡️',
    'Swipe Up':     '☝️',
    'Swipe Down':   '👇',
    'Swipe Left':   '👈',
    'Swipe Right':  '👉',
    'Zoom In':      '🔎',
    'Zoom Out':     '🔍',
    'App Switch':   '🔄',
    'Screenshot':   '📸',
  }
  return map[name] ?? '🤏'
}

// ── Event listeners ──────────────────────────────────────────────────────────
let unlistenStatus:  UnlistenFn | null = null
let unlistenGesture: UnlistenFn | null = null

onMounted(async () => {
  if (!isInsideTauri) return
  try {
    unlistenStatus = await listen<StatusPayload>('tracking-status', (ev) => {
      trackingStatus.value = ev.payload
    })
    unlistenGesture = await listen<string>('gesture-detected', (ev) => {
      pushGesture(ev.payload)
    })
  } catch (e) {
    console.error('[App] Failed to subscribe to events:', e)
  }
})

onUnmounted(() => {
  unlistenStatus?.(); unlistenStatus = null
  unlistenGesture?.(); unlistenGesture = null
})

// ── Toggle tracking ──────────────────────────────────────────────────────────
async function toggleTracking() {
  if (!isInsideTauri) {
    toggleError.value = false
    toggleMessage.value = 'Run via "npm run tauri:dev" to enable tracking.'
    return
  }
  try {
    if (tracking.value) {
      const result = await invoke<string>('stop_tracking')
      toggleMessage.value = result
      tracking.value = false
      trackingStatus.value = { state: '', message: '', hint: '', gesture: null, confidence: null }
    } else {
      const result = await invoke<string>('start_tracking')
      toggleMessage.value = result
      tracking.value = true
      gestureLog.value = []
    }
    toggleError.value = false
  } catch (e: any) {
    toggleMessage.value = 'Error: ' + (e?.message ?? e)
    toggleError.value = true
  }
}
</script>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.25s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
