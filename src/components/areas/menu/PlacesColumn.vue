<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
import { homeDir } from '@tauri-apps/api/path';
import { Command } from '@tauri-apps/plugin-shell';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ListGroup, ListRow } from '@vasakgroup/vue-libvasak';
import { onMounted, ref } from 'vue';
import { getUserDirectories } from '@/tools/file.controller';
import { buildPlaces, type MenuPlace } from '@/tools/menu-places';
import { logError } from '@/utils/logger';

/**
 * La columna de lugares del menú (vasak-desktop#203): Inicio, Documentos,
 * Descargas y Papelera, con iconos del tema. Cada uno abre con el gestor de
 * archivos del sistema, el mismo que usa el widget de archivos.
 *
 * El conjunto es fijo y las rutas salen de `menu-places.ts`, que es lo que se
 * prueba; acá sólo se leen las carpetas del usuario y se dibuja la lista.
 */
const { t } = useI18n();

const places = ref<MenuPlace[]>([]);

const loadPlaces = async () => {
	try {
		const home = await homeDir();
		const userDirs = await getUserDirectories(home);
		places.value = buildPlaces(home, userDirs);
	} catch (error) {
		logError('No se pudieron leer los lugares del menú:', error);
		places.value = [];
	}
};

/** Abre el lugar con el gestor de archivos (la papelera va como `trash:///`). */
const openPlace = async (place: MenuPlace) => {
	try {
		await Command.create('vasak-file-manager', [place.path]).spawn();
	} catch (error) {
		logError(`No se pudo abrir ${place.path}:`, error);
	}
};

onMounted(loadPlaces);
</script>

<template>
  <ListGroup role="group" :label="t('views.menu.places.title')" :divided="false">
    <ListRow
      v-for="place in places"
      :key="place.id"
      role="button"
      :icon="place.icon"
      :title="t(place.labelKey)"
      @click="openPlace(place)"
    />
  </ListGroup>
</template>
