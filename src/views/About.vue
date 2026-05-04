<template>
  <div class="min-h-screen bg-gray-900 text-white p-8">
    <div class="max-w-4xl mx-auto">
      <h1 class="text-3xl font-bold mb-8">About ScreenCursor</h1>

      <div class="bg-gray-800 rounded-lg p-6 mb-8">
        <h2 class="text-xl font-semibold mb-4">Version</h2>
        <p class="text-gray-300">ScreenCursor v0.1.0</p>
      </div>

      <div class="bg-gray-800 rounded-lg p-6 mb-8">
        <h2 class="text-xl font-semibold mb-4">Description</h2>
        <p class="text-gray-300 leading-relaxed">
          ScreenCursor is a desktop application that uses computer vision to track your hand movements
          and translates them into mouse and keyboard actions. Control your computer hands-free
          using intuitive gestures.
        </p>
      </div>

      <div class="bg-gray-800 rounded-lg p-6 mb-8">
        <h2 class="text-xl font-semibold mb-4">Supported Gestures</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">Left Click</h3>
            <p class="text-sm text-gray-400">Pinch thumb + index finger</p>
          </div>
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">Scroll</h3>
            <p class="text-sm text-gray-400">Move hand up/down</p>
          </div>
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">Swipe</h3>
            <p class="text-sm text-gray-400">Quick hand movement in any direction</p>
          </div>
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">Zoom</h3>
            <p class="text-sm text-gray-400">Spread/pinch two fingers (or use two hands)</p>
          </div>
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">App Switch</h3>
            <p class="text-sm text-gray-400">Three-finger gesture (Alt+Tab / Cmd+Tab)</p>
          </div>
          <div class="bg-gray-700 rounded p-4">
            <h3 class="font-semibold mb-2">Screenshot</h3>
            <p class="text-sm text-gray-400">Specific gesture (Win+Shift+S / Cmd+Shift+4)</p>
          </div>
        </div>
      </div>

      <div class="bg-gray-800 rounded-lg p-6">
        <h2 class="text-xl font-semibold mb-4">Technology</h2>
        <ul class="list-disc list-inside space-y-2 text-gray-300">
          <li>Tauri v2 (Desktop Framework)</li>
          <li>OpenCV (Computer Vision)</li>
          <li>ONNX Runtime (Hand Detection & Landmarks)</li>
          <li>Vue 3 + TypeScript (Frontend)</li>
          <li>Rust (Backend)</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
</script>
