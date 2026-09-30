<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onMounted, ref } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import type { SystrayPopupPayload, TrayMenu } from '@/interfaces/tray';
import { getTrayPopupData, trayPopupClick } from '@/services/tray.service';
import { dismissApplet } from '@/services/window.service';
import { hasActions, menuFocusTarget, trayMenuRows } from '@/tools/tray-menu';
import { logError } from '@/utils/logger';

/**
 * El menú del clic derecho sobre un icono de la bandeja.
 *
 * Es un menú contextual y nada más: las entradas que publica el programa, del
 * tamaño de un menú. Hasta la 1.18 era una ficha de 700×620 con el icono, el
 * título, el estado y el nombre de servicio arriba, y las acciones abajo como
 * tarjetas: quien buscaba «Salir» encontraba la información del programa.
 *
 * El alto lo calcula el backend antes de abrir (`tray_menu_size`), con las
 * mismas medidas que las clases de acá: ver `tools/tray-menu.ts`.
 */

const { t } = useI18n();

const data = ref<SystrayPopupPayload | null>(null);
const menu = ref<HTMLElement | null>(null);

const rows = computed(() => trayMenuRows(data.value?.items));
const empty = computed(() => !hasActions(rows.value));

const menuLabel = computed(() =>
	t('views.applets.tray.menuLabel').replace(
		'{0}',
		data.value?.title || data.value?.tooltip || t('views.applets.tray.fallbackTitle')
	)
);

const close = () => dismissApplet('tray').catch(() => undefined);

const handleItemClick = async (item: TrayMenu) => {
	if (!item.enabled) return;
	try {
		await trayPopupClick({ menuId: item.id });
	} catch (error) {
		logError('[TrayPopup] Error executing menu action:', error);
	}
	void close();
};

const entries = (): HTMLButtonElement[] => [
	...(menu.value?.querySelectorAll<HTMLButtonElement>('[data-tray-entry]') ?? []),
];

const onKeydown = (event: KeyboardEvent) => {
	const buttons = entries();
	const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
	const target = menuFocusTarget(
		buttons.map((button) => !button.disabled),
		from,
		event.key
	);
	if (target === null) return;
	event.preventDefault();
	buttons[target]?.focus();
};

/**
 * Los datos del icono que se tocó.
 *
 * Se piden al montarse y cada vez que el applet vuelve a la vista: es el mismo
 * applet para todos los iconos de la bandeja, y esconderlo no lo desmonta.
 */
const loadData = async () => {
	try {
		data.value = await getTrayPopupData();
		// El foco adentro, como cualquier menú: así las flechas funcionan de una.
		await nextTick();
		menu.value?.focus();
	} catch (error) {
		logError('[TrayPopup] Error loading popup data:', error);
		void close();
	}
};

onMounted(loadData);
</script>

<template>
  <AppletPopover applet="tray" compact @shown="loadData">
    <div
      ref="menu"
      role="menu"
      tabindex="-1"
      :aria-label="menuLabel"
      class="h-full overflow-y-auto outline-none"
      @keydown="onKeydown"
    >
      <p
        v-if="empty"
        class="flex h-8 items-center px-3 text-sm text-tx-muted"
      >
        {{ t('views.applets.tray.noItems') }}
      </p>

      <template v-for="row in rows" :key="row.kind === 'separator' ? row.key : `${row.kind}-${row.item.id}`">
        <div
          v-if="row.kind === 'separator'"
          role="separator"
          class="mx-2 my-1 h-px bg-ui-border"
        />

        <p
          v-else-if="row.kind === 'caption'"
          role="presentation"
          class="flex h-7 items-center truncate px-3 text-xs font-semibold text-tx-muted"
          :style="{ paddingLeft: `${0.75 + row.depth}rem` }"
        >
          {{ row.item.label }}
        </p>

        <button
          v-else
          data-tray-entry
          type="button"
          :role="row.item.checked === undefined || row.item.checked === null ? 'menuitem' : 'menuitemcheckbox'"
          :aria-checked="row.item.checked ?? undefined"
          :disabled="!row.item.enabled"
          class="flex h-8 w-full items-center gap-2 rounded-corner-sm pr-3 text-left text-sm text-tx-main transition-colors hover:bg-primary hover:text-tx-on-primary focus-visible:bg-primary focus-visible:text-tx-on-primary focus-visible:outline-none disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-tx-main"
          :style="{ paddingLeft: `${0.75 + row.depth}rem` }"
          @click="handleItemClick(row.item)"
        >
          <span class="min-w-0 flex-1 truncate">{{ row.item.label }}</span>
          <ThemeIcon
            v-if="row.item.checked"
            name="object-select-symbolic"
            type="symbol"
            :size="14"
            alt=""
          />
        </button>
      </template>
    </div>
  </AppletPopover>
</template>
