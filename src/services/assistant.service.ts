import { invoke } from '@tauri-apps/api/core';

/** Espejo de lo que devuelve `vasak-assistant` (campos en snake_case, como connect). */
export interface AssistantModel {
	provider: string;
	name: string;
	local: boolean;
	host: string;
}

export interface AssistantStatus {
	backend_available: boolean;
	reason: string;
	model: AssistantModel;
	conversations: number;
}

export interface AssistantCapability {
	domain: string;
	available: boolean;
	reason: string;
}

export type AssistantRole = 'system' | 'user' | 'assistant';

export interface AssistantMessage {
	role: AssistantRole;
	content: string;
}

export interface AssistantPendingAction {
	id: string;
	conversation_id: string;
	kind: string;
	summary: string;
	to: string;
	subject: string;
	body_preview: string;
}

export interface AssistantReply {
	text: string;
	pending: AssistantPendingAction[];
}

/** Si el asistente está encendido: decide si el panel dibuja su botón. */
export const assistantAvailable = (): Promise<boolean> => invoke<boolean>('assistant_available');

export const assistantStatus = (): Promise<AssistantStatus> =>
	invoke<AssistantStatus>('assistant_status');

export const assistantCapabilities = (): Promise<AssistantCapability[]> =>
	invoke<AssistantCapability[]>('assistant_capabilities');

export const assistantNewConversation = (): Promise<string> =>
	invoke<string>('assistant_new_conversation');

export const assistantHistory = (id: string): Promise<AssistantMessage[]> =>
	invoke<AssistantMessage[]>('assistant_history', { id });

/** Sin plazo del lado de la página: con un modelo local puede tardar minutos. */
export const assistantChatTurn = (id: string, content: string): Promise<AssistantReply> =>
	invoke<AssistantReply>('assistant_chat_turn', { id, content });

export const assistantConfirmAction = (actionId: string): Promise<string> =>
	invoke<string>('assistant_confirm_action', { actionId });

export const assistantCancelAction = (actionId: string): Promise<void> =>
	invoke<void>('assistant_cancel_action', { actionId });

export const toggleAssistant = (): Promise<void> => invoke<void>('toggle_assistant');

/** Cierra el chat. No el conmutador: puede reabrir lo que se pidió cerrar. */
export const hideAssistant = (): Promise<void> => invoke<void>('hide_assistant');
