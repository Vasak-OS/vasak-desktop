import { invoke } from '@tauri-apps/api/core';

export interface TwingateResource {
	name: string;
	address: string;
	alias: string | null;
	/** `MAIN`, `KUBERNETES` o `BACKGROUND`, como los agrupa Twingate. */
	group: string;
	/** El texto de estado sin interpretar, para poder mostrarlo tal cual. */
	status: string;
	usable: boolean;
	needs_auth: boolean;
	/** Lo que falta para el vencimiento, en las palabras de Twingate. */
	expires_in: string | null;
}

export interface TwingateInfo {
	installed: boolean;
	status: string;
	connected: boolean;
	resources: TwingateResource[];
}

export const getTwingateInfo = (): Promise<TwingateInfo> => invoke('twingate_info');

export const authorizeTwingateResource = (resource: string): Promise<void> =>
	invoke('twingate_authorize', { resource });
