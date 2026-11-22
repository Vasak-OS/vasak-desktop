/**
 * La superficie de chat de Lince, montada (`AssistantChatView.vue`).
 *
 * Los componentes de la librería son los publicados; el servicio, la superficie
 * y el bus son dobles (`support/assistant-doubles.ts`).
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import type { AssistantPendingAction } from '../src/services/assistant.service';
import { assistant, deferred, localStatus } from './support/assistant-doubles';
import { loadComponent, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'assistant-doubles.ts');
const dom = useDom();
const STORAGE_KEY = 'vasak.assistant.conversation';

let View: any;
beforeAll(async () => {
	View = await loadComponent(dom.workdir(), 'src/views/AssistantChatView.vue', DOUBLES, 'AssistantChatView');
}, 60_000);

beforeEach(() => {
	assistant.reset();
	localStorage.clear();
});

// Una prueba que falla no llega a su `unmount()`: el `keydown` del documento
// quedaría escuchando y contaminaría a las siguientes.
const mounted: Array<{ unmount: () => void }> = [];
afterEach(() => {
	for (const view of mounted.splice(0)) view.unmount();
});

const settle = async () => {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await nextTick();
	}
};

async function render() {
	const view = mount(View, { attachTo: document.body });
	mounted.push(view);
	await settle();
	return view;
}

async function type(view: ReturnType<typeof mount>, text: string, press = true) {
	const input = view.find('input');
	await input.setValue(text);
	if (press) await input.trigger('keydown', { key: 'Enter' });
	await settle();
}

const pendingMail = (over: Partial<AssistantPendingAction> = {}): AssistantPendingAction => ({
	id: 'act-1',
	conversation_id: 'conv-nueva',
	kind: 'send_mail',
	summary: 'Mandar un correo a Ana',
	to: 'ana@example.test',
	subject: 'Hola Ana',
	body_preview: 'Nos vemos mañana',
	...over,
});

const called = (name: string) => assistant.calls.filter((call) => call.name === name);

describe('mandar un mensaje', () => {
	test('Enter manda el turno con el id de la conversación y el texto', async () => {
		const view = await render();
		await type(view, '  ¿qué hora es?  ');

		expect(called('assistantChatTurn')[0]?.args).toEqual(['conv-nueva', '¿qué hora es?']);
	});

	test('mientras espera, el campo y el botón se deshabilitan y se ve «pensando»', async () => {
		const reply = deferred<{ text: string; pending: never[] }>();
		assistant.turn = () => reply.promise;
		const view = await render();
		await type(view, 'hola');

		expect(view.find('[data-thinking]').exists()).toBe(true);
		expect(view.find('input').attributes('disabled')).toBeDefined();
		expect(view.find('[data-send]').attributes('disabled')).toBeDefined();
		// Un segundo Enter mientras espera no manda otro turno.
		await view.find('input').trigger('keydown', { key: 'Enter' });
		expect(called('assistantChatTurn')).toHaveLength(1);

		reply.resolve({ text: 'Son las tres', pending: [] });
		await settle();

		expect(view.find('[data-thinking]').exists()).toBe(false);
		const mine = view.find('[data-role="user"]');
		const theirs = view.find('[data-role="assistant"]');
		expect(mine.text()).toBe('hola');
		expect(theirs.text()).toBe('Son las tres');
		expect(view.find('input').attributes('disabled')).toBeUndefined();
	});

	test('Shift+Enter no manda, y un mensaje en blanco tampoco', async () => {
		const view = await render();
		await view.find('input').setValue('algo');
		await view.find('input').trigger('keydown', { key: 'Enter', shiftKey: true });
		await type(view, '   ');

		expect(called('assistantChatTurn')).toHaveLength(0);
	});
});

describe('lo que llega se dibuja como texto, nunca como HTML', () => {
	const attack = '<img src=x onerror="window.__pwned = 1"><b>negrita</b>';

	test('el contenido de un mensaje con etiquetas se ve literal y no crea nodos', async () => {
		assistant.turn = async () => ({ text: attack, pending: [] });
		const view = await render();
		await type(view, 'hola');

		const bubble = view.find('[data-role="assistant"]');
		expect(bubble.text()).toBe(attack);
		expect(bubble.element.querySelector('img')).toBeNull();
		expect(bubble.element.querySelector('b')).toBeNull();
		expect(bubble.element.innerHTML).toContain('&lt;img');
		expect((window as any).__pwned).toBeUndefined();
	});

	test('el historial también: un mensaje viejo con HTML no se interpreta', async () => {
		localStorage.setItem(STORAGE_KEY, 'conv-vieja');
		assistant.history = [{ role: 'assistant', content: attack }];
		const view = await render();

		const bubble = view.find('[data-role="assistant"]');
		expect(bubble.element.querySelector('img')).toBeNull();
		expect(bubble.element.innerHTML).toContain('&lt;b&gt;negrita&lt;/b&gt;');
	});

	test('el resumen y los campos de la acción propuesta se ven literales', async () => {
		assistant.turn = async () => ({
			text: 'ok',
			pending: [
				pendingMail({
					summary: '<img src=x onerror=alert(1)>',
					to: '<b>x@y.test</b>',
					subject: '<i>asunto</i>',
					body_preview: '<script>alert(1)</script>',
				}),
			],
		});
		const view = await render();
		await type(view, 'mandá un correo');

		const card = view.find('[data-confirmation]');
		expect(card.element.querySelector('img, b, i, script')).toBeNull();
		expect(card.element.innerHTML).toContain('&lt;img');
		expect(card.element.innerHTML).toContain('&lt;script&gt;');
		expect(card.text()).toContain('<b>x@y.test</b>');
	});

	test('el resultado de confirmar también se ve literal', async () => {
		assistant.turn = async () => ({ text: 'ok', pending: [pendingMail()] });
		assistant.confirm = async () => '<img src=x onerror=alert(1)>';
		const view = await render();
		await type(view, 'mandalo');
		await view.find('[data-confirm]').trigger('click');
		await settle();

		const bubbles = view.findAll('[data-role="assistant"]');
		const last = bubbles[bubbles.length - 1]!;
		expect(last.element.querySelector('img')).toBeNull();
		expect(last.text()).toBe('<img src=x onerror=alert(1)>');
	});
});

describe('el indicador «sale de la caja»', () => {
	test('con un modelo remoto avisa a qué host viaja lo que se escribe', async () => {
		assistant.status = {
			...localStatus(),
			model: { provider: 'openai', name: 'gpt', local: false, host: 'api.remoto.test' },
		};
		const view = await render();

		expect(view.text()).toContain('api.remoto.test');
		// El catálogo no se carga en la prueba: `t()` devuelve la clave.
		expect(view.find('.border-status-warning\\/30').text()).toBe('views.assistant.leavesBox');
		expect(view.find('[data-chip]').attributes('title')).toBe('views.assistant.leavesBox');
	});

	test('con un modelo local no hay aviso y el indicador dice que es en el equipo', async () => {
		const view = await render();

		expect(view.text()).not.toContain('views.assistant.leavesBox');
		expect(view.find('.border-status-warning\\/30').exists()).toBe(false);
		expect(view.find('[data-chip] [data-label]').text()).toBe('views.assistant.local');
	});
});

describe('la tarjeta de confirmación', () => {
	async function withPending() {
		assistant.turn = async () => ({ text: 'Lo preparé', pending: [pendingMail()] });
		const view = await render();
		await type(view, 'mandá el correo');
		return view;
	}

	test('muestra el resumen y, en un correo, destinatario, asunto y vista previa', async () => {
		const view = await withPending();
		const card = view.find('[data-confirmation]');

		expect(card.exists()).toBe(true);
		for (const text of ['Mandar un correo a Ana', 'ana@example.test', 'Hola Ana', 'Nos vemos mañana']) {
			expect(card.text()).toContain(text);
		}
	});

	test('una acción que no es un correo muestra sólo el resumen', async () => {
		assistant.turn = async () => ({
			text: 'ok',
			pending: [pendingMail({ kind: 'create_event', summary: 'Crear una reunión' })],
		});
		const view = await render();
		await type(view, 'agendá');

		const card = view.find('[data-confirmation]');
		expect(card.text()).toContain('Crear una reunión');
		expect(card.text()).not.toContain('ana@example.test');
	});

	test('Confirmar llama con el id de la acción, muestra el resultado y cierra la tarjeta', async () => {
		assistant.confirm = async () => 'Correo encolado';
		const view = await withPending();
		await view.find('[data-confirm]').trigger('click');
		await settle();

		expect(called('assistantConfirmAction')[0]?.args).toEqual(['act-1']);
		expect(view.find('[data-confirmation]').exists()).toBe(false);
		expect(view.text()).toContain('Correo encolado');
	});

	test('Cancelar descarta la acción sin confirmarla', async () => {
		const view = await withPending();
		await view.find('[data-cancel]').trigger('click');
		await settle();

		expect(called('assistantCancelAction')[0]?.args).toEqual(['act-1']);
		expect(called('assistantConfirmAction')).toHaveLength(0);
		expect(view.find('[data-confirmation]').exists()).toBe(false);
	});

	test('con una acción pendiente no se puede mandar otro mensaje', async () => {
		const view = await withPending();

		expect(view.find('input').attributes('disabled')).toBeDefined();
		expect(view.find('[data-send]').attributes('disabled')).toBeDefined();
		await view.find('input').trigger('keydown', { key: 'Enter' });
		expect(called('assistantChatTurn')).toHaveLength(1);
	});

	test('si confirmar falla, la tarjeta queda para reintentar y el error se ve', async () => {
		assistant.confirm = () => Promise.reject('Busy');
		const view = await withPending();
		await view.find('[data-confirm]').trigger('click');
		await settle();

		expect(view.find('[data-confirmation]').exists()).toBe(true);
		expect(view.text()).toContain('Busy');
	});
});

describe('la conversación persistida', () => {
	test('sin id guardado abre una nueva y la guarda', async () => {
		const view = await render();

		expect(called('assistantNewConversation')).toHaveLength(1);
		expect(localStorage.getItem(STORAGE_KEY)).toBe('conv-nueva');
	});

	test('con un id guardado lo usa: pide su historial y lo muestra', async () => {
		localStorage.setItem(STORAGE_KEY, 'conv-guardada');
		assistant.history = [
			{ role: 'system', content: 'instrucciones internas' },
			{ role: 'user', content: 'hola antes' },
			{ role: 'assistant', content: 'respuesta antes' },
		];
		const view = await render();

		expect(called('assistantHistory')[0]?.args).toEqual(['conv-guardada']);
		expect(called('assistantNewConversation')).toHaveLength(0);
		expect(view.text()).toContain('hola antes');
		expect(view.text()).toContain('respuesta antes');
		expect(view.text()).not.toContain('instrucciones internas');
	});

	test('si el id guardado ya no existe, empieza otra conversación sin romper', async () => {
		localStorage.setItem(STORAGE_KEY, 'conv-muerta');
		assistant.historyError = 'NotFound';
		const view = await render();

		expect(called('assistantNewConversation')).toHaveLength(1);
		expect(localStorage.getItem(STORAGE_KEY)).toBe('conv-nueva');
		expect(view.find('input').exists()).toBe(true);
		// El chat sigue andando con el id nuevo.
		await type(view, 'hola');
		expect(called('assistantChatTurn')[0]?.args[0]).toBe('conv-nueva');
	});

	test('si la superficie no está a la vista al montar, no consulta nada', async () => {
		assistant.shown = false;
		const view = await render();

		expect(assistant.calls).toHaveLength(0);
	});

	test('al volver a mostrarse (`window-shown`) recarga el estado y el historial', async () => {
		const view = await render();
		const before = called('assistantStatus').length;
		assistant.handlers.get('window-shown')?.();
		await settle();

		expect(called('assistantStatus').length).toBe(before + 1);
	});
});

describe('los errores no rompen el chat', () => {
	test('un turno que falla se muestra en una alerta y se puede seguir escribiendo', async () => {
		assistant.turn = () => Promise.reject('Unavailable: sin modelo');
		const view = await render();
		await type(view, 'hola');

		expect(view.text()).toContain('Unavailable: sin modelo');
		expect(view.find('[data-thinking]').exists()).toBe(false);
		expect(view.find('input').attributes('disabled')).toBeUndefined();

		assistant.turn = async () => ({ text: 'ya anda', pending: [] });
		await type(view, 'otra vez');
		expect(view.text()).toContain('ya anda');
		expect(view.text()).not.toContain('Unavailable: sin modelo');
	});

	test('un servicio sin modelo avisa el motivo', async () => {
		assistant.status = { ...localStatus(), backend_available: false, reason: 'Ollama no responde' };
		const view = await render();

		expect(view.text()).toContain('Ollama no responde');
	});

	test('Escape y el botón de cerrar esconden la superficie', async () => {
		const view = await render();
		document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		await settle();

		expect(called('hideAssistant')).toHaveLength(1);
	});
});

describe('la carga se coalesce', () => {
	test('montar y `window-shown` a la vez crean una sola conversación', async () => {
		const gate = deferred<void>();
		assistant.newGate = gate.promise;
		const view = mount(View, { attachTo: document.body });
		mounted.push(view);
		await settle();
		// La primera carga espera a `assistantNewConversation`; llega `window-shown`.
		assistant.handlers.get('window-shown')?.();
		await settle();
		gate.resolve();
		await settle();

		expect(called('assistantNewConversation')).toHaveLength(1);
		expect(called('assistantStatus')).toHaveLength(1);
		expect(localStorage.getItem(STORAGE_KEY)).toBe('conv-nueva');
	});

	test('terminada una carga, la siguiente vuelve a correr', async () => {
		await render();
		assistant.handlers.get('window-shown')?.();
		await settle();

		expect(called('assistantStatus')).toHaveLength(2);
	});
});

describe('el id de la conversación', () => {
	test('un error temporal del historial conserva el id, lo muestra y no abre otra conversación', async () => {
		localStorage.setItem(STORAGE_KEY, 'conv-guardada');
		assistant.historyError = 'Unavailable: ocupado';
		const view = await render();

		expect(called('assistantNewConversation')).toHaveLength(0);
		expect(localStorage.getItem(STORAGE_KEY)).toBe('conv-guardada');
		expect(view.text()).toContain('Unavailable: ocupado');
		// El id sigue siendo el de siempre: se puede escribir con él.
		await type(view, 'hola');
		expect(called('assistantChatTurn')[0]?.args[0]).toBe('conv-guardada');
	});

	test('sin id (no se pudo abrir una conversación) no se manda y aparece reintentar', async () => {
		assistant.newGate = Promise.reject('Unavailable: sin servicio');
		const view = await render();

		expect(view.find('[data-retry]').exists()).toBe(true);
		await type(view, 'hola');
		expect(called('assistantChatTurn')).toHaveLength(0);
		expect(view.find('[data-send]').attributes('disabled')).toBeDefined();
	});

	test('Reintentar vuelve a cargar y, con el servicio de vuelta, se puede escribir', async () => {
		assistant.newGate = Promise.reject('Unavailable: sin servicio');
		const view = await render();
		assistant.newGate = null;
		await view.find('[data-retry]').trigger('click');
		await settle();

		expect(called('assistantNewConversation')).toHaveLength(2);
		expect(view.find('[data-retry]').exists()).toBe(false);
		await type(view, 'hola');
		expect(called('assistantChatTurn')[0]?.args[0]).toBe('conv-nueva');
	});
});

describe('la acción pendiente que se pierde si muere WebKit', () => {
	const PENDING_KEY = 'vasak.assistant.pending';
	const withAction = async () => {
		assistant.turn = async () => ({ text: 'ok', pending: [pendingMail()] });
		const view = await render();
		await type(view, 'mandalo');
		return view;
	};

	test('al recibir una acción se guarda su id', async () => {
		await withAction();
		expect(localStorage.getItem(PENDING_KEY)).toBe('act-1');
	});

	test('al confirmar o cancelar se borra', async () => {
		let view = await withAction();
		await view.find('[data-confirm]').trigger('click');
		await settle();
		expect(localStorage.getItem(PENDING_KEY)).toBeNull();

		view.unmount();
		view = await withAction();
		await view.find('[data-cancel]').trigger('click');
		await settle();
		expect(localStorage.getItem(PENDING_KEY)).toBeNull();
	});

	test('un turno sin acción también lo borra', async () => {
		localStorage.setItem(PENDING_KEY, 'act-vieja');
		const view = await render();
		localStorage.setItem(PENDING_KEY, 'act-1');
		await type(view, 'hola');

		expect(localStorage.getItem(PENDING_KEY)).toBeNull();
	});

	test('si al cargar hay clave y ninguna pendiente en memoria, avisa y la limpia', async () => {
		localStorage.setItem(PENDING_KEY, 'act-perdida');
		const view = await render();

		expect(view.text()).toContain('views.assistant.pendingLost');
		expect(localStorage.getItem(PENDING_KEY)).toBeNull();
		expect(view.find('[data-confirmation]').exists()).toBe(false);
	});

	test('sin clave no hay aviso', async () => {
		const view = await render();
		expect(view.text()).not.toContain('views.assistant.pendingLost');
	});
});
