import { computed, onMounted, ref } from 'vue';
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

	const refreshWindows = async (): Promise<void> => {
		try {
			windows.value = await getWindows();
		} catch (error) {
			logError('[Activity] no se pudieron leer las ventanas:', error);
		}
	};

	const refreshEntries = async (): Promise<void> => {
		try {
			launcherEntries.value = await getLauncherEntries();
		} catch (error) {
			logError('[Activity] no se pudo leer LauncherEntry:', error);
		}
	};

	const refreshDownloads = async (): Promise<void> => {
		try {
			downloads.value = await getActiveDownloads();
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
	useSharedEvent('window-delta', () => {
		void refreshWindows();
	});
	useSharedEvent<LauncherEntryView[]>('launcher-entry-update', (entries) => {
		launcherEntries.value = Array.isArray(entries) ? entries : [];
	});
	// El backend sólo emite cuando cambia la lista de descargas en curso, así que
	// se usa tal cual. Sin ventana que cruzar: es una actividad por sí misma.
	useSharedEvent<DownloadActivity[]>('downloads-update', (list) => {
		downloads.value = Array.isArray(list) ? list : [];
	});

	onMounted(async () => {
		await Promise.all([refreshWindows(), refreshEntries(), refreshDownloads()]);
	});

	return { progressApps, topProgress, downloads };
}
