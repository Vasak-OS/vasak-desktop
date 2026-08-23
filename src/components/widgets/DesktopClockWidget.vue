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
    <div class="flex h-full w-full flex-col items-center justify-center p-[4cqmin]">
        <h1 class="font-mono text-[clamp(1.75rem,38cqh,7rem)] font-bold leading-none text-tx-main">{{ time }}</h1>
    </div>
</template>