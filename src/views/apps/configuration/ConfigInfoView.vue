<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import SystemInfoView from '@/components/configuration/SystemInfoView.vue';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';
</script>

<template>
  <ConfigAppLayout>
    <SystemInfoView />
  </ConfigAppLayout>
</template>