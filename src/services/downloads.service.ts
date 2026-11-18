import { invoke } from '@tauri-apps/api/core';
import type { DownloadsSnapshot } from '@/tools/composables/useActivityProgress';

/**
 * El snapshot de las descargas del navegador en curso ahora mismo. La vista lo
 * llama al montar: el backend sólo emite `downloads-update` cuando la lista
 * cambia, así que una descarga que ya estaba bajando cuando se abrió el panel no
 * llegaría por el bus. El `seq` la deja ordenar esta respuesta contra los eventos.
 */
export const getActiveDownloads = (): Promise<DownloadsSnapshot> =>
	invoke<DownloadsSnapshot>('active_downloads');
