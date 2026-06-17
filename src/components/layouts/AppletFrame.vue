<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';

interface Props {
	closeFn: () => void;
}

const props = defineProps<Props>();
const leaving = ref(false);

const closeAfterAnimation = () => {
	leaving.value = true;
	setTimeout(() => {
		try { props.closeFn(); } catch { /* window already closed */ }
	}, 200);
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') {
		closeAfterAnimation();
	}
};

const onBlur = () => {
	closeAfterAnimation();
};

onMounted(() => {
	document.addEventListener('keydown', onKeydown);
	window.addEventListener('blur', onBlur);
});

onBeforeUnmount(() => {
	document.removeEventListener('keydown', onKeydown);
	window.removeEventListener('blur', onBlur);
});
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <div
      :class="['bg-ui-bg/80 h-screen w-screen border border-ui-border rounded-corner-window p-4', { 'leave-active': leaving }]"
    >
      <slot />
    </div>
  </Transition>
</template>

<style scoped>
@keyframes scale-in {
  from {
    transform: scale(0.95);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes scale-out {
  from {
    transform: scale(1);
    opacity: 1;
  }
  to {
    transform: scale(0.95);
    opacity: 0;
  }
}

.enter-active {
  animation: scale-in 200ms ease-out;
}

.leave-active {
  animation: scale-out 200ms ease-in;
}
</style>
