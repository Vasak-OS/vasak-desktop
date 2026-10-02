/**
 * La vista radial del Bluetooth y de la red (vasak-desktop#132), cableada.
 *
 * Qué satélites se arman y en qué orden lo prueba `connection-orbit.test.ts`;
 * cómo los dibuja la órbita, montada, vue-libvasak (`DeviceOrbit`). Lo que se
 * fija acá es lo que se separa sin avisar: que la órbita sea la de la librería
 * y no un dibujo propio, que la lista siga estando (la vista radial se suma, no
 * la reemplaza), que el perfil de audio salga del escritorio y no de un segundo
 * camino a PipeWire, y que cada texto esté en los dos idiomas.
 *
 * Se mira el texto porque este repositorio todavía no monta componentes, igual
 * que `music-applet.test.ts`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const AREA = read('src/components/areas/connections/ConnectionsArea.vue');
const HELPERS = read('src/tools/connection-orbit.ts');
const BLUETOOTH_LIST = read('src/components/areas/bluetooth/BluetoothControlArea.vue');
const LIB = read('src-tauri/src/lib.rs');
const AUDIO_APPLET = read('src-tauri/src/applets/audio.rs');
const PW_DUMP = read('src-tauri/src/audio_native.rs');
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;
const PACKAGE = JSON.parse(read('package.json')) as { dependencies: Record<string, string> };

describe('la vista radial', () => {
	test('los dos applets abren en ella, cada uno con su pestaña', () => {
		expect(read('src/views/applets/BluetoothAppletView.vue')).toContain(
			'<ConnectionsArea initial-tab="bluetooth" applet="bluetooth" />'
		);
		expect(read('src/views/applets/NetworkAppletView.vue')).toContain(
			'<ConnectionsArea initial-tab="wifi" applet="network" />'
		);
	});

	test('la órbita, el selector y el encendido son de la librería', () => {
		expect(AREA).toMatch(
			/import \{[^}]*DeviceOrbit[^}]*SegmentedControl[^}]*SwitchToggle[^}]*\} from '@vasakgroup\/vue-libvasak'/
		);
		expect(AREA).toContain('<DeviceOrbit');
		expect(AREA).toContain('<SegmentedControl');
		expect(PACKAGE.dependencies['@vasakgroup/vue-libvasak']).toBe('^2.6.0');
		// Ni líneas ni círculos propios: los dibuja la órbita.
		expect(AREA).not.toMatch(/<svg[\s>]/);
		expect(AREA).not.toMatch(/rounded-corner-full/);
	});

	test('la lista sigue estando: la vista radial se suma, no la reemplaza', () => {
		expect(AREA).toContain('<BluetoothControlArea');
		expect(AREA).toContain('<NetworkControlArea');
		// Se pasa a la lista desde el satélite de arriba, y se vuelve.
		expect(AREA).toMatch(/id === 'scan'[\s\S]*?view\.value = 'list'/);
		expect(AREA).toMatch(/id === 'networks'[\s\S]*?view\.value = 'list'/);
		expect(AREA).toContain('@click="backToOrbit"');
	});

	test('«Buscar dispositivos» llega a la lista buscando', () => {
		expect(AREA).toContain(':scan-on-mount="scanOnOpen"');
		expect(BLUETOOTH_LIST).toMatch(/props\.scanOnMount[\s\S]*?scanDevices\(\)/);
	});

	test('al volver a mostrarse el applet, vuelve a la órbita y pide todo de nuevo', () => {
		expect(AREA).toMatch(/useSharedEvent<[^>]*>\('applet-shown'[\s\S]*?view\.value = 'radial'[\s\S]*?refreshAll\(\)/);
	});

	test('el pie pasa a dos renglones por contenedor, no por la pantalla', () => {
		expect(AREA).toContain('@container');
		expect(AREA).toContain('@md:grid-cols-[1fr_auto_1fr]');
		expect(AREA).not.toMatch(/(?<![\w@-])(?:sm|md|lg|xl):[a-z]/);
	});
});

describe('los datos', () => {
	test('Bluetooth y red salen de los complementos que ya usa el escritorio', () => {
		expect(AREA).toContain("from '@vasakgroup/plugin-bluetooth-manager'");
		expect(AREA).toContain("from '@/services/network.service'");
		expect(HELPERS).toContain('device.battery');
	});

	test('el perfil de audio lo da el escritorio, leído del pw-dump que ya corre', () => {
		expect(AREA).toContain("invoke<BluetoothAudioProfile | null>('get_bluetooth_audio_profile'");
		expect(AREA).toContain("'bluetooth-audio-profile-changed'");
		expect(LIB).toContain('bluetooth_audio_profile::get_bluetooth_audio_profile');
		expect(AUDIO_APPLET).toContain('"bluetooth-audio-profile-changed"');
		expect(PW_DUMP).toContain('crate::bluetooth_audio_profile::ingest(&batch)');
		// Un solo `pw-dump`: el perfil no abre otro proceso.
		expect(PW_DUMP.match(/Command::new\("pw-dump"\)/g)).toHaveLength(1);
	});
});

describe('los textos', () => {
	test('cada clave que usa la vista está en español y en inglés', () => {
		const keys = new Set(
			[...`${AREA}\n${HELPERS}`.matchAll(/(?:components\.ConnectionsArea\.|\$\{KEY\}\.)([A-Za-z0-9]+)/g)].map(
				(match) => match[1] as string
			)
		);
		expect(keys.size).toBeGreaterThan(20);
		for (const key of keys) {
			expect(ES.components.ConnectionsArea[key], `es: ${key}`).toBeString();
			expect(EN.components.ConnectionsArea[key], `en: ${key}`).toBeString();
		}
	});
});
