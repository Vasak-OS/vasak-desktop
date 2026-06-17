<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed, onMounted, ref } from 'vue';
import DailyWeatherCard from '@/components/cards/DailyWeatherCard.vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';

const weather = ref({
	latitude: 52.52,
	longitude: 13.419998,
	generationtime_ms: 0.06401538848876953,
	utc_offset_seconds: 0,
	timezone: 'GMT',
	timezone_abbreviation: 'GMT',
	elevation: 38,
	current_units: {
		time: 'iso8601',
		interval: 'seconds',
		temperature_2m: '°C',
		is_day: '',
		weather_code: 'wmo code',
	},
	current: {
		time: '2023-12-12T13:45',
		interval: 900,
		temperature_2m: 6.7,
		is_day: 1,
		weather_code: 3,
	},
	daily_units: {
		time: 'iso8601',
		weather_code: 'wmo code',
		temperature_2m_max: '°C',
		temperature_2m_min: '°C',
	},
	daily: {
		time: [
			'2023-12-12',
			'2023-12-13',
			'2023-12-14',
			'2023-12-15',
			'2023-12-16',
			'2023-12-17',
			'2023-12-18',
		],
		weather_code: [80, 63, 61, 3, 61, 61, 3],
		temperature_2m_max: [7, 4.3, 3.7, 5.1, 7.8, 9.5, 8.2],
		temperature_2m_min: [3, 1, 1.1, 0.1, 4.8, 8, 6.6],
	},
});

const getLocation = async () => {
	const location = await fetch('http://ip-api.com/json/')
		.then((res) => res.json())
		.then((data) => data);
	return location;
};

const getWeather = async () => {
	const location = await getLocation();
	const weather = await fetch(
		`https://api.open-meteo.com/v1/forecast?latitude=${location.lat}&longitude=${location.lon}&current=temperature_2m,is_day,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min`
	)
		.then((res) => res.json())
		.then((data) => data);
	return weather;
};

const dayOrNight = computed(() => {
	return weather.value.current.is_day ? 'day' : 'night';
});

const formatDay = (dateStr: string) => {
	const d = new Date(dateStr);
	d.setDate(d.getDate() + 1);
	return d.toLocaleDateString(undefined, { weekday: 'long' });
};

onMounted(async () => {
	weather.value = await getWeather();
});
</script>

<template>
  <div class="h-full grid gap-2 min-h-0" style="grid-template-columns: 2fr 3fr;">
    <template v-if="weather">
      <div class="flex flex-col items-center justify-center gap-4 rounded-corner bg-ui-surface/80 border border-primary p-4">
        <div class="text-4xl font-bold">{{ weather.current.temperature_2m }}{{ weather.current_units.temperature_2m }}</div>
        <WeatherIcon :code="weather.current.weather_code" :dayOrNight="dayOrNight" class="w-16 h-16" />
        <div class="text-lg font-semibold">{{ formatDay(weather.daily.time[0]) }}</div>
        <div class="flex gap-3 text-base">
          <span class="font-semibold">{{ weather.daily.temperature_2m_max[0] }}°</span>
          <span class="text-vsk-text/60">{{ weather.daily.temperature_2m_min[0] }}°</span>
        </div>
      </div>

      <div class="grid grid-cols-3 grid-rows-2 gap-2 min-h-0">
        <DailyWeatherCard
          v-for="(_, key) in weather.daily.time.slice(1)"
          :key="key + 1"
          :date="weather.daily.time[key + 1]"
          :min="weather.daily.temperature_2m_min[key + 1]"
          :max="weather.daily.temperature_2m_max[key + 1]"
          :units="weather.daily_units"
          :dayOrNight="dayOrNight"
          :weatherCode="weather.daily.weather_code[key + 1]"
          class="h-full"
        />
      </div>
    </template>
    <template v-else> NO se puede cargar el clima </template>
  </div>
</template>

