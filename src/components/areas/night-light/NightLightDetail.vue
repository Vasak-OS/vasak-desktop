<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El detalle de la luz nocturna, dentro del bloque de ajustes del centro de
 * control (vasak-desktop#178): qué tan cálida y cuándo —al atardecer según la
 * ubicación, o con un horario fijo—.
 *
 * La configuración la lee y la guarda `@vasakgroup/plugin-display-manager`,
 * el mismo que usa Configuración, así que lo que se elige acá se ve allá. Cada
 * cambio se guarda al soltarlo (el deslizador y los campos son «lazy») y, si
 * la luz está encendida, el escritorio reinicia `wlsunset` para que lo tome.
 *
 * Una columna sola: a 240 px no se corta nada; las dos horas y las dos
 * coordenadas se ponen lado a lado sólo si el bloque es ancho (consulta de
 * contenedor).
 */
import {
	getNightLight,
	type NightLight,
	type NightLightConfig,
	type NightLightMode,
	setNightLight,
} from '@vasakgroup/plugin-display-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	AlertMessage,
	FormGroup,
	SegmentedControl,
	type SegmentedOption,
	Slider,
	TextInput,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { applyNightLight } from '@/services/night-light.service';
import { weatherPlace } from '@/services/weather.service';
import {
	formatCoordinate,
	MIN_TEMPERATURE,
	maxNightTemperature,
	parseCoordinate,
	TEMPERATURE_STEP,
} from '@/tools/night-light-form';
import { createSerialQueue } from '@/tools/serial-queue';

const { t } = useI18n();

const nightLight = ref<NightLight | null>(null);
const latitude = ref('');
const longitude = ref('');
const saving = ref(false);
const error = ref('');
/** Los guardados de a uno: dos cambios seguidos no se pisan al volver. */
const queue = createSerialQueue();

const config = computed(() => nightLight.value?.config ?? null);
const isLocation = computed(() => config.value?.mode === 'location');
const maxNight = computed(() => maxNightTemperature(config.value?.dayTemperature ?? 6500));

const modes = computed<SegmentedOption<NightLightMode>[]>(() => [
	{ value: 'location', label: t('components.NightLightDetail.atSunset') },
	{ value: 'manual', label: t('components.NightLightDetail.manual') },
]);

const kelvin = (value: number) =>
	t('components.NightLightDetail.kelvin').replace('{0}', String(value));

function show(next: NightLight): void {
	nightLight.value = next;
	latitude.value = formatCoordinate(next.config.latitude);
	longitude.value = formatCoordinate(next.config.longitude);
}

onMounted(async () => {
	try {
		show(await getNightLight());
	} catch (cause) {
		console.error('[night-light] no se pudo leer la configuración:', cause);
		error.value = t('components.NightLightDetail.loadFailed');
	}
});

/** Guarda un cambio con el plugin y lo aplica si la luz está encendida. */
function save(patch: Partial<NightLightConfig>): Promise<void> {
	return queue(async () => {
		if (!nightLight.value) return;
		saving.value = true;
		error.value = '';
		try {
			show(await setNightLight({ ...nightLight.value.config, ...patch }));
			await applyNightLight();
		} catch (cause) {
			console.error('[night-light] no se pudo guardar:', cause);
			error.value = t('components.NightLightDetail.saveFailed');
		} finally {
			saving.value = false;
		}
	});
}

function onTemperature(value: number): void {
	void save({ nightTemperature: value });
}

/**
 * Al pasar a «al atardecer» sin coordenadas guardadas se toman las del clima,
 * si el widget ya las averiguó: es la misma ubicación, y así no hay que
 * escribirlas.
 */
async function onMode(mode: NightLightMode | null): Promise<void> {
	if (!mode || !config.value || mode === config.value.mode) return;
	const patch: Partial<NightLightConfig> = { mode };
	if (mode === 'location' && (config.value.latitude === null || config.value.longitude === null)) {
		try {
			const place = await weatherPlace();
			if (place) {
				patch.latitude = place.lat;
				patch.longitude = place.lon;
			}
		} catch {
			// Sin clima no hay de dónde sacarlas: quedan los campos para escribirlas.
		}
	}
	await save(patch);
}

function onTime(key: 'sunrise' | 'sunset', value: string): void {
	if (value) void save({ [key]: value });
}

function onCoordinates(): void {
	const lat = parseCoordinate(latitude.value, 90);
	const lon = parseCoordinate(longitude.value, 180);
	if (lat === undefined || lon === undefined) {
		error.value = t('components.NightLightDetail.invalidCoordinates');
		return;
	}
	void save({ latitude: lat, longitude: lon });
}
</script>

<template>
  <section class="@container flex flex-col gap-3 p-1" data-night-light-detail>
    <AlertMessage v-if="nightLight && !nightLight.available" tone="warning" data-night-light-missing>
      {{ t('components.NightLightDetail.missing') }}
    </AlertMessage>
    <AlertMessage v-if="error" tone="error" data-night-light-error>{{ error }}</AlertMessage>

    <template v-if="config">
      <FormGroup :label="t('components.NightLightDetail.temperature')">
        <div class="flex min-w-0 items-center gap-3">
          <Slider
            class="min-w-0 flex-1"
            :label="t('components.NightLightDetail.temperature')"
            :model-value="config.nightTemperature"
            :min="MIN_TEMPERATURE"
            :max="maxNight"
            :step="TEMPERATURE_STEP"
            :value-text="kelvin"
            :disabled="!nightLight?.available"
            lazy
            data-night-light-temperature
            @update:model-value="onTemperature"
          />
          <span class="shrink-0 text-sm tabular-nums text-tx-muted">{{ kelvin(config.nightTemperature) }}</span>
        </div>
      </FormGroup>

      <FormGroup :label="t('components.NightLightDetail.schedule')">
        <SegmentedControl
          :label="t('components.NightLightDetail.schedule')"
          :options="modes"
          :model-value="config.mode"
          :disabled="!nightLight?.available"
          data-night-light-mode
          @update:model-value="onMode"
        />
      </FormGroup>

      <div v-if="isLocation" class="grid grid-cols-1 gap-3 @[16rem]:grid-cols-2" data-night-light-location>
        <FormGroup :label="t('components.NightLightDetail.latitude')">
          <template #default="{ id }">
            <TextInput
              :id="id"
              v-model="latitude"
              placeholder="-34.60"
              :spellcheck="false"
              :disabled="!nightLight?.available"
              lazy
              data-night-light-latitude
              @update:model-value="onCoordinates"
            />
          </template>
        </FormGroup>
        <FormGroup :label="t('components.NightLightDetail.longitude')">
          <template #default="{ id }">
            <TextInput
              :id="id"
              v-model="longitude"
              placeholder="-58.38"
              :spellcheck="false"
              :disabled="!nightLight?.available"
              lazy
              data-night-light-longitude
              @update:model-value="onCoordinates"
            />
          </template>
        </FormGroup>
        <p class="m-0 text-xs text-tx-muted @[16rem]:col-span-2">
          {{ t('components.NightLightDetail.locationHint') }}
        </p>
      </div>
      <div v-else class="grid grid-cols-1 gap-3 @[16rem]:grid-cols-2" data-night-light-times>
        <FormGroup :label="t('components.NightLightDetail.from')">
          <template #default="{ id }">
            <TextInput
              :id="id"
              type="time"
              :model-value="config.sunset"
              :disabled="!nightLight?.available"
              lazy
              data-night-light-sunset
              @update:model-value="onTime('sunset', $event)"
            />
          </template>
        </FormGroup>
        <FormGroup :label="t('components.NightLightDetail.until')">
          <template #default="{ id }">
            <TextInput
              :id="id"
              type="time"
              :model-value="config.sunrise"
              :disabled="!nightLight?.available"
              lazy
              data-night-light-sunrise
              @update:model-value="onTime('sunrise', $event)"
            />
          </template>
        </FormGroup>
      </div>
      <p v-if="saving" class="m-0 text-xs text-tx-muted" aria-live="polite" data-night-light-saving>
        {{ t('components.NightLightDetail.saving') }}
      </p>
    </template>
  </section>
</template>
