<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
</script>

<template>
  <AppletPopover applet="network">
    <NetworkControlArea />
  </AppletPopover>
</template>
