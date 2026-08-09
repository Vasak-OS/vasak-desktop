<script lang="ts" setup>
import { onMounted, onUnmounted, ref } from 'vue';

const time = ref(new Date().toLocaleTimeString());

// The timer used to be created at setup scope and never cleared, so it kept
// running — and kept the component alive — after the widget was gone.
let tick: ReturnType<typeof setInterval> | undefined;

const updateTime = () => {
	time.value = new Date().toLocaleTimeString();
};

onMounted(() => {
	updateTime();
	tick = setInterval(updateTime, 1000);
});

onUnmounted(() => {
	if (tick !== undefined) clearInterval(tick);
});
</script>

<template>
    <div class="flex flex-col items-center justify-center h-auto p-4 rounded-corner bg-ui-bg/80 backdrop-blur-lg shadow-lg mb-4 ring-2 ring-primary/50">
        <h1 class="text-6xl font-bold text-white font-mono">{{ time }}</h1>
    </div>
</template>