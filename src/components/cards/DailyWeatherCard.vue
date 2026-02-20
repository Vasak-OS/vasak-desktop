<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';

const props = defineProps({
	date: {
		type: String,
		required: true,
	},
	min: {
		type: Number,
		required: true,
	},
	max: {
		type: Number,
		required: true,
	},
	units: {
		type: Object,
		required: true,
	},
	dayOrNight: {
		type: String as () => 'day' | 'night',
		required: true,
	},
	weatherCode: {
		type: Number,
		required: true,
	},
});

const formattedDate = computed(() => {
	const dateObj = new Date(props.date);
	dateObj.setDate(dateObj.getDate() + 1);
	return dateObj.toLocaleDateString(undefined, { month: 'numeric', day: 'numeric' });
});

const dayOrNightType = computed(() => props.dayOrNight as 'day' | 'night');
</script>
<template>
  <div class="flex flex-col items-center gap-1 p-2 rounded-corner bg-primary/80 dark:bg-primary-dark/80 daily-weather-card-layout">
    <div class="date-display">{{ formattedDate }}</div>
	<WeatherIcon :code="weatherCode" :dayOrNight="dayOrNightType" class="weather-icon" />
    <div class="temperatures">
      <span class="temp-max">{{ max }}°</span>
      <span class="temp-min">{{ min }}°</span>
    </div>
  </div>
</template>

<style scoped>
.daily-weather-card-layout {
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out;
}

.daily-weather-card-layout:hover {
  transform: translateY(-3px) scale(1.02);
  box-shadow: 0 3px 10px rgba(0,0,0,0.15);
}

.date-display {
  font-size: 0.875rem;
  font-weight: 500;
}

.weather-icon {
  margin-top: 0.1rem;
  margin-bottom: 0.1rem;
  transition: transform 0.2s ease-in-out;
}

.daily-weather-card-layout:hover .weather-icon {
  transform: scale(1.1);
}

.temperatures {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.temp-max {
  font-weight: 600;
}

.temp-min {
  font-weight: 400;
}
</style>
