<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import ConfigAppPage from '@/layouts/ConfigAppPage.vue';
import SystemInfoView from '@/views/applets/SystemInfoView.vue';
</script>

<template>
  <ConfigAppPage :component="SystemInfoView" />
</template>