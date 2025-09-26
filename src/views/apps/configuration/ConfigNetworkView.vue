<script lang="ts" setup>
import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';

</script>

<template>
  <ConfigAppLayout>
    <NetworkControlArea class="w-[calc(100%-96px)]" hide-x />
  </ConfigAppLayout>
</template>