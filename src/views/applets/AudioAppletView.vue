<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import SystemAudioArea from '@/components/areas/audio/SystemAudioArea.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
</script>

<template>
  <AppletPopover applet="audio">
    <SystemAudioArea />
  </AppletPopover>
</template>
