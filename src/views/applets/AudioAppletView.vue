
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import SystemAudioArea from '@/components/areas/audio/SystemAudioArea.vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import { toggleAudioApplet } from '@/services/window.service';
</script>

<template>
  <AppletFrame :close-fn="toggleAudioApplet">
    <SystemAudioArea />
  </AppletFrame>
</template>
