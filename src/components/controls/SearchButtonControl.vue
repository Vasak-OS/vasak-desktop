<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El botón que abre el lanzador.
 *
 * Abre `vasak-prism`, que es otro programa: la búsqueda global salió del
 * escritorio para poder quedarse residente —que es lo que la hace instantánea—
 * y crecer por su cuenta. Antes abría la ventana de búsqueda que vivía acá.
 *
 * `--toggle` y no el binario a secas: sin el argumento, cada clic dejaría un
 * proceso más. Con él, el que arranca le habla por D-Bus al que ya está y se
 * muere; y si no hay ninguno, se queda él. Es exactamente lo que hace el atajo
 * del teclado en `wayfire.ini`, y por eso se lanza el programa en vez de
 * hablarle al bus desde acá: una sola forma de abrirlo, y las dos pasan por la
 * misma lógica.
 *
 * El lanzamiento necesita permiso: `shell:allow-spawn` de
 * `capabilities/default.json` lista los binarios que esta ventana puede lanzar.
 * Sin `vasak-prism` en esa lista lo rechaza el propio Tauri, en tiempo de
 * ejecución y sólo en el paquete instalado.
 */

import { Command } from '@tauri-apps/plugin-shell';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const openSearch = async () => {
	try {
		const cmd = Command.create('vasak-prism', ['--toggle']);
		await cmd.spawn();
	} catch (error) {
		logError('Error opening search:', error);
	}
};
</script>

<template>
  <button
    @click="openSearch"
    class="p-2 rounded-corner bg-ui-bg/80 transition-all duration-500 h-17 w-17 group relative overflow-hidden hover:scale-105 active:scale-95 ring-2 ring-primary"
    :title="t('components.SearchButtonControl.openSearch')" :aria-label="t('components.SearchButtonControl.openSearch')">
    <!-- Overlay decorativo como ThemeToggle -->
    <div
      class="absolute inset-0 rounded-corner transition-all duration-500"
      :class="'bg-linear-to-br from-primary to-secondary'"
      style="opacity: 0"
    ></div>

    <ThemeIcon
      name="search"
      :size="48"
      :alt="t('components.SearchButtonControl.searchAlt')"
      class="m-auto transition-all duration-500 group-hover:scale-110 relative z-10 drop-shadow-lg group-hover:drop-shadow-xl"
    />
  </button>
</template>
