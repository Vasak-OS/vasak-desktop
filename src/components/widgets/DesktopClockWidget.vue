<script lang="ts" setup>
import { ref } from 'vue';

const time = ref(new Date().toLocaleTimeString());

setInterval(() => {
    time.value = new Date().toLocaleTimeString();
}, 1000);
</script>

<template>
    <div class="flex flex-col items-center justify-center h-auto p-4 rounded-vsk bg-gray-900/50 backdrop-blur-lg shadow-lg mb-4">
        <h1 class="text-6xl font-bold text-white">{{ time }}</h1>
    </div>
</template>