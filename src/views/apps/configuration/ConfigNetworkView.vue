<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import ConfigAppPage from '@/layouts/ConfigAppPage.vue';
</script>

<template>
  <ConfigAppPage
    :component="NetworkControlArea"
    :component-props="{ hideX: true }"
  />
</template>