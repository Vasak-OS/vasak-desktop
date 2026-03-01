<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed, onMounted, ref } from 'vue';
import CurrentWeatherCard from '@/components/cards/CurrentWeatherCard.vue';
import DailyWeatherCard from '@/components/cards/DailyWeatherCard.vue';

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

onMounted(async () => {
	weather.value = await getWeather();
});
</script>

<template>
  <div class="flex flex-col gap-4">
    <template v-if="weather">
      <CurrentWeatherCard
        :current="weather.current"
        :units="weather.current_units"
        :dayOrNight="dayOrNight"
      />
      <transition-group
        tag="div"
        move-class="transition-transform duration-300 ease-out" enter-active-class="transition-all duration-300 ease-out" leave-active-class="transition-all duration-300 ease-out" enter-from-class="opacity-0 scale-80 translate-y-[15px]" leave-to-class="opacity-0 scale-80 translate-y-[15px]"
        appear
        class="flex flex-wrap gap-2 justify-around [&>div]:basis-[90px] [&>div]:grow [&>div]:text-center [&_.weather-icon]:w-[38px] [&_.weather-icon]:h-[38px] [&_.weather-icon]:drop-shadow-[0px_1px_2px_rgba(0,0,0,0.3)] [&_.temp-max]:text-base [&_.temp-max]:font-semibold [&_.temp-min]:text-base [&_.temp-min]:font-semibold"
      >
        <DailyWeatherCard
          v-for="(_value, key) in weather.daily.time"
          :key="key"
          :date="weather.daily.time[key]"
          :min="weather.daily.temperature_2m_min[key]"
          :max="weather.daily.temperature_2m_max[key]"
          :units="weather.daily_units"
          :dayOrNight="dayOrNight"
          :weatherCode="weather.daily.weather_code[key]"
        />
      </transition-group>
    </template>
    <template v-else> NO se puede cargar el clima </template>
  </div>
</template>

