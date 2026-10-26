/**
 * El applet de Wi-Fi del escritorio: conectar a una red con contraseña.
 *
 * `security_type` es un campo requerido de `WiFiConnectionConfig` en el plugin
 * de Rust (sin `#[serde(default)]`): si el applet no lo manda, el `invoke` no
 * deserializa y no conecta, sin ningún aviso visible. Ajustes sí lo mandaba.
 *
 * La tarjeta se monta con el servicio de verdad y el `invoke` de mentira: lo
 * que se mira es qué configuración le llega al plugin.
 */
import { afterEach, beforeAll, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, tauriCalls, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'wifi-card-doubles.ts');
const dom = useDom();

const CONNECT = 'plugin:network-manager|connect_to_wifi';

const settle = async () => {
	for (let i = 0; i < 6; i++) await nextTick();
	await new Promise((resolve) => setTimeout(resolve, 0));
	for (let i = 0; i < 6; i++) await nextTick();
};

const network = {
	name: 'Casa',
	ssid: 'Casa',
	connection_type: 'wifi',
	icon: 'network-wireless-signal-good',
	ip_address: '',
	mac_address: '',
	signal_strength: 80,
	security_type: 'wpa2-psk',
	is_connected: false,
};

let Card: any;
beforeAll(async () => {
	Card = await loadComponent(dom.workdir(), 'src/components/cards/NetworkWiFiCard.vue', DOUBLES, 'NetworkWiFiCard');
}, 60_000);

afterEach(() => {
	tauriCalls.length = 0;
	document.body.innerHTML = '';
});

/** Abre el diálogo y deja la vista montada. */
async function openDialog(props: Record<string, unknown> = network) {
	const view = mount(Card, { props, attachTo: document.body });
	await view.get('div').trigger('click');
	await settle();
	return view;
}

const passwordInput = () => document.body.querySelector('input[autocomplete="current-password"]') as HTMLInputElement;

describe('conectar a una red con contraseña', () => {
	for (const security_type of ['wpa2-psk', 'wpa3-psk', 'wep']) {
		test(`manda la seguridad de la red (${security_type}) junto con el ssid y la contraseña`, async () => {
			await openDialog({ ...network, security_type });
			const input = passwordInput();
			input.value = 'secreta123';
			input.dispatchEvent(new Event('input', { bubbles: true }));
			await settle();

			const connect = [...document.body.querySelectorAll('button')].find((b) => !b.disabled && b.textContent?.includes('connectAction'));
			expect(connect).toBeDefined();
			connect?.click();
			await settle();

			const calls = tauriCalls.filter((call) => call.cmd === CONNECT);
			expect(calls).toHaveLength(1);
			expect((calls[0].args as { config: unknown }).config).toEqual({
				ssid: 'Casa',
				password: 'secreta123',
				security_type,
			});
		});
	}
});

describe('mostrar la contraseña', () => {
	test('el campo es el de la librería, con su botón de mostrar, y no un input crudo', async () => {
		await openDialog();
		expect(passwordInput().type).toBe('password');
		const toggle = document.body.querySelector('button[aria-pressed]');
		expect(toggle).not.toBeNull();

		(toggle as HTMLButtonElement).click();
		await settle();
		expect(passwordInput().type).toBe('text');
	});
});
