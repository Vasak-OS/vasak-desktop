<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import SystemAudioArea from '@/components/areas/audio/SystemAudioArea.vue';
import ConfigAppPage from '@/layouts/ConfigAppPage.vue';
</script>

<template>
  <ConfigAppPage :component="SystemAudioArea" />
</template>