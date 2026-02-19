<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import BluetoothControlArea from '@/components/areas/bluetooth/BluetoothControlArea.vue';
import ConfigAppPage from '@/layouts/ConfigAppPage.vue';
</script>

<template>
  <ConfigAppPage
    :component="BluetoothControlArea"
    :component-props="{ class: 'w-full' }"
  />
</template>