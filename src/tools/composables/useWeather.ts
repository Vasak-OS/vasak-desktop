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
import { crearEscuchaDeVisibilidad } from '@/tools/composables/escucha-de-visibilidad';
import { logError } from '@/utils/logger';

/**
 * El clima de todo el escritorio, pedido una sola vez.
 *
 * El estado es del módulo, no del componente: el widget del escritorio y el del
 * panel viven en ventanas distintas, y dentro de cada ventana puede haber más
 * de uno mirando lo mismo. Acá se comparte entre los de la misma ventana; entre
 * ventanas lo comparte el cache de Rust, que es quien decide cuál sale a pedir.
 */
const datos = ref<any>(null);
const fallo = ref(false);
const cargando = ref(false);

/**
 * Cuánto se espera a cada pedido.
 *
 * Sin esto, un pedido que se cuelga —una red que acepta la conexión y después
 * no contesta— deja `cargando` en verdadero para siempre: el turno queda tomado
 * y el clima no se vuelve a pedir en toda la sesión.
 */
const LIMITE = 10_000;

/** Cada cuánto se revisa si lo guardado venció. No toca la red. */
const REVISION = 60_000;

/**
 * Si esta ventana está a la vista.
 *
 * La revisión del minuto no toca la red, pero sí cruza el IPC para preguntarle a
 * Rust si le toca pedir. Hacerlo mientras la ventana está escondida —el panel
 * cerrado, el menú sin abrir— es preguntar por un dato que nadie está mirando: el
 * clima no cambia en un minuto, y al volver a mostrarse se revisa igual.
 */
function aLaVista(): boolean {
	return typeof document === 'undefined' || !document.hidden;
}

let arrancado = false;
let reloj: ReturnType<typeof setInterval> | undefined;
let consumidores = 0;
/**
 * El escucha que refresca al volver la ventana a la vista.
 *
 * Se suelta cuando se desmonta el último consumidor. Con un booleano marcando
 * «ya enganché» no había forma de soltarlo: quedaba vivo para siempre, y un ciclo
 * de esconder y mostrar seguía disparando un IPC —y a veces un pedido a la red—
 * sin que hubiera nadie mirando el clima. Ver `escucha-de-visibilidad.ts`.
 */
const escuchaDeVisibilidad = crearEscuchaDeVisibilidad(
	typeof document === 'undefined' ? undefined : document,
	() => void refrescar()
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
async function deducirLugar(): Promise<WeatherPlace> {
	const guardado = await weatherPlace();
	if (guardado) return guardado;

	const zona = Intl.DateTimeFormat().resolvedOptions().timeZone;
	const ciudad = zona?.split('/').pop()?.replace(/_/g, ' ');

	if (!ciudad) throw new Error(`No se pudo deducir la ciudad de la zona horaria: ${zona}`);

	const respuesta = await fetch(
		`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(ciudad)}&count=1&format=json`,
		{ signal: AbortSignal.timeout(LIMITE) }
	);
	const lugares = await respuesta.json();
	const lugar = lugares?.results?.[0];

	if (!lugar) throw new Error(`Sin coordenadas para ${ciudad}`);

	return { lat: lugar.latitude, lon: lugar.longitude };
}

async function pedirPronostico(lugar: WeatherPlace) {
	const respuesta = await fetch(
		`https://api.open-meteo.com/v1/forecast?latitude=${lugar.lat}&longitude=${lugar.lon}&current=temperature_2m,is_day,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min&timezone=auto`,
		{ signal: AbortSignal.timeout(LIMITE) }
	);

	if (!respuesta.ok) throw new Error(`El servicio del clima contestó ${respuesta.status}`);

	return await respuesta.json();
}

/**
 * Pide el pronóstico sólo si a esta ventana le toca. Si le toca a otra, lo que
 * traiga llega por el evento `weather-updated`.
 */
async function refrescar() {
	if (cargando.value) return;

	try {
		if (!(await weatherClaim())) return;
	} catch (error) {
		// Sin el puente con Rust no hay coordinación posible; pedir igual sería
		// multiplicar los pedidos por la cantidad de ventanas abiertas.
		logError('[clima] No se pudo consultar el turno:', error);
		return;
	}

	cargando.value = true;

	try {
		const lugar = await deducirLugar();
		const pronostico = await pedirPronostico(lugar);

		datos.value = pronostico;
		fallo.value = false;
		await weatherStore(pronostico, lugar);
	} catch (error) {
		// Estar sin red es lo normal acá, no una excepción para gritar.
		if (!datos.value) fallo.value = true;
		console.warn('No se pudo obtener el clima:', error);
		await weatherRelease().catch(() => {});
	} finally {
		cargando.value = false;
	}
}

async function arrancar() {
	if (arrancado) return;
	arrancado = true;

	// Lo guardado primero: si otra ventana ya lo trajo, esta muestra el clima
	// sin pedir nada.
	try {
		const guardado = await weatherCached();
		if (guardado) datos.value = guardado.datos;
	} catch (error) {
		logError('[clima] No se pudo leer el cache:', error);
	}

	// El evento no se desengancha: mientras la ventana viva, lo que traiga
	// cualquier otra tiene que llegar acá.
	listen<WeatherSnapshot>('weather-updated', (evento) => {
		datos.value = evento.payload.datos;
		fallo.value = false;
	}).catch((error) => logError('[clima] No se pudo escuchar las actualizaciones:', error));

	void refrescar();
}

export function useWeather() {
	consumidores += 1;
	void arrancar();

	if (!reloj) {
		reloj = setInterval(() => {
			if (aLaVista()) void refrescar();
		}, REVISION);
	}

	// Al volver a la vista se revisa enseguida, sin esperar hasta un minuto: si
	// estuvo escondida un rato largo, lo guardado puede haber vencido hace mucho.
	escuchaDeVisibilidad.enganchar();

	onUnmounted(() => {
		consumidores -= 1;
		if (consumidores > 0) return;

		if (reloj) {
			clearInterval(reloj);
			reloj = undefined;
		}
		// Y el escucha con él: sin nadie mirando, esconder y mostrar la ventana no
		// tiene que disparar ningún trabajo.
		escuchaDeVisibilidad.soltar();
	});

	const actual = computed(() => datos.value?.current ?? null);
	const dayOrNight = computed<'day' | 'night'>(() => (actual.value?.is_day ? 'day' : 'night'));

	/** El pronóstico de mañana en adelante, aplanado: así la plantilla no
	 * indexa cuatro arreglos paralelos a mano. */
	const proximos = computed(() => {
		const diario = datos.value?.daily;
		if (!diario) return [];

		return diario.time.slice(1).map((fecha: string, i: number) => ({
			date: fecha,
			min: diario.temperature_2m_min[i + 1],
			max: diario.temperature_2m_max[i + 1],
			code: diario.weather_code[i + 1],
		}));
	});

	return {
		weather: datos,
		current: actual,
		failed: computed(() => fallo.value && !datos.value),
		loading: computed(() => cargando.value && !datos.value),
		dayOrNight,
		upcoming: proximos,
		refresh: refrescar,
	};
}
