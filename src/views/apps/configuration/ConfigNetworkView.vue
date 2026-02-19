<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import NetworkControlArea from '@/components/configuration/NetworkControlArea.vue';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';
</script>

<template>
  <ConfigAppLayout>
    <NetworkControlArea hide-x />
  </ConfigAppLayout>
</template>