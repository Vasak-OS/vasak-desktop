<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, PageDots, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import AppTile from '@/components/areas/menu/AppTile.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import type { MenuApp } from '@/tools/menu-favorites';
import { clampPage, gridTracks, pageCount as pageCountOf } from '@/tools/menu-grid';

/**
 * La variante en grilla: las categorías como píldoras centradas arriba y las
 * aplicaciones de la elegida como una cuadrícula de mosaicos **paginada**
 * (vasak-desktop#203, bug #206). El buscador lo pone el marco (`MenuView`).
 *
 * La cuadrícula llena el espacio disponible sin barras de desplazamiento: cuántas
 * columnas y filas entran lo mide un `ResizeObserver` sobre el área —no un punto
 * de corte del viewport, que en WebKitGTK no avisa—, y de ahí sale el tamaño de
 * página; los puntos de abajo (`PageDots`) cambian de página. En angosto entran
 * menos por fila y hay más páginas, sin cortar ni pisar nada.
 *
 * Las categorías NO son `CategoryMenuPill` (ésa se estira a su celda, `h/w-full`,
 * y en una fila se apilaba a lo ancho tapando todo): son píldoras a contenido, la
 * forma de un `Chip` de Once UI con el estado elegido marcado.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { categorySelected, appsOfCategory, categoryEntries } = props.controller;

const { t } = useI18n();

const apps = computed<MenuApp[]>(() => appsOfCategory.value as MenuApp[]);

const categories = computed(() => {
	const { all, others } = categoryEntries.value;
	return all ? [all, ...others] : others;
});

// ── Paginado medido ──────────────────────────────────────────────────────────
/** Medidas del mosaico, en px: ancho mínimo de columna y alto de fila, con el hueco. */
const COL_MIN = 88;
const ROW_H = 92;
const GAP = 12;

const area = ref<HTMLElement | null>(null);
const columns = ref(1);
const rows = ref(1);
let observer: ResizeObserver | undefined;

function measure() {
	const el = area.value;
	if (!el) return;
	const tracks = gridTracks(el.clientWidth, el.clientHeight, COL_MIN, ROW_H, GAP);
	columns.value = tracks.columns;
	rows.value = tracks.rows;
}

onMounted(() => {
	observer = new ResizeObserver(measure);
});
onBeforeUnmount(() => observer?.disconnect());

// El área está dentro de un `v-if` (hay aplicaciones): puede aparecer después
// del montaje —cuando el menú termina de cargar—, así que el observador se
// engancha cuando el elemento existe, no en `onMounted`. En WebKitGTK ni
// `matchMedia` ni `resize` avisan: mide el `ResizeObserver` sobre el área.
watch(
	area,
	(el, previous) => {
		if (previous) observer?.unobserve(previous);
		if (el) {
			observer?.observe(el);
			measure();
		}
	},
	{ flush: 'post' }
);

const pageSize = computed(() => Math.max(1, columns.value * rows.value));
const pageCount = computed(() => pageCountOf(apps.value.length, pageSize.value));

const page = ref(0);
const pageApps = computed(() =>
	apps.value.slice(page.value * pageSize.value, page.value * pageSize.value + pageSize.value)
);

/** El estilo de la cuadrícula: columnas y filas fijas que miden lo que entra. */
const gridStyle = computed(() => ({
	gridTemplateColumns: `repeat(${columns.value}, minmax(0, 1fr))`,
	gridAutoRows: `${ROW_H}px`,
}));

// Al cambiar de categoría se vuelve a la primera página; y la página se acota si
// deja de existir (menos aplicaciones, o entran más por página).
watch(categorySelected, () => {
	page.value = 0;
});
watch(pageCount, (count) => {
	page.value = clampPage(page.value, count);
});
</script>

<template>
  <div class="@container flex min-h-0 flex-1 flex-col gap-4">
    <!-- Categorías a contenido, centradas, con la elegida marcada. -->
    <div role="menu" :aria-label="t('views.menu.categories')" class="flex shrink-0 flex-wrap items-center justify-center gap-2">
      <button
        v-for="([key, value]) in categories"
        :key="key"
        type="button"
        :aria-pressed="key === categorySelected"
        class="inline-flex h-8 shrink-0 items-center gap-2 rounded-corner-full border px-3 text-label-xs transition-colors duration-200 ease-ui focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ui-focus"
        :class="key === categorySelected
          ? 'border-primary bg-ui-selected-accent font-semibold text-tx-main'
          : 'border-ui-line-weak bg-ui-surface/70 text-tx-main hover:bg-ui-hover active:bg-ui-pressed active:duration-100'"
        @click="categorySelected = key">
        <ThemeIcon :name="value.icon" :size="16" alt="" class="shrink-0" />
        <span class="truncate">{{ t(value.description) }}</span>
      </button>
    </div>

    <EmptyState
      v-if="apps.length === 0"
      class="flex-1"
      :title="t('views.menu.noApps')"
      icon="application-x-executable"
    />
    <template v-else>
      <!-- El área que se mide y se pagina: sin desplazamiento, una página llena. -->
      <div
        ref="area"
        role="menu"
        :aria-label="t('views.menu.applications')"
        class="grid min-h-0 flex-1 content-start gap-3 overflow-hidden"
        :style="gridStyle"
      >
        <AppTile v-for="app in pageApps" :key="app.path" :app="app" />
      </div>

      <PageDots
        v-if="pageCount > 1"
        class="shrink-0 justify-center"
        :count="pageCount"
        v-model="page"
        :label="t('views.menu.applications')"
      />
    </template>
  </div>
</template>
