<script>
import { onMounted } from 'vue';
import { listen } from '@tauri-apps/api/event';

export default {
  setup() {
    onMounted(() => {
      listen('music-playing-update', (event) => {
        console.log('Music playing update:', event);
      });
    });
  },
};
</script>
<template>
  
</template>