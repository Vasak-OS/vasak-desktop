<script lang="ts" setup>
import AudioDeviceSelector from '@/components/controls/AudioDeviceSelector.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';
</script>
<template>
  <div class="p-4">
    <h1 class="text-2xl font-bold mb-4">Audio Configuration</h1>
    <div class="mb-6">
      <h2 class="text-xl font-semibold mb-2">Volume Control</h2>
      <VolumeControl />
    </div>
    <h2 class="text-xl font-semibold mb-2">Audio Device Selector</h2>
    <div class="background rounded-vsk p-4">
      <AudioDeviceSelector />
    </div>
  </div>
</template>
