/**
 * El escaneo de Bluetooth del applet: de la foto con cronómetro al streaming
 * abierto.
 *
 * El síntoma era que «buscar» duraba poco y no encontraba, mientras
 * `bluetoothctl` sí: el botón llamaba a `scanForDevices`, que es una foto con
 * reloj —`startScan` → esperar 10 s → `stopScan` → `listDevices`—, y los BLE
 * que anuncian lento quedaban fuera de esa ventana. Lo que se fija acá es que
 * el applet ya no dependa de esa foto: arranca `startScan` y deja el escaneo
 * ABIERTO, siembra una vez lo conocido, suma los `device-added` en vivo, y sólo
 * corta con `stopScan` al cerrar la vista —nunca por un temporizador—.
 *
 * Se mira el texto del componente porque este repositorio todavía no monta
 * componentes (igual que `radial-view.test.ts`); el agregado en vivo sí se
 * prueba de verdad, sobre `applyBluetoothChange`, que es lo que el componente
 * engancha a `bluetooth-change`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { ref } from 'vue';
import type { BluetoothStateRefs } from '@/interfaces/bluetooth';
import { applyBluetoothChange } from '@/tools/bluetooth.controller';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const AREA = read('src/components/areas/bluetooth/BluetoothControlArea.vue');
const CONNECTIONS = read('src/components/areas/connections/ConnectionsArea.vue');
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;

/** Sólo el cuerpo de `scanDevices`, para no mirar el resto del componente. */
const scanBody = (() => {
	const start = AREA.indexOf('const scanDevices = async');
	const end = AREA.indexOf('const stopScanning = async');
	return AREA.slice(start, end);
})();

describe('el escaneo es streaming abierto, no una foto con cronómetro', () => {
	test('ya no se usa el ayudante con temporizador', () => {
		// `scanForDevices` es el que mete el `setTimeout(10000)`: si vuelve, vuelve
		// el corte corto.
		expect(AREA).not.toContain('scanForDevices');
	});

	test('`scanDevices` arranca `startScan` y no corta por tiempo', () => {
		expect(scanBody).toContain('await startScan(defaultAdapter.value.path)');
		// Nada de cronómetros ni de cerrar el escaneo dentro del propio escaneo.
		expect(scanBody).not.toContain('setTimeout');
		expect(scanBody).not.toContain('stopScan');
	});

	test('siembra lo conocido una vez antes de dejar abierto', () => {
		// La siembra va antes de `startScan` para que los `device-added` del
		// streaming sólo sumen encima de lo ya sembrado.
		const seed = scanBody.indexOf('await refreshDevices()');
		const open = scanBody.indexOf('await startScan(');
		expect(seed).toBeGreaterThanOrEqual(0);
		expect(open).toBeGreaterThan(seed);
	});

	test('`startScan` y `stopScan` salen del complemento, `scanForDevices` no', () => {
		expect(AREA).toMatch(
			/import \{[^}]*\bstartScan\b[^}]*\bstopScan\b[^}]*\} from '@vasakgroup\/plugin-bluetooth-manager'/s
		);
	});

	test('al desmontar la vista se cierra el escaneo con `stopScan`', () => {
		expect(AREA).toContain('onUnmounted(');
		expect(AREA).toMatch(/onUnmounted\(\(\) => \{\s*void stopScanning\(\);\s*\}\)/);
		expect(AREA).toMatch(/const stopScanning = async[\s\S]*?await stopScan\(defaultAdapter\.value\.path\)/);
	});

	test('mientras busca, la bandera `scanning` y el indicador de la librería', () => {
		expect(AREA).toContain('const scanning = ref(false)');
		// La rueda del botón y el aro de espera de la librería, ambos atados a la
		// misma bandera.
		expect(AREA).toContain(':loading="scanning"');
		expect(AREA).toMatch(/<LoadingState[\s\S]*?v-if="scanning"/);
		expect(AREA).toContain("t('components.BluetoothControlArea.scanning')");
	});

	test('el indicador se apaga si se apaga el Bluetooth, sin rueda colgada', () => {
		expect(AREA).toMatch(/watch\(isBluetoothOn[\s\S]*?scanning\.value = false/);
	});
});

describe('el botón «Buscar» de la vista radial sigue arrancando el escaneo', () => {
	test('la lista abre buscando cuando se llegó desde «Buscar»', () => {
		expect(CONNECTIONS).toContain(':scan-on-mount="scanOnOpen"');
	});

	test('`scanOnMount` dispara el nuevo `scanDevices` al montar', () => {
		expect(AREA).toMatch(/props\.scanOnMount[\s\S]*?await scanDevices\(\)/);
	});
});

describe('los `device-added` suman en vivo mientras el escaneo está abierto', () => {
	test('el applet sigue suscrito a `bluetooth-change` y lo enruta a `applyBluetoothChange`', () => {
		// El escaneo abierto no sirve de nada si el applet deja de escuchar el
		// evento: sin esta suscripción —y sin que el manejador reenvíe al
		// controlador— los `device-added` del streaming nunca llegan a la lista,
		// y el agregado en vivo que se prueba abajo quedaría colgado del vacío.
		expect(AREA).toContain("useSharedEvent('bluetooth-change', handleBluetoothChange)");
		expect(AREA).toMatch(/const handleBluetoothChange = async[\s\S]*?applyBluetoothChange\(/);
	});

	const state = (): BluetoothStateRefs => ({
		availableDevices: ref<any[]>([]),
		connectedDevices: ref<any[]>([]),
		defaultAdapter: ref<any>({ path: '/org/bluez/hci0', powered: true }),
	});

	test('un dispositivo anunciado después de abrir aparece en disponibles', () => {
		const s = state();
		// Ya había uno sembrado; el streaming anuncia otro más tarde.
		s.availableDevices.value = [{ path: '/dev_seed', connected: false }];
		applyBluetoothChange(
			{ changeType: 'device-added', data: { path: '/dev_late', connected: false } },
			s
		);

		expect(s.availableDevices.value.map((d) => d.path)).toEqual(['/dev_seed', '/dev_late']);
	});

	test('uno ya conectado anunciado en vivo cae en conectados', () => {
		const s = state();
		applyBluetoothChange(
			{ changeType: 'device-added', data: { path: '/dev_x', connected: true } },
			s
		);

		expect(s.connectedDevices.value.map((d) => d.path)).toEqual(['/dev_x']);
		expect(s.availableDevices.value).toEqual([]);
	});
});

describe('los textos nuevos están en los dos idiomas', () => {
	test('la clave `scanning` existe en español y en inglés', () => {
		expect(ES.components.BluetoothControlArea.scanning).toBeString();
		expect(EN.components.BluetoothControlArea.scanning).toBeString();
	});
});
