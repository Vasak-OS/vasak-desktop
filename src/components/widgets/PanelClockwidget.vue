<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';

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

/**
 * Wakes up on the minute instead of every five seconds.
 *
 * The old timer fired 12 times a minute for a clock that only shows HH:MM, and
 * because it was not aligned to the minute the displayed time could be up to
 * five seconds stale. Scheduling to the next minute boundary is both accurate
 * and ~12x fewer wakeups. The interval was also never cleared.
 */
let tickTimer: ReturnType<typeof setTimeout> | undefined;

const scheduleNextTick = () => {
	const msUntilNextMinute = 60_000 - (Date.now() % 60_000);
	tickTimer = setTimeout(() => {
		updateTime();
		scheduleNextTick();
	}, msUntilNextMinute);
};

onMounted(() => {
	updateTime();
	scheduleNextTick();
});

onUnmounted(() => {
	if (tickTimer !== undefined) clearTimeout(tickTimer);
});
</script>

<template>
  <div class="flex items-center p-1 font-mono text-sm">
    <span 
      :title="`${timeData.day}/${timeData.month}/${timeData.year}`"
      class="cursor-default"
    >
      {{ timeData.hour }}:{{ timeData.minute }}
    </span>
  </div>
</template>

