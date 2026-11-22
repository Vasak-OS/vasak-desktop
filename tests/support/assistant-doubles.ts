/**
 * Los dobles de lo que `AssistantChatView.vue` importa desde `@/…`: el servicio
 * del asistente, el estado de la superficie, la medida de la caja y el bus de
 * eventos. Lo que la prueba cambia o mira cuelga de `assistant`.
 */
import { ref } from 'vue';
import type {
	AssistantMessage,
	AssistantPendingAction,
	AssistantReply,
	AssistantStatus,
} from '../../src/services/assistant.service';

export function localStatus(): AssistantStatus {
	return {
		backend_available: true,
		reason: '',
		model: { provider: 'ollama', name: 'llama3', local: true, host: 'localhost' },
		conversations: 0,
	};
}

type Deferred<T> = { promise: Promise<T>; resolve: (value: T) => void; reject: (error: unknown) => void };
export function deferred<T>(): Deferred<T> {
	let resolve!: (value: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

export const assistant = {
	status: localStatus() as AssistantStatus,
	history: [] as AssistantMessage[],
	/** Si no es `null`, `assistantHistory` rechaza con esto (id viejo). */
	historyError: null as unknown,
	newId: 'conv-nueva',
	/** Lo que contesta el turno; la prueba lo puede cambiar por una promesa a mano. */
	turn: (): Promise<AssistantReply> => Promise.resolve({ text: 'hola', pending: [] }),
	confirm: (): Promise<string> => Promise.resolve('Listo'),
	cancel: (): Promise<void> => Promise.resolve(),
	shown: true,
	calls: [] as Array<{ name: string; args: unknown[] }>,
	handlers: new Map<string, () => void>(),
	reset() {
		this.status = localStatus();
		this.history = [];
		this.historyError = null;
		this.newId = 'conv-nueva';
		this.turn = () => Promise.resolve({ text: 'hola', pending: [] });
		this.confirm = () => Promise.resolve('Listo');
		this.cancel = () => Promise.resolve();
		this.shown = true;
		this.calls = [];
		this.handlers.clear();
	},
};

const record = (name: string, ...args: unknown[]) => assistant.calls.push({ name, args });

export const assistantStatus = async () => {
	record('assistantStatus');
	return assistant.status;
};
export const assistantNewConversation = async () => {
	record('assistantNewConversation');
	return assistant.newId;
};
export const assistantHistory = async (id: string) => {
	record('assistantHistory', id);
	if (assistant.historyError !== null) throw assistant.historyError;
	return assistant.history;
};
export const assistantChatTurn = (id: string, content: string) => {
	record('assistantChatTurn', id, content);
	return assistant.turn();
};
export const assistantConfirmAction = (actionId: string) => {
	record('assistantConfirmAction', actionId);
	return assistant.confirm();
};
export const assistantCancelAction = (actionId: string) => {
	record('assistantCancelAction', actionId);
	return assistant.cancel();
};
export const hideAssistant = async () => {
	record('hideAssistant');
};
export const surfaceIsShown = async () => assistant.shown;
export const useBoxSize = () => ({ width: ref(600), height: ref(500) });
export const useSharedEvent = (name: string, handler: () => void) => {
	assistant.handlers.set(name, handler);
};

export type { AssistantPendingAction };
