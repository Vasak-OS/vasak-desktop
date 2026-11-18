import { invoke } from '@tauri-apps/api/core';
import type { DownloadActivity } from '@/tools/composables/useActivityProgress';

/**
 * Las descargas del navegador en curso ahora mismo. La vista la llama al montar:
 * el backend sólo emite `downloads-update` cuando la lista cambia, así que una
 * descarga que ya estaba bajando cuando se abrió el panel no llegaría por el bus.
 */
export const getActiveDownloads = (): Promise<DownloadActivity[]> =>
	invoke<DownloadActivity[]>('active_downloads');
