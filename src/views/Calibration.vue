<!--
  Calibration.vue — live camera preview + gesture tester.

  Preview always works:
  • Not tracking → polls get_camera_preview for raw camera feed.
  • Tracking active → sidecar streams annotated JPEG frames via
    the "preview-frame" Tauri event; get_camera_preview also returns
    the cached sidecar frame so the poll still works.
-->
<template>
  <div class="min-h-screen bg-gray-900 text-white p-6">
    <div class="max-w-4xl mx-auto space-y-6">
      <h1 class="text-2xl font-bold">Calibration &amp; Testing</h1>

      <!-- ═══════════ Camera Preview ═══════════ -->
      <div class="bg-gray-800 rounded-xl p-5 border border-gray-700">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold">Camera Preview</h2>
          <div class="flex items-center gap-3">
            <span v-if="trackingActive" class="flex items-center gap-1.5 text-xs text-blue-400">
              <span class="w-2 h-2 rounded-full bg-blue-400 animate-pulse"></span>Tracking
            </span>
            <span v-else-if="previewActive" class="flex items-center gap-1.5 text-xs text-green-400">
              <span class="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>Live
            </span>
          </div>
        </div>

        <!-- Canvas -->
        <div class="relative bg-black rounded-lg overflow-hidden" style="aspect-ratio: 4/3;">
          <canvas
            ref="canvas"
            class="w-full h-full object-contain"
            width="640"
            height="480"
          ></canvas>
          <!-- Idle overlay (neither preview nor tracking) -->
          <div
            v-if="!previewActive && !trackingActive"
            class="absolute inset-0 flex flex-col items-center justify-center text-gray-500 gap-2"
          >
            <span class="text-5xl">📷</span>
            <span class="text-sm">Camera preview inactive</span>
          </div>
        </div>

        <!-- Status message below canvas -->
        <p v-if="statusMessage" class="mt-2 text-sm text-amber-400">{{ statusMessage }}</p>

        <!-- Controls -->
        <div class="mt-4 flex flex-wrap gap-3 items-center">
          <button
            @click="togglePreview"
            :disabled="!isInsideTauri"
            :class="[
              previewActive ? 'bg-red-600 hover:bg-red-700' : 'bg-green-600 hover:bg-green-700',
              !isInsideTauri ? 'opacity-40 cursor-not-allowed' : '',
            ]"
            class="px-5 py-2.5 rounded-lg text-sm font-medium transition text-white"
          >
            {{ previewActive ? 'Stop Preview' : 'Start Preview' }}
          </button>
          <span class="text-xs text-gray-400" v-if="trackingActive">
            Showing live annotated tracking feed
          </span>
          <span v-if="!isInsideTauri" class="text-xs text-amber-400">
            Requires Tauri — run <code class="bg-gray-700 px-1 rounded">npm run tauri:dev</code>
          </span>
        </div>
      </div>

      <!-- ═══════════ Live Tracking Status (when tracking is active) ═══════════ -->
      <div v-if="trackingActive" class="bg-gray-800 rounded-xl p-5 border border-gray-700">
        <h2 class="text-lg font-semibold mb-4">Live Tracking Status</h2>
        <div :class="statusBannerClass" class="rounded-lg p-4 flex items-start gap-3 mb-4 transition-colors duration-200">
          <span class="text-2xl flex-shrink-0">{{ statusIcon }}</span>
          <div>
            <p class="font-semibold text-sm">{{ liveStatus.message || 'Waiting for data…' }}</p>
            <p class="text-xs mt-0.5 opacity-75">{{ liveStatus.hint }}</p>
          </div>
        </div>
      </div>

      <!-- ═══════════ Gesture Test ═══════════ -->
      <div class="bg-gray-800 rounded-xl p-5 border border-gray-700">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold">Gesture Test</h2>
          <button
            v-if="recentGestures.length > 0"
            @click="recentGestures = []"
            class="text-xs text-gray-500 hover:text-gray-300 transition"
          >Clear</button>
        </div>

        <div v-if="recentGestures.length > 0" class="space-y-2">
          <div
            v-for="(g, i) in recentGestures"
            :key="g.id"
            :class="i === 0 ? 'ring-1 ring-blue-500 bg-blue-900/30' : 'bg-gray-700/50'"
            class="flex items-center gap-3 rounded-lg px-4 py-2.5 text-sm transition-all"
          >
            <span class="text-xl">{{ gestureIcon(g.name) }}</span>
            <span class="font-medium flex-1">{{ g.name }}</span>
            <span class="text-xs text-gray-400 tabular-nums">{{ g.time }}</span>
          </div>
        </div>
        <p v-else class="text-gray-500 text-sm text-center py-6">
          No gestures detected yet.<br>
          <span class="text-xs">Start tracking or camera preview and perform a gesture.</span>
        </p>
      </div>

      <!-- ═══════════ Gesture Reference ═══════════ -->
      <div class="bg-gray-800 rounded-xl p-5 border border-gray-700">
        <h2 class="text-lg font-semibold mb-4">Gesture Reference</h2>

        <!-- Single hand -->
        <p class="text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">✋ Single Hand</p>
        <div class="space-y-2 mb-4">
          <div class="flex items-start gap-3 bg-gray-700/40 rounded-lg px-4 py-3">
            <span class="text-2xl">🖐️</span>
            <div>
              <p class="text-sm font-semibold text-white">Cursor Move <span class="text-xs font-normal text-green-400 ml-1">4–5 fingers up</span></p>
              <p class="text-xs text-gray-400">Raise all 4–5 fingers. Index PIP joint (lm 6) drives the cursor — smoother than fingertip. Cursor shown as <span class="text-green-400">green crosshair</span>.</p>
            </div>
          </div>
          <div class="flex items-start gap-3 bg-gray-700/40 rounded-lg px-4 py-3">
            <span class="text-2xl">🤌</span>
            <div>
              <p class="text-sm font-semibold text-white">Left Click <span class="text-xs font-normal text-cyan-400 ml-1">pinch while in cursor mode</span></p>
              <p class="text-xs text-gray-400">While 4–5 fingers up, bring <strong class="text-white">thumb tip → index tip</strong> close together. Hold 2+ frames to confirm. The pinch bar at the bottom of the preview shows proximity.</p>
            </div>
          </div>
          <div class="flex items-start gap-3 bg-gray-700/40 rounded-lg px-4 py-3">
            <span class="text-2xl">✌️</span>
            <div>
              <p class="text-sm font-semibold text-white">Scroll <span class="text-xs font-normal text-orange-400 ml-1">2 fingers up (index + middle)</span></p>
              <p class="text-xs text-gray-400">Extend only <strong class="text-white">index + middle fingers</strong> (others folded). Move hand up/down/left/right to scroll. Skeleton turns <span class="text-orange-400">orange</span> in scroll mode.</p>
            </div>
          </div>
        </div>

        <!-- Two hands -->
        <p class="text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">🤲 Two Hands</p>
        <div class="space-y-2 mb-4">
          <div class="flex items-start gap-3 bg-gray-700/40 rounded-lg px-4 py-3">
            <span class="text-2xl">🖱️</span>
            <div>
              <p class="text-sm font-semibold text-white">Right Click</p>
              <p class="text-xs text-gray-400">Both hands: pinch <strong class="text-white">thumb + index</strong> simultaneously</p>
            </div>
          </div>
          <div class="flex items-start gap-3 bg-gray-700/40 rounded-lg px-4 py-3">
            <span class="text-2xl">👋</span>
            <div>
              <p class="text-sm font-semibold text-white">Swipe Left / Right / Up / Down</p>
              <p class="text-xs text-gray-400">Move <strong class="text-white">both hands together</strong> in the same direction — wrist centroid tracked over 6 frames, 1.5 s cooldown</p>
            </div>
          </div>
        </div>

        <!-- Visual key -->
        <p class="text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">🎨 Colour Guide</p>
        <div class="flex flex-wrap gap-3 text-xs">
          <span class="flex items-center gap-1.5"><span class="w-3 h-3 rounded-full bg-green-400 inline-block"></span><span class="text-gray-300">Cursor mode</span></span>
          <span class="flex items-center gap-1.5"><span class="w-3 h-3 rounded-full bg-orange-400 inline-block"></span><span class="text-gray-300">Scroll mode</span></span>
          <span class="flex items-center gap-1.5"><span class="w-3 h-3 rounded-full bg-cyan-400 inline-block"></span><span class="text-gray-300">Gesture fired</span></span>
          <span class="flex items-center gap-1.5"><span class="w-3 h-3 rounded-full bg-gray-500 inline-block"></span><span class="text-gray-300">Idle / unrecognised</span></span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// ── Canvas / preview ────────────────────────────────────────────────────────
const canvas       = ref<HTMLCanvasElement | null>(null)
const previewActive = ref(false)
const statusMessage = ref('')
const isInsideTauri = isTauri()

let previewInterval: ReturnType<typeof setInterval> | null = null


async function stopPreviewAndTracking() {
  previewActive.value = false
  trackingActive.value = false
  if (previewInterval !== null) { clearInterval(previewInterval); previewInterval = null }
  const c = canvas.value
  if (c) { const ctx = c.getContext('2d'); ctx?.clearRect(0, 0, c.width, c.height) }
  try { await invoke('stop_tracking') } catch (_) {}
}

/** Render a base64 JPEG string onto the canvas. */
function renderBase64Frame(b64: string) {
  if (!b64) return
  const c = canvas.value; if (!c) return
  const ctx = c.getContext('2d'); if (!ctx) return
  const img = new Image()
  img.onload = () => ctx.drawImage(img, 0, 0, c.width, c.height)
  img.src = 'data:image/jpeg;base64,' + b64
}

function startPreviewPoll() {
  previewInterval = setInterval(async () => {
    try {
      const result = await invoke<string>('get_camera_preview')
      if (result) renderBase64Frame(result)
    } catch (e) {
      console.error('[Calibration] Preview error:', e)
    }
  }, 120)
}

async function togglePreview() {
  if (!isInsideTauri) return
  if (previewActive.value) {
    // Stop preview and tracking
    await stopPreviewAndTracking()
    statusMessage.value = ''
  } else {
    // Start tracking then begin preview poll
    try {
      await invoke('start_tracking')
    } catch (e) {
      console.error('[Calibration] start_tracking error:', e)
    }
    previewActive.value = true
    statusMessage.value = ''
    startPreviewPoll()
  }
}

// ── Tracking-status event subscription ─────────────────────────────────────
interface StatusPayload { state: string; message: string; hint: string; gesture: string | null; confidence: number | null }
const liveStatus    = ref<StatusPayload>({ state: '', message: '', hint: '', gesture: null, confidence: null })
const trackingActive = ref(false)

const statusBannerClass = computed(() => {
  switch (liveStatus.value.state) {
    case 'no_hand':       return 'bg-gray-700 text-gray-200'
    case 'too_close':
    case 'too_far':       return 'bg-amber-800/80 text-amber-100'
    case 'hand_detected': return 'bg-green-800/70 text-green-100'
    case 'gesture':       return 'bg-blue-700/80 text-blue-100'
    default:              return 'bg-gray-700 text-gray-200'
  }
})

const statusIcon = computed(() => {
  switch (liveStatus.value.state) {
    case 'no_hand':       return '👀'
    case 'too_close':     return '↔️'
    case 'too_far':       return '🔍'
    case 'hand_detected': return '✋'
    case 'gesture':       return '⚡'
    default:              return '⏳'
  }
})

// ── Gesture log ─────────────────────────────────────────────────────────────
interface GestureEntry { id: number; name: string; time: string }
const recentGestures = ref<GestureEntry[]>([])
let seq = 0

function gestureIcon(name: string): string {
  const map: Record<string, string> = {
    'Left Click':    '👆',   // index+middle tap
    'Right Click':   '🖱️',
    'Scroll Up':     '⬆️',
    'Scroll Down':   '⬇️',
    'Scroll Left':   '⬅️',
    'Scroll Right':  '➡️',
    'Swipe Up':      '☝️',
    'Swipe Down':    '👇',
    'Swipe Left':    '👈',
    'Swipe Right':   '👉',
    'Zoom In':       '🔎',
    'Zoom Out':      '🔍',
  }
  return map[name] ?? '🤏'
}

// ── Event listeners ──────────────────────────────────────────────────────────
let unlistenStatus:  UnlistenFn | null = null
let unlistenPreview: UnlistenFn | null = null
let unlistenGesture: UnlistenFn | null = null
let unlistenStopped: UnlistenFn | null = null

onMounted(async () => {
  if (!isInsideTauri) return
  try {
    unlistenStatus = await listen<StatusPayload>('tracking-status', (ev) => {
      liveStatus.value = ev.payload
      trackingActive.value = true
    })
    // Live annotated preview frames streamed from the sidecar
    unlistenPreview = await listen<string>('preview-frame', (ev) => {
      renderBase64Frame(ev.payload)
      if (!previewActive.value && trackingActive.value) {
        previewActive.value = true
      }
    })
    unlistenGesture = await listen<string>('gesture-detected', (ev) => {
      const now = new Date()
      const time = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
      recentGestures.value.unshift({ id: seq++, name: ev.payload, time })
      if (recentGestures.value.length > 12) recentGestures.value.pop()
    })
    // Reset preview/tracking state when tracking is stopped externally (e.g. tray)
    unlistenStopped = await listen('tracking-stopped', () => {
      previewActive.value  = false
      trackingActive.value = false
      if (previewInterval !== null) { clearInterval(previewInterval); previewInterval = null }
    })
  } catch (e) {
    console.error('[Calibration] Failed to subscribe to events:', e)
  }
})

onUnmounted(() => {
  stopPreviewAndTracking()
  unlistenStatus?.();  unlistenStatus  = null
  unlistenPreview?.(); unlistenPreview = null
  unlistenGesture?.(); unlistenGesture = null
  unlistenStopped?.(); unlistenStopped = null
})
</script>
