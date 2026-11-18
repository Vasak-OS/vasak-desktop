import { computed, onMounted, onUnmounted, ref } from 'vue';
import type { LauncherEntryView } from '@/interfaces/tray';
import type { WindowInfo } from '@/interfaces/window';
import { getActiveDownloads } from '@/services/downloads.service';
import { getLauncherEntries } from '@/services/tray.service';
import { getWindows } from '@/services/window.service';
import { useSharedEvent } from '@/tools/event.bus';
import { launcherForApp } from '@/tools/tray-item';
import { logError } from '@/utils/logger';

/** Una aplicación que está trabajando ahora, para la cápsula de actividad. */
export interface ActivityProgress {
	/** El id de la ventana, para traerla al frente. */
	id: string;
	/** El título de la ventana, ya resuelto por el backend. */
	title: string;
	/** El nombre de icono del tema, ya resuelto por el backend. */
	icon: string;
	/** El avance, de 0 a 1. */
	progress: number;
}

/**
 * Una descarga del navegador en curso, sin ventana propia. El avance es
 * indeterminado (el parcial no trae el total), así que no lleva porcentaje: se
 * muestra que hay una descarga. La manda el backend por `downloads-update`.
 */
export interface DownloadActivity {
	/** El nombre del archivo parcial, estable mientras dura la descarga. */
	id: string;
	/** El nombre final, sin el sufijo del parcial. */
	title: string;
	/** La carpeta de descargas, para abrirla al tocar la actividad. */
	dir: string;
}

/**
 * La lista de descargas con su número de versión. La consulta `active_downloads`
 * y el evento `downloads-update` comparten la cuenta, así que la vista aplica
 * siempre el `seq` más alto: una respuesta vieja de la consulta no pisa un evento
 * más nuevo, ni al revés.
 */
export interface DownloadsSnapshot {
	seq: number;
	items: DownloadActivity[];
}

/**
 * Las aplicaciones con una tarea en curso, para la cápsula de actividad.
 *
 * Una «actividad» es una ventana cuya aplicación publica por
 * `com.canonical.Unity.LauncherEntry` un avance que no terminó (0 < avance < 1):
 * una copia del gestor de archivos, una descarga, una exportación. Se cruza la
 * lista de ventanas —que ya trae icono y título resueltos— con lo que cada
 * aplicación publicó, el mismo `launcherForApp` que usa `WindowsArea` para el
 * badge sobre cada botón de ventana. No hay un segundo camino a `LauncherEntry`.
 *
 * shortcut: sólo progreso de apps con ventana; una tarea de fondo sin ventana no
 * aparece. Subir cuando haga falta mostrarla, resolviendo icono y nombre desde el
 * `desktop_id`.
 */
export function useActivityProgress() {
	const windows = ref<WindowInfo[]>([]);
	const launcherEntries = ref<LauncherEntryView[]>([]);
	const downloads = ref<DownloadActivity[]>([]);
	// La última versión aplicada. El backend numera cada snapshot (la consulta y
	// el evento comparten la cuenta), así que sólo se aplica el más nuevo y nunca
	// uno viejo pisa a uno reciente. -1 para que el primero —aun el vacío, seq 0—
	// entre.
	let appliedDownloadsSeq = -1;

	const applyDownloads = (snapshot: DownloadsSnapshot | null): void => {
		if (!snapshot || snapshot.seq <= appliedDownloadsSeq) return;
		appliedDownloadsSeq = snapshot.seq;
		downloads.value = Array.isArray(snapshot.items) ? snapshot.items : [];
	};

	// `get_windows` puede contestar una lista fresca o una caché de hasta cinco
	// segundos, y el `window-delta` dispara otra lectura en paralelo: nada ordena
	// las respuestas. Se numera cada pedido y se aplica sólo si sigue siendo el
	// último, para que una respuesta vieja no pise a una nueva.
	let windowRequest = 0;
	const refreshWindows = async (): Promise<void> => {
		const request = ++windowRequest;
		try {
			const next = await getWindows();
			if (request === windowRequest) {
				windows.value = next;
			}
		} catch (error) {
			logError('[Activity] no se pudieron leer las ventanas:', error);
		}
	};

	// La lectura inicial de LauncherEntry puede resolver después de un
	// `launcher-entry-update` que ya trajo entradas más nuevas. Se cuenta cada
	// aviso, y la lectura se descarta si llegó uno mientras estaba en vuelo: así no
	// reemplaza lo nuevo con la foto vieja.
	let launcherVersion = 0;
	const refreshEntries = async (): Promise<void> => {
		const seen = launcherVersion;
		try {
			const next = await getLauncherEntries();
			if (seen === launcherVersion) {
				launcherEntries.value = next;
			}
		} catch (error) {
			logError('[Activity] no se pudo leer LauncherEntry:', error);
		}
	};

	const refreshDownloads = async (): Promise<void> => {
		try {
			applyDownloads(await getActiveDownloads());
		} catch (error) {
			logError('[Activity] no se pudieron leer las descargas:', error);
		}
	};

	/** Las apps con tarea en curso, la más avanzada —la que está por terminar— primero. */
	const progressApps = computed<ActivityProgress[]>(() => {
		const out: ActivityProgress[] = [];
		for (const win of windows.value) {
			const badge = launcherForApp(launcherEntries.value, win.app_id);
			const value = badge?.progress;
			if (value == null || value <= 0 || value >= 1) continue;
			out.push({ id: win.id, title: win.title, icon: win.icon, progress: value });
		}
		return out.sort((first, second) => second.progress - first.progress);
	});

	const topProgress = computed<ActivityProgress | null>(() => progressApps.value[0] ?? null);

	// Los mismos eventos que `WindowsArea`: la ventana que cambió y el avance que
	// publicó la app. El refresco de ventanas es completo —la lista es corta— para
	// no duplicar acá el `applyDelta` del panel.
	// shortcut: refetch completo de ventanas por cada delta; pasar a delta si pesara.
	const windowFeed = useSharedEvent('window-delta', () => {
		void refreshWindows();
	});
	const launcherFeed = useSharedEvent<LauncherEntryView[]>('launcher-entry-update', (entries) => {
		launcherVersion++;
		launcherEntries.value = Array.isArray(entries) ? entries : [];
	});
	// El backend emite cuando cambia la lista de descargas en curso. Sin ventana
	// que cruzar: es una actividad por sí misma. Se aplica por `seq` para que el
	// orden entre este evento y la consulta inicial no dependa de quién llega antes.
	useSharedEvent<DownloadsSnapshot>('downloads-update', applyDownloads);

	// La espera al enganche es asíncrona: el componente puede desmontarse antes de
	// que resuelva. Vue no cancela el `onMounted` ya suspendido, así que se marca el
	// desarme para no arrancar las lecturas de una instancia que ya no está.
	let disposed = false;
	onUnmounted(() => {
		disposed = true;
	});

	onMounted(async () => {
		// Esperar a que los listeners estén enganchados antes de la lectura inicial:
		// si no, un evento emitido entre la suscripción y el enganche se perdería y
		// la vista quedaría atrasada hasta el próximo cambio. Con esto la lectura ve
		// el estado ya enganchado y lo posterior llega por el evento.
		await Promise.all([windowFeed.ready, launcherFeed.ready]);
		if (disposed) return;
		await Promise.all([refreshWindows(), refreshEntries(), refreshDownloads()]);
	});

	return { progressApps, topProgress, downloads };
}
