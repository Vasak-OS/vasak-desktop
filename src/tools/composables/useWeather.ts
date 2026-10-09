import { listen } from '@tauri-apps/api/event';
import { computed, onUnmounted, ref } from 'vue';
import {
	type WeatherPlace,
	type WeatherSnapshot,
	weatherCached,
	weatherClaim,
	weatherPlace,
	weatherRelease,
	weatherStore,
} from '@/services/weather.service';
import { createVisibilityListener } from '@/tools/composables/visibility-listener';
import { forecastUrl } from '@/tools/forecast-url';

/**
 * El clima de todo el escritorio, pedido una sola vez.
 *
 * Los errores van por `console`, que el logger del escritorio ya recoge:
 * importar el logger acá ataba el clima a una ventana (`window`) y no dejaba
 * probarlo.
 *
 * El estado es del módulo, no del componente: el widget del escritorio, el del
 * panel y el tablero de fecha viven en ventanas distintas, y dentro de cada
 * ventana puede haber más de uno mirando lo mismo. Acá se comparte entre los de
 * la misma ventana; entre ventanas lo comparte el cache de Rust, que es quien
 * decide cuál sale a pedir.
 */
const data = ref<any>(null);
const failed = ref(false);
const loading = ref(false);

/**
 * Cuánto se espera a cada pedido.
 *
 * Sin esto, un pedido que se cuelga —una red que acepta la conexión y después
 * no contesta— deja `loading` en verdadero para siempre: el turno queda tomado
 * y el clima no se vuelve a pedir en toda la sesión.
 */
const REQUEST_TIMEOUT = 10_000;

/** Cada cuánto se revisa si lo guardado venció. No toca la red. */
const CHECK_INTERVAL = 60_000;

/**
 * Si esta ventana está a la vista.
 *
 * La revisión del minuto no toca la red, pero sí cruza el IPC para preguntarle a
 * Rust si le toca pedir. Hacerlo mientras la ventana está escondida —el panel
 * cerrado, el menú sin abrir— es preguntar por un dato que nadie está mirando: el
 * clima no cambia en un minuto, y al volver a mostrarse se revisa igual.
 */
function isVisible(): boolean {
	return typeof document === 'undefined' || !document.hidden;
}

let started = false;
let timer: ReturnType<typeof setInterval> | undefined;
let consumers = 0;
/**
 * El escucha que refresca al volver la ventana a la vista.
 *
 * Se suelta cuando se desmonta el último consumidor. Con un booleano marcando
 * «ya enganché» no había forma de soltarlo: quedaba vivo para siempre, y un ciclo
 * de esconder y mostrar seguía disparando un IPC —y a veces un pedido a la red—
 * sin que hubiera nadie mirando el clima. Ver `visibility-listener.ts`.
 */
const visibility = createVisibilityListener(
	typeof document === 'undefined' ? undefined : document,
	() => void refresh()
);

/**
 * Coordenadas a partir de la zona horaria.
 *
 * Antes esto mandaba la IP del usuario a `http://ip-api.com` —en texto plano, a
 * un tercero, cada vez que se abría el menú—. La zona horaria ya nombra una
 * ciudad cercana y es información local, así que geocodificarla contra el mismo
 * proveedor del pronóstico no agrega ningún tercero y no manda la IP a ninguna
 * parte.
 */
async function guessPlace(): Promise<WeatherPlace> {
	const saved = await weatherPlace();
	if (saved) return saved;

	const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const city = zone?.split('/').pop()?.replaceAll('_', ' ');

	if (!city) throw new Error(`No se pudo deducir la ciudad de la zona horaria: ${zone}`);

	const response = await fetch(
		`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&format=json`,
		{ signal: AbortSignal.timeout(REQUEST_TIMEOUT) }
	);
	const places = await response.json();
	const place = places?.results?.[0];

	if (!place) throw new Error(`Sin coordenadas para ${city}`);

	return { lat: place.latitude, lon: place.longitude };
}

async function fetchForecast(place: WeatherPlace) {
	const response = await fetch(forecastUrl(place), {
		signal: AbortSignal.timeout(REQUEST_TIMEOUT),
	});

	if (!response.ok) throw new Error(`El servicio del clima contestó ${response.status}`);

	return await response.json();
}

/**
 * Pide el pronóstico sólo si a esta ventana le toca. Si le toca a otra, lo que
 * traiga llega por el evento `weather-updated`.
 */
async function refresh() {
	if (loading.value) return;

	try {
		if (!(await weatherClaim())) return;
	} catch (error) {
		// Sin el puente con Rust no hay coordinación posible; pedir igual sería
		// multiplicar los pedidos por la cantidad de ventanas abiertas.
		console.error('[clima] No se pudo consultar el turno:', error);
		return;
	}

	loading.value = true;

	try {
		const place = await guessPlace();
		const forecast = await fetchForecast(place);

		data.value = forecast;
		failed.value = false;
		await weatherStore(forecast, place);
	} catch (error) {
		// Estar sin red es lo normal acá, no una excepción para gritar.
		if (!data.value) failed.value = true;
		console.warn('No se pudo obtener el clima:', error);
		await weatherRelease().catch(() => {});
	} finally {
		loading.value = false;
	}
}

async function start() {
	if (started) return;
	started = true;

	// Lo guardado primero: si otra ventana ya lo trajo, esta muestra el clima
	// sin pedir nada.
	try {
		const saved = await weatherCached();
		if (saved) data.value = saved.datos;
	} catch (error) {
		console.error('[clima] No se pudo leer el cache:', error);
	}

	// El evento no se desengancha: mientras la ventana viva, lo que traiga
	// cualquier otra tiene que llegar acá.
	listen<WeatherSnapshot>('weather-updated', (event) => {
		data.value = event.payload.datos;
		failed.value = false;
	}).catch((error) => console.error('[clima] No se pudo escuchar las actualizaciones:', error));

	void refresh();
}

export function useWeather() {
	consumers += 1;
	void start();

	if (!timer) {
		timer = setInterval(() => {
			if (isVisible()) void refresh();
		}, CHECK_INTERVAL);
	}

	// Al volver a la vista se revisa enseguida, sin esperar hasta un minuto: si
	// estuvo escondida un rato largo, lo guardado puede haber vencido hace mucho.
	visibility.attach();

	onUnmounted(() => {
		consumers -= 1;
		if (consumers > 0) return;

		if (timer) {
			clearInterval(timer);
			timer = undefined;
		}
		// Y el escucha con él: sin nadie mirando, esconder y mostrar la ventana no
		// tiene que disparar ningún trabajo.
		visibility.detach();
	});

	const current = computed(() => data.value?.current ?? null);
	const dayOrNight = computed<'day' | 'night'>(() => (current.value?.is_day ? 'day' : 'night'));

	/** El pronóstico de mañana en adelante, aplanado: así la plantilla no
	 * indexa cuatro arreglos paralelos a mano. */
	const upcoming = computed(() => {
		const daily = data.value?.daily;
		if (!daily) return [];

		return daily.time.slice(1).map((date: string, i: number) => ({
			date,
			min: daily.temperature_2m_min[i + 1],
			max: daily.temperature_2m_max[i + 1],
			code: daily.weather_code[i + 1],
		}));
	});

	return {
		weather: data,
		current,
		failed: computed(() => failed.value && !data.value),
		loading: computed(() => loading.value && !data.value),
		dayOrNight,
		upcoming,
		refresh,
	};
}
