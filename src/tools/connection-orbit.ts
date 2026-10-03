import type { DeviceInfo } from '@vasakgroup/plugin-bluetooth-manager';
import type { OrbitCenter, OrbitSatellite } from '@vasakgroup/vue-libvasak';
import type { NetworkInfo, WiFiSecurityType } from '@/services/network.service';

/**
 * Lo que la vista radial del Bluetooth y de la red (vasak-desktop#132) le pasa
 * a `DeviceOrbit`: el centro y los satélites, ya armados y traducidos.
 *
 * Vive aparte del componente para probarlo sin montar nada. `DeviceOrbit` no
 * sabe de Bluetooth ni de redes; esto sí, y sólo esto.
 *
 * # El orden de los satélites
 *
 * La órbita los reparte desde arriba en el sentido del reloj, así que el orden
 * es el lugar: con cuatro, **arriba la acción, a la derecha la batería o la
 * señal, abajo el perfil o la seguridad, a la izquierda la MAC o la IP**, que
 * es el reparto del video de referencia.
 *
 * # Lo que falta no se inventa
 *
 * Un dato que no se sabe va con `value` vacío y `DeviceOrbit` no lo dibuja: la
 * batería ausente de BlueZ es «no lo sé», no cero; el perfil que PipeWire no
 * informa, lo mismo; una IP `0.0.0.0` es que no hay.
 */

/** Las claves de traducción de la vista. */
const KEY = 'components.ConnectionsArea';

export type Translate = (key: string) => string;

/** El perfil de audio activo, como lo manda `get_bluetooth_audio_profile`. */
export interface BluetoothAudioProfile {
	address: string;
	profile: string | null;
	description: string | null;
	codec: string | null;
}

/** El icono de batería de freedesktop para un porcentaje, de diez en diez. */
export function batteryIconName(percent: number): string {
	const level = Math.min(100, Math.max(0, Math.round(percent / 10) * 10));
	return `battery-level-${level}`;
}

/** Por debajo de esto la batería se marca con el acento. */
export const LOW_BATTERY = 20;

/**
 * El nombre legible de un perfil de audio, con su códec.
 *
 * Los nombres de PipeWire cambiaron entre versiones (`a2dp-sink`,
 * `a2dp-sink-aac`, `headset-head-unit-msbc`…), así que se reconoce la familia
 * por el principio. Uno que no se reconoce va con la descripción de WirePlumber
 * o con su nombre tal cual: un dato raro es mejor que ninguno.
 */
export function audioProfileLabel(profile: BluetoothAudioProfile | null, t: Translate): string {
	const name = profile?.profile?.toLowerCase() ?? '';
	if (!name) return '';

	let family: string;
	if (name.startsWith('a2dp')) family = t(`${KEY}.profileA2dp`);
	else if (name.startsWith('headset') || name.startsWith('hsp') || name.startsWith('hfp'))
		family = t(`${KEY}.profileHeadset`);
	else if (name.startsWith('bap') || name.includes('le-audio') || name.startsWith('le'))
		family = t(`${KEY}.profileLeAudio`);
	else family = profile?.description || profile?.profile || '';

	return profile?.codec ? `${family} · ${profile.codec}` : family;
}

/** El dispositivo del centro de la órbita de Bluetooth. */
export function bluetoothCenter(device: DeviceInfo | null, t: Translate): OrbitCenter | null {
	if (!device) return null;
	return {
		icon: device.icon || 'bluetooth',
		iconType: 'symbol',
		title: device.alias || device.name || device.address,
		subtitle: t(`${KEY}.connected`),
	};
}

/** Los satélites de la órbita de Bluetooth: buscar, batería, perfil y MAC. */
export function bluetoothSatellites(
	device: DeviceInfo | null,
	profile: BluetoothAudioProfile | null,
	t: Translate,
	options: { scanning?: boolean; disabled?: boolean } = {}
): OrbitSatellite[] {
	const scan: OrbitSatellite = {
		id: 'scan',
		icon: 'view-refresh',
		value: options.scanning ? t(`${KEY}.scanning`) : t(`${KEY}.scan`),
		label: t(`${KEY}.switchView`),
		action: true,
		disabled: options.disabled,
	};
	if (!device) return [scan];

	const battery =
		typeof device.battery === 'number' && Number.isFinite(device.battery) ? device.battery : null;
	// El perfil es de esta MAC o de ninguna: uno que quedó de otro dispositivo
	// no se muestra.
	const ownProfile =
		profile && profile.address.toUpperCase() === device.address.toUpperCase() ? profile : null;

	return [
		scan,
		{
			id: 'battery',
			icon: battery === null ? undefined : batteryIconName(battery),
			value: battery === null ? null : `${Math.round(battery)} %`,
			label: t(`${KEY}.battery`),
			tone: battery !== null && battery < LOW_BATTERY ? 'accent' : 'default',
		},
		{
			id: 'profile',
			icon: 'audio-speakers',
			value: audioProfileLabel(ownProfile, t),
			label: t(`${KEY}.audioProfile`),
		},
		{
			id: 'mac',
			icon: 'dialog-information',
			value: device.address,
			label: t(`${KEY}.macAddress`),
		},
	];
}

/** Si la conexión es Wi-Fi. NetworkManager y el complemento no la nombran igual. */
export function isWifi(info: Pick<NetworkInfo, 'connection_type'> | null): boolean {
	const type = info?.connection_type?.toLowerCase() ?? '';
	return type === 'wifi' || type === 'wireless' || type === '802-11-wireless';
}

/** El nombre legible del tipo de seguridad. */
export function securityLabel(type: WiFiSecurityType | string | undefined, t: Translate): string {
	switch (type) {
		case 'none':
			return t(`${KEY}.securityOpen`);
		case 'wep':
			return 'WEP';
		case 'wpa-psk':
			return t(`${KEY}.securityWpaPersonal`);
		case 'wpa-eap':
			return t(`${KEY}.securityWpaEnterprise`);
		case 'wpa2-psk':
			return t(`${KEY}.securityWpa2Personal`);
		case 'wpa3-psk':
			return t(`${KEY}.securityWpa3Personal`);
		default:
			return type ? String(type) : '';
	}
}

/** El icono de señal de freedesktop para un porcentaje. */
export function signalIconName(strength: number): string {
	if (strength >= 80) return 'network-wireless-signal-excellent';
	if (strength >= 55) return 'network-wireless-signal-good';
	if (strength >= 30) return 'network-wireless-signal-ok';
	if (strength > 0) return 'network-wireless-signal-weak';
	return 'network-wireless-signal-none';
}

/** Una IP que dice algo: vacía o `0.0.0.0` es que no hay. */
function usableIp(ip: string | undefined): string {
	return ip && ip !== '0.0.0.0' ? ip : '';
}

/**
 * La conexión del centro de la órbita de red.
 *
 * Wi-Fi, con su SSID; si lo conectado es otra cosa —un cable—, también va al
 * centro, con su nombre: lo que importa es lo que está en uso.
 */
export function networkCenter(info: NetworkInfo | null, t: Translate): OrbitCenter | null {
	if (!info?.is_connected) return null;
	if (isWifi(info)) {
		return {
			icon: signalIconName(info.signal_strength),
			iconType: 'symbol',
			title: info.ssid || info.name,
			subtitle: t(`${KEY}.connected`),
		};
	}
	return {
		icon: info.icon || 'network-wired',
		iconType: 'symbol',
		title: info.name || info.connection_type,
		subtitle: t(`${KEY}.connected`),
	};
}

/** Los satélites de la órbita de red: ver redes, señal, seguridad e IP. */
export function networkSatellites(
	info: NetworkInfo | null,
	t: Translate,
	options: { disabled?: boolean } = {}
): OrbitSatellite[] {
	const networks: OrbitSatellite = {
		id: 'networks',
		icon: 'view-list',
		value: t(`${KEY}.showNetworks`),
		label: t(`${KEY}.switchView`),
		action: true,
		disabled: options.disabled,
	};
	if (!info?.is_connected) return [networks];

	const wifi = isWifi(info);
	return [
		networks,
		{
			id: 'signal',
			icon: wifi ? signalIconName(info.signal_strength) : undefined,
			value: wifi ? `${Math.round(info.signal_strength)} %` : null,
			label: t(`${KEY}.signal`),
		},
		{
			id: 'security',
			icon: info.security_type === 'none' ? 'security-low' : 'security-high',
			value: wifi ? securityLabel(info.security_type, t) : null,
			label: t(`${KEY}.security`),
		},
		{
			id: 'ip',
			icon: 'network-workgroup',
			value: usableIp(info.ip_address),
			label: t(`${KEY}.ipAddress`),
		},
	];
}
