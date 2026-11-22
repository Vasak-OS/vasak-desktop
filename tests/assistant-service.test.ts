/**
 * El servicio del asistente (`assistant.service.ts`): cada función llama al
 * comando de Tauri correcto con los argumentos en el casing que espera Rust
 * (`id`, `content`, `actionId`: Tauri pasa a camelCase los parámetros).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import * as service from '../src/services/assistant.service';
import { tauriCalls, useDom } from './support/mount-sfc';

useDom();

type Internals = { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
const internals = () => (globalThis as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;

/** Lo que contesta el `invoke` de mentira, por comando. */
let replies: Record<string, unknown> = {};

beforeAll(() => {
	// Se envuelve una sola vez: envolver en cada prueba anotaría cada llamada varias veces.
	const original = internals().invoke;
	internals().invoke = async (cmd, args) => {
		if (cmd.startsWith('plugin:')) return original(cmd, args);
		tauriCalls.push({ cmd, args });
		if (cmd in replies) return replies[cmd];
		return '';
	};
});

beforeEach(() => {
	tauriCalls.length = 0;
	replies = {};
});

const last = () => tauriCalls[tauriCalls.length - 1];

describe('los comandos que llama cada función', () => {
	test('las que no llevan argumentos llaman a su comando sin ellos', async () => {
		const casos: Array<[() => Promise<unknown>, string]> = [
			[service.assistantAvailable, 'assistant_available'],
			[service.assistantStatus, 'assistant_status'],
			[service.assistantCapabilities, 'assistant_capabilities'],
			[service.assistantNewConversation, 'assistant_new_conversation'],
			[service.toggleAssistant, 'toggle_assistant'],
			[service.hideAssistant, 'hide_assistant'],
		];
		for (const [call, cmd] of casos) {
			await call();
			expect(last()?.cmd).toBe(cmd);
			expect(last()?.args).toEqual({});
		}
	});

	test('el historial manda `{ id }`', async () => {
		await service.assistantHistory('conv-1');
		expect(last()).toEqual({ cmd: 'assistant_history', args: { id: 'conv-1' } });
	});

	test('un turno manda `{ id, content }`', async () => {
		await service.assistantChatTurn('conv-1', 'hola');
		expect(last()).toEqual({ cmd: 'assistant_chat_turn', args: { id: 'conv-1', content: 'hola' } });
	});

	test('confirmar y cancelar mandan `{ actionId }`, no `action_id`', async () => {
		await service.assistantConfirmAction('act-9');
		expect(last()).toEqual({ cmd: 'assistant_confirm_action', args: { actionId: 'act-9' } });

		await service.assistantCancelAction('act-9');
		expect(last()).toEqual({ cmd: 'assistant_cancel_action', args: { actionId: 'act-9' } });
	});
});

describe('lo que vuelve llega sin tocar', () => {
	test('el estado, con el modelo y el host', async () => {
		const status: service.AssistantStatus = {
			backend_available: true,
			reason: '',
			model: { provider: 'openai', name: 'gpt', local: false, host: 'api.remoto.test' },
			conversations: 2,
		};
		replies.assistant_status = status;
		expect(await service.assistantStatus()).toEqual(status);
	});

	test('la respuesta de un turno, con su acción pendiente', async () => {
		const reply: service.AssistantReply = {
			text: 'Lo preparé',
			pending: [
				{
					id: 'a',
					conversation_id: 'c',
					kind: 'send_mail',
					summary: 's',
					to: 't',
					subject: 'x',
					body_preview: 'b',
				},
			],
		};
		replies.assistant_chat_turn = reply;
		expect(await service.assistantChatTurn('c', 'mandá')).toEqual(reply);
	});

	test('el historial, las capacidades y los booleanos', async () => {
		replies.assistant_history = [{ role: 'user', content: 'hola' }];
		replies.assistant_capabilities = [{ domain: 'mail', available: false, reason: 'todavía no' }];
		replies.assistant_available = true;
		replies.assistant_new_conversation = 'conv-7';
		replies.assistant_confirm_action = 'Correo encolado';

		expect(await service.assistantHistory('c')).toEqual([{ role: 'user', content: 'hola' }]);
		expect(await service.assistantCapabilities()).toEqual([
			{ domain: 'mail', available: false, reason: 'todavía no' },
		]);
		expect(await service.assistantAvailable()).toBe(true);
		expect(await service.assistantNewConversation()).toBe('conv-7');
		expect(await service.assistantConfirmAction('a')).toBe('Correo encolado');
	});

	test('un error del comando llega como rechazo, para que la vista lo muestre', async () => {
		replies.assistant_history = Promise.reject('NotFound');
		await expect(service.assistantHistory('viejo')).rejects.toBe('NotFound');
	});
});
