
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import { toggleNetworkApplet } from '@/services/window.service';
</script>

<template>
  <AppletFrame :close-fn="toggleNetworkApplet">
    <NetworkControlArea />
  </AppletFrame>
</template>
