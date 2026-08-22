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
    <!-- container-type hace que las unidades cqh de abajo midan contra el widget y
         no contra la ventana: sin esto, la hora salía del tamaño de la pantalla. -->
    <div style="container-type: size"
        class="flex h-full w-full flex-col items-center justify-center overflow-hidden rounded-corner bg-ui-bg/80 p-3 shadow-lg ring-2 ring-primary/50 backdrop-blur-lg">
        <h1 class="font-mono text-[clamp(1.5rem,18cqh,4rem)] font-bold leading-none text-tx-main">{{ time }}</h1>
    </div>
</template>