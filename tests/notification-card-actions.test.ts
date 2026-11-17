/**
 * Accionar una notificación desde el centro de notificaciones.
 *
 * Tauri v2 convierte el nombre de cada argumento de un comando a camelCase, así
 * que `invoke_notification_action(id, action_key)` espera `actionKey` en el
 * payload. Con `action_key` el comando fallaba antes de llegar al demonio y ni
 * el clic ni los botones hacían nada. La tarjeta se monta de verdad con el
 * servicio de verdad: lo que se mira es el payload que cruza el borde.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { loadComponent, tauriCalls, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'notification-card-doubles.ts');
const dom = useDom();

const notification = {
	id: 42,
	app_name: 'Navegador',
	app_icon: 'web-browser',
	summary: 'Descarga lista',
	body: '',
	timestamp: 100,
	seen: false,
	actions: ['default', 'Abrir', 'open-folder', 'Mostrar carpeta'],
};

let Card: any;
beforeAll(async () => {
	Card = await loadComponent(
		dom.workdir(),
		'src/components/cards/NotificationCard.vue',
		DOUBLES,
		'NotificationCard'
	);
});

beforeEach(() => {
	tauriCalls.length = 0;
});

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const actionCalls = () => tauriCalls.filter((call) => call.cmd === 'invoke_notification_action');

describe('accionar una notificación', () => {
	test('el clic en la notificación manda la acción por omisión con actionKey en camelCase', async () => {
		const wrapper = mount(Card, { props: { notification } });
		await wrapper.find('[role="button"]').trigger('click');
		await settle();

		const calls = actionCalls();
		expect(calls).toHaveLength(1);
		expect(calls[0].args).toEqual({ id: 42, actionKey: 'default' });
		expect(calls[0].args).not.toHaveProperty('action_key');
	});

	test('un botón de acción manda su clave con actionKey en camelCase', async () => {
		const wrapper = mount(Card, { props: { notification } });
		const button = wrapper.findAll('button').find((b) => b.text().includes('Mostrar carpeta'));
		expect(button).toBeDefined();
		await button?.trigger('click');
		await settle();

		const calls = actionCalls();
		expect(calls).toHaveLength(1);
		expect(calls[0].args).toEqual({ id: 42, actionKey: 'open-folder' });
		expect(calls[0].args).not.toHaveProperty('action_key');
	});
});
