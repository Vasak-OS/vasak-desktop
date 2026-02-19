<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import BluetoothControlArea from '@/components/configuration/BluetoothControlArea.vue';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';
</script>

<template>
  <ConfigAppLayout>
    <BluetoothControlArea class="w-full" />
  </ConfigAppLayout>
</template>