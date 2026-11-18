import { invoke } from '@tauri-apps/api/core';

export interface PhoenixResource {
	name: string;
	address: string;
	alias: string | null;
	/** `MAIN`, `KUBERNETES` o `BACKGROUND`, como los agrupa Phoenix. */
	group: string;
	/** El texto de estado sin interpretar, para poder mostrarlo tal cual. */
	status: string;
	usable: boolean;
	needs_auth: boolean;
	/** Lo que falta para el vencimiento, en las palabras de Phoenix. */
	expires_in: string | null;
}

export interface PhoenixInfo {
	installed: boolean;
	status: string;
	connected: boolean;
	resources: PhoenixResource[];
}

export const getPhoenixInfo = (): Promise<PhoenixInfo> => invoke('phoenix_info');

export const authorizePhoenixResource = (resource: string): Promise<void> =>
	invoke('phoenix_authorize', { resource });
