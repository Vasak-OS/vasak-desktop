<script setup lang="ts">
import { onMounted, ref } from 'vue';

interface TimeData {
	day: string;
	month: string;
	year: string;
	hour: string;
	minute: string;
}

const timeData = ref<TimeData>({
	day: '00',
	month: '00',
	year: '0000',
	hour: '00',
	minute: '00',
});

const formatNumber = (num: number): string => num.toString().padStart(2, '0');

const updateTime = () => {
	const date = new Date();
	timeData.value = {
		hour: formatNumber(date.getHours()),
		minute: formatNumber(date.getMinutes()),
		day: formatNumber(date.getDate()),
		month: formatNumber(date.getMonth() + 1),
		year: date.getFullYear().toString(),
	};
};

onMounted(() => {
	updateTime();
	setInterval(updateTime, 5000);
});
</script>

<template>
  <div class="flex items-center p-1 font-mono text-sm">
    <span 
      :title="`${timeData.day}/${timeData.month}/${timeData.year}`"
      class="cursor-default animate-pulse"
    >
      {{ timeData.hour }}:{{ timeData.minute }}
    </span>
  </div>
</template>

