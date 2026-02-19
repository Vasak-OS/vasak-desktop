<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';
import SystemInfoView from '@/views/applets/SystemInfoView.vue';
</script>

<template>
  <ConfigAppLayout>
    <SystemInfoView />
  </ConfigAppLayout>
</template>