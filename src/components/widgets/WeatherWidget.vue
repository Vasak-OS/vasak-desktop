<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import DailyWeatherCard from '@/components/cards/DailyWeatherCard.vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';

const { t } = useI18n();

// Starts empty so the "unavailable" state can actually render.
//
// It used to be seeded with a hardcoded Berlin forecast dated 2023-12-12, which
// made the v-else branch unreachable: with no network the widget showed that
// stale data as if it were real, rather than saying it had nothing.
const weather = ref<any>(null);
const failed = ref(false);

/**
 * Approximate location from the system time zone.
 *
 * This used to POST the user's IP to `http://ip-api.com` — over plaintext HTTP,
 * to a third party, on every menu open, with no consent and no way to turn it
 * off. The time zone already names a nearby city and is local information, so
 * geocoding that through the same provider we already use for the forecast
 * needs no new third party and never sends the IP anywhere.
 */
const resolveLocation = async (): Promise<{ lat: number; lon: number }> => {
	const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const city = timeZone?.split('/').pop()?.replace(/_/g, ' ');

	if (!city) throw new Error(`No se pudo deducir la ciudad de la zona horaria: ${timeZone}`);

	const response = await fetch(
		`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&format=json`
	);
	const data = await response.json();
	const place = data?.results?.[0];

	if (!place) throw new Error(`Sin coordenadas para ${city}`);

	return { lat: place.latitude, lon: place.longitude };
};

const getWeather = async () => {
	const { lat, lon } = await resolveLocation();
	const response = await fetch(
		`https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current=temperature_2m,is_day,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min&timezone=auto`
	);
	return await response.json();
};

const dayOrNight = computed(() => (weather.value?.current?.is_day ? 'day' : 'night'));

/** The forecast after today, flattened so the template doesn't index four
 * parallel arrays by hand. */
const upcoming = computed(() => {
	const daily = weather.value?.daily;
	if (!daily) return [];

	return daily.time.slice(1).map((date: string, index: number) => ({
		date,
		min: daily.temperature_2m_min[index + 1],
		max: daily.temperature_2m_max[index + 1],
		code: daily.weather_code[index + 1],
	}));
});

/**
 * `2026-08-08` parsed by `new Date()` is UTC midnight, so anywhere behind UTC it
 * renders as the previous day. The old code added a day to compensate, which
 * then overshot for anyone ahead of UTC. Building the date from its parts keeps
 * it local in every time zone.
 */
const formatDay = (dateStr: string) => {
	const [year, month, day] = dateStr.split('-').map(Number);
	return new Date(year, month - 1, day).toLocaleDateString(undefined, { weekday: 'long' });
};

onMounted(async () => {
	try {
		weather.value = await getWeather();
	} catch (error) {
		// Offline is the normal case here, not an exception worth shouting about.
		failed.value = true;
		console.warn('No se pudo obtener el clima:', error);
	}
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
          v-for="day in upcoming"
          :key="day.date"
          :date="day.date"
          :min="day.min"
          :max="day.max"
          :units="weather.daily_units"
          :dayOrNight="dayOrNight"
          :weatherCode="day.code"
          class="h-full"
        />
      </div>
    </template>
    <template v-else>
      <div class="col-span-2 flex items-center justify-center text-tx-main/60">
        {{ failed ? t('components.WeatherWidget.failed') : t('components.WeatherWidget.loading') }}
      </div>
    </template>
  </div>
</template>

