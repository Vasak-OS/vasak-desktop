<script lang="ts" setup>
import { onMounted, Ref, ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';

type WeatherPeriod = {
  description: string;
  image: string;
};

type WeatherInfo = {
  day: WeatherPeriod;
  night: WeatherPeriod;
};

type CodeDataType = {
  [key: string]: WeatherInfo;
};

const codeData: CodeDataType = {
	'0': {
		day: {
			description: 'Sunny',
			image: 'weather-clear',
		},
		night: {
			description: 'Clear',
			image: 'weather-clear-night',
		},
	},
	'1': {
		day: {
			description: 'Mainly Sunny',
			image: 'weather-clear',
		},
		night: {
			description: 'Mainly Clear',
			image: 'weather-clear-night',
		},
	},
	'2': {
		day: {
			description: 'Partly Cloudy',
			image: 'weather-few-clouds',
		},
		night: {
			description: 'Partly Cloudy',
			image: 'weather-few-clouds-night',
		},
	},
	'3': {
		day: {
			description: 'Cloudy',
			image: 'weather-overcast',
		},
		night: {
			description: 'Cloudy',
			image: 'weather-overcast',
		},
	},
	'45': {
		day: {
			description: 'Foggy',
			image: 'weather-fog',
		},
		night: {
			description: 'Foggy',
			image: 'weather-fog',
		},
	},
	'48': {
		day: {
			description: 'Rime Fog',
			image: 'weather-fog',
		},
		night: {
			description: 'Rime Fog',
			image: 'weather-fog',
		},
	},
	'51': {
		day: {
			description: 'Light Drizzle',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Light Drizzle',
			image: 'weather-showers-scattered',
		},
	},
	'53': {
		day: {
			description: 'Drizzle',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Drizzle',
			image: 'weather-showers-scattered',
		},
	},
	'55': {
		day: {
			description: 'Heavy Drizzle',
			image: 'weather-showers',
		},
		night: {
			description: 'Heavy Drizzle',
			image: 'weather-showers',
		},
	},
	'56': {
		day: {
			description: 'Light Freezing Drizzle',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Light Freezing Drizzle',
			image: 'weather-showers-scattered',
		},
	},
	'57': {
		day: {
			description: 'Freezing Drizzle',
			image: 'weather-showers',
		},
		night: {
			description: 'Freezing Drizzle',
			image: 'weather-showers',
		},
	},
	'61': {
		day: {
			description: 'Light Rain',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Light Rain',
			image: 'weather-showers-scattered',
		},
	},
	'63': {
		day: {
			description: 'Rain',
			image: 'weather-showers',
		},
		night: {
			description: 'Rain',
			image: 'weather-showers',
		},
	},
	'65': {
		day: {
			description: 'Heavy Rain',
			image: 'weather-showers',
		},
		night: {
			description: 'Heavy Rain',
			image: 'weather-showers',
		},
	},
	'66': {
		day: {
			description: 'Light Freezing Rain',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Light Freezing Rain',
			image: 'weather-showers-scattered',
		},
	},
	'67': {
		day: {
			description: 'Freezing Rain',
			image: 'weather-showers',
		},
		night: {
			description: 'Freezing Rain',
			image: 'weather-showers',
		},
	},
	'71': {
		day: {
			description: 'Light Snow',
			image: 'weather-snow',
		},
		night: {
			description: 'Light Snow',
			image: 'weather-snow',
		},
	},
	'73': {
		day: {
			description: 'Snow',
			image: 'weather-snow',
		},
		night: {
			description: 'Snow',
			image: 'weather-snow',
		},
	},
	'75': {
		day: {
			description: 'Heavy Snow',
			image: 'weather-snow',
		},
		night: {
			description: 'Heavy Snow',
			image: 'weather-snow',
		},
	},
	'77': {
		day: {
			description: 'Snow Grains',
			image: 'weather-snow',
		},
		night: {
			description: 'Snow Grains',
			image: 'weather-snow',
		},
	},
	'80': {
		day: {
			description: 'Light Showers',
			image: 'weather-showers-scattered',
		},
		night: {
			description: 'Light Showers',
			image: 'weather-showers-scattered',
		},
	},
	'81': {
		day: {
			description: 'Showers',
			image: 'weather-showers',
		},
		night: {
			description: 'Showers',
			image: 'weather-showers',
		},
	},
	'82': {
		day: {
			description: 'Heavy Showers',
			image: 'weather-showers',
		},
		night: {
			description: 'Heavy Showers',
			image: 'weather-showers',
		},
	},
	'85': {
		day: {
			description: 'Light Snow Showers',
			image: 'weather-snow',
		},
		night: {
			description: 'Light Snow Showers',
			image: 'weather-snow',
		},
	},
	'86': {
		day: {
			description: 'Snow Showers',
			image: 'weather-snow',
		},
		night: {
			description: 'Snow Showers',
			image: 'weather-snow',
		},
	},
	'95': {
		day: {
			description: 'Thunderstorm',
			image: 'weather-storm',
		},
		night: {
			description: 'Thunderstorm',
			image: 'weather-storm',
		},
	},
	'96': {
		day: {
			description: 'Light Thunderstorms With Hail',
			image: 'weather-storm',
		},
		night: {
			description: 'Light Thunderstorms With Hail',
			image: 'weather-storm',
		},
	},
	'99': {
		day: {
			description: 'Thunderstorm With Hail',
			image: 'weather-storm',
		},
		night: {
			description: 'Thunderstorm With Hail',
			image: 'weather-storm',
		},
	},
};

const iconPath: Ref<string> = ref('');
const weatherInfo: Ref<WeatherInfo | null> = ref(null);
const props = defineProps<{
  code: number;
  dayOrNight: 'day' | 'night';
}>();

onMounted(async () => {
	weatherInfo.value = codeData[String(props.code)];
	if (weatherInfo.value) {
		iconPath.value = await getIconSource(weatherInfo.value[props.dayOrNight].image);
	} else {
		iconPath.value = 'weather-severe-alert'; // Default icon for unknown codes
	}
});
</script>

<template>
  <transition name="icon-fade" mode="out-in">
	<img
	  v-if="iconPath"
	  :src="iconPath"
	  :alt="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  :title="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  class="img-fluid weather-icon-img h-24"
	/>
  </transition>
</template>

<style scoped>
.weather-icon-img {
  transition: transform 0.3s ease;
}

/* Transición para cambio de iconos del clima */
.icon-fade-enter-active,
.icon-fade-leave-active {
  transition: opacity 0.3s ease;
}

.icon-fade-enter-from,
.icon-fade-leave-to {
  opacity: 0;
}
</style>
