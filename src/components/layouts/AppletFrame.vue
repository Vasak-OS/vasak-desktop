<script setup lang="ts">
import { useEscapeToClose } from '@/tools/escape.close';

interface Props {
	closeFn: () => void;
}

const props = defineProps<Props>();
useEscapeToClose(props.closeFn);
</script>

<template>
  <div class="bg-ui-bg/80 h-screen w-screen border border-ui-border rounded-corner-window p-4">
    <slot />
  </div>
</template>
