<script lang="ts" setup>
import { homeDir } from '@tauri-apps/api/path';
import { Command } from '@tauri-apps/plugin-shell';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import type { FileEntry } from '@/interfaces/file';
import { getUserDirectories, loadDirectory } from '@/tools/file.controller';
import { logError } from '@/utils/logger';

/**
 * Los archivos del escritorio, ahora como widget.
 *
 * Antes ocupaban la pantalla entera y se prendían con un interruptor en
 * Ajustes. Como widget, se muestran o no según esté o no puesto en la
 * cuadrícula, y ocupan el lugar que uno les da: eso es lo que hace que el
 * interruptor de «mostrar archivos» ya no tenga sentido.
 *
 * Que los iconos vivan dentro de un recuadro con desplazamiento propio es la
 * diferencia importante: cuando había más archivos que pantalla, antes se
 * perdían fuera del borde.
 */
const { t } = useI18n();
const configStore = useConfigStore() as any;

const files = ref<FileEntry[]>([]);

const iconSize = computed(() => configStore.config?.desktop?.iconsize ?? 48);
const showHidden = computed(() => configStore.config?.desktop?.showhiddenfiles ?? false);

async function cargar() {
	try {
		const home = await homeDir();
		const carpetas = await getUserDirectories(home);
		const escritorio = carpetas.find((dir) => dir.xdgKey === 'XDG_DESKTOP_DIR');

		files.value = await loadDirectory(escritorio?.path ?? `${home}/Desktop`, showHidden.value);
	} catch (error) {
		logError(`No se pudieron leer los archivos del escritorio: ${error}`);
		files.value = [];
	}
}

async function abrir(file: FileEntry) {
	try {
		// Una carpeta va al gestor de archivos; lo demás, a la aplicación que le
		// corresponda según el sistema.
		const comando = file.isDirectory
			? Command.create('vasak-file-manager', [file.path])
			: Command.create('xdg-open', [file.path]);

		await comando.spawn();
	} catch (error) {
		logError(`No se pudo abrir ${file.path}: ${error}`);
	}
}

let recarga: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
	void cargar();
	// El escritorio cambia por fuera de la aplicación —se descarga algo, se
	// borra un archivo—, así que se relee cada tanto. Barato: es un directorio.
	recarga = setInterval(() => void cargar(), 10_000);
});

onUnmounted(() => {
	if (recarga) clearInterval(recarga);
});

watch(showHidden, () => void cargar());
</script>

<template>
	<div
		class="flex h-full w-full flex-col overflow-hidden rounded-corner border border-ui-border bg-ui-bg/60 backdrop-blur-md"
	>
		<div
			v-if="files.length === 0"
			class="flex h-full items-center justify-center p-3 text-center text-sm text-tx-muted"
		>
			{{ t('widgets.files.empty') }}
		</div>

		<div v-else class="flex-1 overflow-auto p-3">
			<div
				class="grid content-start gap-3"
				:style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${40 + iconSize}px, 1fr))` }"
			>
				<button
					v-for="file in files"
					:key="file.path"
					type="button"
					class="flex flex-col items-center justify-start rounded-corner p-2 transition-colors hover:bg-ui-surface/50"
					:title="file.name"
					@dblclick="abrir(file)"
				>
					<img
						v-if="file.icon"
						:src="file.icon"
						:alt="file.name"
						class="mb-1 shrink-0"
						:style="{ width: `${iconSize}px`, height: `${iconSize}px` }"
					/>
					<span
						class="max-w-full break-words px-1 text-center text-tx-main"
						:style="{ fontSize: `${Math.max(12, iconSize / 6)}px` }"
					>
						{{ file.name }}
					</span>
				</button>
			</div>
		</div>
	</div>
</template>
