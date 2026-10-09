/**
 * Lo que la vista radial del Bluetooth y de la red le pasa a `DeviceOrbit`
 * (vasak-desktop#132).
 *
 * Lo que se mira: el orden de los satélites —que es su lugar en la órbita—, que
 * un dato que falta vaya vacío para que la órbita no lo dibuje (la batería que
 * BlueZ no publica no es «0 %»), el estado sin conexión, y los nombres
 * legibles del perfil de audio y de la seguridad.
 */
import { describe, expect, test } from 'bun:test';
import type { DeviceInfo } from '@vasakgroup/plugin-bluetooth-manager';
import type { NetworkInfo } from '@/services/network.service';
import {
	audioProfileLabel,
	batteryIconName,
	bluetoothCenter,
	bluetoothSatellites,
	isWifi,
	networkCenter,
	networkSatellites,
	securityLabel,
	signalIconName,
} from './connection-orbit';

/** Devuelve la clave: así se ve qué texto se pidió. */
const t = (key: string) => key.replace('components.ConnectionsArea.', '');

const HEADPHONES: DeviceInfo = {
	path: '/org/bluez/hci0/dev_08_92_CC_7C_37_A1',
	address: '08:92:CC:7C:37:A1',
	name: 'JBL Tune 720BT',
	alias: 'JBL Tune 720BT',
	icon: 'audio-headphones',
	paired: true,
	trusted: true,
	blocked: false,
	legacyPairing: false,
	connected: true,
	uuids: [],
	adapter: '/org/bluez/hci0',
	servicesResolved: true,
	battery: 70,
};

const A2DP = {
	address: '08:92:cc:7c:37:a1',
	profile: 'a2dp-sink',
	description: 'High Fidelity Playback (A2DP Sink, codec AAC)',
	codec: 'AAC',
};

const WIFI: NetworkInfo = {
	name: 'Fibernet-IA',
	ssid: 'Fibernet-IA',
	connection_type: 'wifi',
	icon: 'network-wireless-signal-good-symbolic',
	ip_address: '192.0.2.42',
	mac_address: '00:11:22:33:44:55',
	signal_strength: 78,
	security_type: 'wpa2-psk',
	is_connected: true,
};

const ids = (satellites: { id: string }[]) => satellites.map((satellite) => satellite.id);
const satelliteValue = (satellites: { id: string; value?: string | null }[], id: string) =>
	satellites.find((satellite) => satellite.id === id)?.value;

describe('la órbita de Bluetooth', () => {
	test('el dispositivo al centro, con su icono, su nombre y «Conectado»', () => {
		expect(bluetoothCenter(HEADPHONES, t)).toEqual({
			icon: 'audio-headphones',
			iconType: 'symbol',
			title: 'JBL Tune 720BT',
			subtitle: 'connected',
		});
		// Sin nombre ni icono: la MAC y el icono de Bluetooth.
		const bare = { ...HEADPHONES, name: undefined, alias: undefined, icon: undefined };
		expect(bluetoothCenter(bare, t)).toMatchObject({
			icon: 'bluetooth',
			title: HEADPHONES.address,
		});
	});

	test('cuatro satélites en el orden de la referencia: buscar, batería, perfil, MAC', () => {
		const satellites = bluetoothSatellites(HEADPHONES, A2DP, t);

		expect(ids(satellites)).toEqual(['scan', 'battery', 'profile', 'mac']);
		expect(satellites[0]).toMatchObject({ action: true, value: 'scan', label: 'switchView' });
		expect(satelliteValue(satellites, 'battery')).toBe('70 %');
		expect(satelliteValue(satellites, 'profile')).toBe('profileA2dp · AAC');
		expect(satelliteValue(satellites, 'mac')).toBe('08:92:CC:7C:37:A1');
	});

	test('sin batería publicada, el satélite va vacío y no «0 %»', () => {
		const satellites = bluetoothSatellites({ ...HEADPHONES, battery: undefined }, A2DP, t);
		const battery = satellites.find((satellite) => satellite.id === 'battery');

		expect(battery?.value).toBeNull();
		expect(battery?.icon).toBeUndefined();
	});

	test('una batería en cero sí es un dato: «0 %», en el acento', () => {
		const battery = bluetoothSatellites({ ...HEADPHONES, battery: 0 }, null, t).find(
			(s) => s.id === 'battery'
		);

		expect(battery).toMatchObject({ value: '0 %', tone: 'accent', icon: 'battery-level-0' });
	});

	test('por debajo de 20 % el valor va en el acento; de 20 para arriba, no', () => {
		const tone = (battery: number) =>
			bluetoothSatellites({ ...HEADPHONES, battery }, null, t).find((s) => s.id === 'battery')
				?.tone;

		expect(tone(19)).toBe('accent');
		expect(tone(20)).toBe('default');
	});

	test('sin perfil conocido, o con el de otro dispositivo, el satélite va vacío', () => {
		expect(satelliteValue(bluetoothSatellites(HEADPHONES, null, t), 'profile')).toBe('');
		const other = { ...A2DP, address: 'AA:BB:CC:DD:EE:FF' };
		expect(satelliteValue(bluetoothSatellites(HEADPHONES, other, t), 'profile')).toBe('');
	});

	test('sin nada conectado: el centro vacío y sólo la acción de buscar', () => {
		expect(bluetoothCenter(null, t)).toBeNull();
		expect(ids(bluetoothSatellites(null, A2DP, t))).toEqual(['scan']);
	});

	test('mientras busca lo dice, y con la radio apagada la acción no se puede usar', () => {
		const [scan] = bluetoothSatellites(null, null, t, { scanning: true, disabled: true });

		expect(scan).toMatchObject({ value: 'scanning', disabled: true });
	});

	test('el icono de la batería se llena de diez en diez', () => {
		expect(batteryIconName(70)).toBe('battery-level-70');
		expect(batteryIconName(74)).toBe('battery-level-70');
		expect(batteryIconName(75)).toBe('battery-level-80');
		expect(batteryIconName(3)).toBe('battery-level-0');
		expect(batteryIconName(140)).toBe('battery-level-100');
	});
});

describe('el perfil de audio', () => {
	test('la familia por el principio del nombre, que cambia entre versiones de PipeWire', () => {
		const label = (profile: string, codec: string | null = null) =>
			audioProfileLabel({ address: '', profile, description: null, codec }, t);

		expect(label('a2dp-sink')).toBe('profileA2dp');
		expect(label('a2dp-sink-aac', 'AAC')).toBe('profileA2dp · AAC');
		expect(label('headset-head-unit', 'mSBC')).toBe('profileHeadset · mSBC');
		expect(label('headset-head-unit-msbc')).toBe('profileHeadset');
		expect(label('bap-sink', 'LC3')).toBe('profileLeAudio · LC3');
	});

	test('uno que no se reconoce va con la descripción de WirePlumber o con su nombre', () => {
		expect(
			audioProfileLabel(
				{ address: '', profile: 'raro', description: 'Raro (algo)', codec: null },
				t
			)
		).toBe('Raro (algo)');
		expect(
			audioProfileLabel({ address: '', profile: 'raro', description: null, codec: null }, t)
		).toBe('raro');
	});

	test('sin perfil no hay texto', () => {
		expect(audioProfileLabel(null, t)).toBe('');
		expect(
			audioProfileLabel({ address: '', profile: null, description: 'x', codec: 'AAC' }, t)
		).toBe('');
	});
});

describe('la órbita de la red', () => {
	test('el SSID al centro, con el icono de su señal', () => {
		expect(networkCenter(WIFI, t)).toEqual({
			icon: 'network-wireless-signal-good',
			iconType: 'symbol',
			title: 'Fibernet-IA',
			subtitle: 'connected',
		});
	});

	test('cuatro satélites: ver redes, señal, seguridad, IP', () => {
		const satellites = networkSatellites(WIFI, t);

		expect(ids(satellites)).toEqual(['networks', 'signal', 'security', 'ip']);
		expect(satellites[0]).toMatchObject({ action: true, value: 'showNetworks' });
		expect(satelliteValue(satellites, 'signal')).toBe('78 %');
		expect(satelliteValue(satellites, 'security')).toBe('securityWpa2Personal');
		expect(satelliteValue(satellites, 'ip')).toBe('192.0.2.42');
	});

	test('una IP 0.0.0.0 es que no hay: el satélite va vacío', () => {
		expect(satelliteValue(networkSatellites({ ...WIFI, ip_address: '0.0.0.0' }, t), 'ip')).toBe('');
		expect(satelliteValue(networkSatellites({ ...WIFI, ip_address: '' }, t), 'ip')).toBe('');
	});

	test('por cable: al centro la conexión, sin señal ni seguridad de Wi-Fi', () => {
		const wired = {
			...WIFI,
			connection_type: 'Ethernet',
			name: 'Conexión cableada 1',
			icon: 'network-wired-symbolic',
		};

		expect(networkCenter(wired, t)).toMatchObject({
			title: 'Conexión cableada 1',
			icon: 'network-wired-symbolic',
		});
		const satellites = networkSatellites(wired, t);
		expect(satelliteValue(satellites, 'signal')).toBeNull();
		expect(satelliteValue(satellites, 'security')).toBeNull();
		expect(satelliteValue(satellites, 'ip')).toBe('192.0.2.42');
	});

	test('sin conexión: el centro vacío y sólo «Ver redes»', () => {
		expect(networkCenter({ ...WIFI, is_connected: false }, t)).toBeNull();
		expect(networkCenter(null, t)).toBeNull();
		expect(ids(networkSatellites(null, t))).toEqual(['networks']);
	});

	test('el Wi-Fi se reconoce como lo nombran NetworkManager y el complemento', () => {
		expect(isWifi({ connection_type: 'wifi' })).toBe(true);
		expect(isWifi({ connection_type: '802-11-wireless' })).toBe(true);
		expect(isWifi({ connection_type: 'Ethernet' })).toBe(false);
		expect(isWifi(null)).toBe(false);
	});

	test('los nombres de la seguridad y los iconos de la señal', () => {
		expect(securityLabel('none', t)).toBe('securityOpen');
		expect(securityLabel('wep', t)).toBe('WEP');
		expect(securityLabel('wpa3-psk', t)).toBe('securityWpa3Personal');
		expect(securityLabel('sae-algo', t)).toBe('sae-algo');
		expect(signalIconName(90)).toBe('network-wireless-signal-excellent');
		expect(signalIconName(60)).toBe('network-wireless-signal-good');
		expect(signalIconName(40)).toBe('network-wireless-signal-ok');
		expect(signalIconName(10)).toBe('network-wireless-signal-weak');
		expect(signalIconName(0)).toBe('network-wireless-signal-none');
	});
});
