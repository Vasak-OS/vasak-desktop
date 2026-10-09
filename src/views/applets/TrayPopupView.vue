<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	Kbd,
	ThemeIcon,
	TOAST_TONE_CLASSES,
} from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onMounted, ref } from 'vue';
import TrayPixmap from '@/components/buttons/TrayPixmap.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import type { SystrayPopupPayload, TrayMenu } from '@/interfaces/tray';
import { getTrayPopupData, trayPopupClick } from '@/services/tray.service';
import { dismissApplet } from '@/services/window.service';
import { useSharedEvent } from '@/tools/event.bus';
import { iconSource } from '@/tools/tray-item';
import {
	DISPOSITION_STYLE,
	entryCheck,
	entryRole,
	hasActions,
	menuFocusTarget,
	shortcutChords,
	shortcutLabel,
	trayMenuRows,
} from '@/tools/tray-menu';
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
 *
 * Cada entrada es un `DropdownMenuItem` de la librería y dibuja **sólo lo que
 * manda**: el icono (por nombre o el PNG de la aplicación), la etiqueta, el
 * atajo, la casilla o la opción de radio con su estado —indeterminado
 * incluido—, y la disposición con su tono. Una entrada sin icono no tiene un
 * hueco para el icono.
 *
 * Desde vue-libvasak 2.2.0 la marca es `checked` del ítem (la tilde o el
 * punto en su columna), la sangría es `inset` —una columna de icono por nivel
 * de submenú, y una más para alinear con las que tienen icono— y el atajo es
 * `Kbd`, una tecla por recuadro. Lo único que la librería no modela es el
 * indeterminado de dbusmenu: ver `entryCheck`.
 */

const { t } = useI18n();

const data = ref<SystrayPopupPayload | null>(null);
const menu = ref<HTMLElement | null>(null);

const rows = computed(() => trayMenuRows(data.value?.items));
const empty = computed(() => !hasActions(rows.value));

/** Si alguna entrada tiene icono o casilla: entonces todas reservan el lugar,
 * para que las etiquetas queden alineadas. Si ninguna tiene, nadie reserva. */
const hasLeadingColumn = computed(() =>
	rows.value.some((row) => row.kind === 'item' && (iconSource(row.item.icon) || row.item.toggle))
);

/** Si la entrada dibuja algo en la columna del icono: la marca, el indeterminado o su icono. */
const ownsLeadingColumn = (item: TrayMenu) => Boolean(item.toggle || iconSource(item.icon));

/**
 * Cuántas columnas de icono se corre la entrada: una por nivel de submenú, y
 * una más si no tiene nada que poner en la columna que reservan las demás.
 */
const entryInset = (item: TrayMenu, depth: number) =>
	depth + (hasLeadingColumn.value && !ownsLeadingColumn(item) ? 1 : 0);

/** El título de un submenú, con la misma sangría por nivel que sus entradas. */
const captionStyle = (depth: number) => ({ paddingLeft: `calc(0.75rem + ${depth * 1.75}rem)` });

/**
 * El `role` y el `aria-checked` sólo del indeterminado: el resto lo pone el
 * ítem de la librería a partir de `checked`. Van como atributos, que la raíz
 * del ítem recibe por encima de los suyos.
 */
const mixedAttrs = (item: TrayMenu): Record<string, string> => {
	if (!entryCheck(item).mixedIcon) return {};
	const { role, checked } = entryRole(item);
	return checked === undefined ? { role } : { role, 'aria-checked': checked };
};

const dispositionLabel = (item: TrayMenu) =>
	item.disposition ? t(`views.applets.tray.disposition.${item.disposition}`) : '';

const menuLabel = computed(() =>
	t('views.applets.tray.menuLabel').replace(
		'{0}',
		data.value?.title || data.value?.tooltip?.title || t('views.applets.tray.fallbackTitle')
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

const entries = (): HTMLElement[] => [
	...(menu.value?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? []),
];

const onKeydown = (event: KeyboardEvent) => {
	const buttons = entries();
	const from = buttons.indexOf(document.activeElement as HTMLElement);
	const target = menuFocusTarget(
		buttons.map((button) => button.getAttribute('aria-disabled') !== 'true'),
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

// El programa cambió su menú con el menú abierto (`ItemsPropertiesUpdated` o
// `LayoutUpdated`): se vuelve a pedir sin mover el foco.
useSharedEvent('tray-popup-update', async () => {
	try {
		data.value = await getTrayPopupData();
	} catch (error) {
		logError('[TrayPopup] Error refreshing popup data:', error);
	}
});
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
        class="flex h-8 items-center px-3 text-label-m text-tx-muted"
      >
        {{ t('views.applets.tray.noItems') }}
      </p>

      <template v-for="row in empty ? [] : rows" :key="row.kind === 'separator' ? row.key : `${row.kind}-${row.item.id}`">
        <DropdownMenuSeparator v-if="row.kind === 'separator'" />

        <DropdownMenuLabel
          v-else-if="row.kind === 'caption'"
          class="truncate"
          :style="captionStyle(row.depth)"
        >
          {{ row.item.label }}
        </DropdownMenuLabel>

        <DropdownMenuItem
          v-else
          v-bind="mixedAttrs(row.item)"
          :checked="entryCheck(row.item).checked"
          :toggle="entryCheck(row.item).toggle"
          :inset="entryInset(row.item, row.depth)"
          :disabled="!row.item.enabled"
          :class="row.item.disposition ? TOAST_TONE_CLASSES[DISPOSITION_STYLE[row.item.disposition].tone] : ''"
          @select="handleItemClick(row.item)"
        >
          <!-- La columna del icono: el indeterminado, que la librería no
               dibuja, o el icono que manda el programa (por nombre o en PNG).
               Una entrada con marca no lleva además su icono, como antes. -->
          <template v-if="entryCheck(row.item).mixedIcon || (!row.item.toggle && iconSource(row.item.icon))" #prefix>
            <ThemeIcon
              v-if="entryCheck(row.item).mixedIcon"
              :name="entryCheck(row.item).mixedIcon ?? ''"
              type="symbol"
              :size="16"
              alt=""
            />
            <TrayPixmap
              v-else-if="iconSource(row.item.icon)?.kind === 'pixmap'"
              :data="row.item.icon?.data"
              :size="16"
            />
            <ThemeIcon
              v-else
              :name="row.item.icon?.name ?? ''"
              :size="16"
              alt=""
            />
          </template>
          <span class="flex min-w-0 items-center gap-2">
            <span class="min-w-0 flex-1 truncate">{{ row.item.label }}</span>
            <ThemeIcon
              v-if="row.item.disposition"
              :name="DISPOSITION_STYLE[row.item.disposition].icon"
              type="symbol"
              :size="14"
              :alt="dispositionLabel(row.item)"
            />
          </span>
          <template v-if="shortcutLabel(row.item.shortcut)" #shortcut>
            <span data-tray-shortcut class="flex items-center gap-1">
              <span class="sr-only">{{ shortcutLabel(row.item.shortcut) }}</span>
              <template v-for="(chord, index) in shortcutChords(row.item.shortcut)" :key="index">
                <span v-if="index > 0" aria-hidden="true" class="text-label-xs">,</span>
                <Kbd :keys="chord" aria-hidden="true" />
              </template>
            </span>
          </template>
        </DropdownMenuItem>
      </template>
    </div>
  </AppletPopover>
</template>
