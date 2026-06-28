<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import type { SystrayPopupPayload, TrayMenu } from '@/interfaces/tray';
import { getTrayPopupData, trayPopupClick } from '@/services/tray.service';
import { useIcon, useSymbol } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

const currentWindow = getCurrentWindow();
const data = ref<SystrayPopupPayload | null>(null);
const leaving = ref(false);
const fallbackIcon = useIcon('applications-other');
const checkIcon = useSymbol('object-select-symbolic');

const popupIcon = computed(() => {
	if (!data.value?.icon_data) return null;
	return `data:image/png;base64,${data.value.icon_data}`;
});

const popupSubtitle = computed(() => {
	return data.value?.tooltip || data.value?.service_name || 'Tray';
});

const itemCount = computed(() => data.value?.items?.length ?? 0);

type RenderedTrayItem = TrayMenu & { depth: number };

const renderItems = computed<RenderedTrayItem[]>(() => {
	const output: RenderedTrayItem[] = [];

	const appendItems = (items: TrayMenu[] | undefined, depth: number) => {
		for (const item of items ?? []) {
			output.push({ ...item, depth });
			if (item.children?.length) {
				appendItems(item.children, depth + 1);
			}
		}
	};

	appendItems(data.value?.items, 0);
	return output;
});

const closeAfterAnimation = () => {
	leaving.value = true;
	setTimeout(() => {
		try {
			currentWindow.close();
		} catch {
			/* window already closed */
		}
	}, 200);
};

const handleItemClick = async (item: TrayMenu) => {
	if (!item.enabled || item.type === 'separator') return;
	try {
		await trayPopupClick({ menuId: item.id });
	} catch (error) {
		logError('[TrayPopup] Error executing menu action:', error);
	}
	closeAfterAnimation();
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') {
		closeAfterAnimation();
	}
};

const onBlur = () => {
	closeAfterAnimation();
};

onMounted(async () => {
	try {
		const payload = await getTrayPopupData();
		data.value = payload;
		if (!payload?.items || payload.items.length === 0) {
			console.warn('[TrayPopup] No menu items available');
		}
	} catch (error) {
		logError('[TrayPopup] Error loading popup data:', error);
		await currentWindow.close();
		return;
	}
	document.addEventListener('keydown', onKeydown);
	window.addEventListener('blur', onBlur);
});

onBeforeUnmount(() => {
	document.removeEventListener('keydown', onKeydown);
	window.removeEventListener('blur', onBlur);
});
</script>

<template>
  <AppletFrame :close-fn="closeAfterAnimation">
    <div class="flex h-full flex-col gap-4">
      <section class="rounded-corner border border-ui-border bg-ui-surface/45 p-4 shadow-sm">
        <div class="flex items-start gap-4 min-w-0">
          <div class="w-14 h-14 rounded-corner border border-ui-border/70 bg-ui-surface/70 flex items-center justify-center overflow-hidden shrink-0">
            <img
              v-if="popupIcon"
              :src="popupIcon"
              class="w-full h-full object-contain p-2"
              alt="Tray icon"
            />
            <img
              v-else
              :src="fallbackIcon"
              class="w-full h-full object-contain p-2"
              alt="Tray icon"
            />
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 min-w-0">
              <h2 class="text-lg font-semibold text-vsk-text truncate">
                {{ data?.title || 'Tray' }}
              </h2>
              <span class="text-[10px] uppercase tracking-[0.18em] text-vsk-text/45 whitespace-nowrap">
                {{ itemCount }} items
              </span>
            </div>
            <p class="text-sm text-vsk-text/70 truncate mt-1">
              {{ popupSubtitle }}
            </p>
            <div class="mt-3 flex flex-wrap gap-2 text-xs">
              <span class="px-2.5 py-1 rounded-full border border-ui-border bg-ui-surface/55 text-vsk-text/75">
                {{ data?.status || 'Tray activo' }}
              </span>
              <span class="px-2.5 py-1 rounded-full border border-ui-border bg-ui-surface/55 text-vsk-text/65 truncate max-w-[280px]">
                {{ data?.service_name || 'service' }}
              </span>
            </div>
          </div>
        </div>
      </section>

      <section class="flex-1 min-h-0 rounded-corner border border-ui-border bg-ui-surface/35 p-4 overflow-hidden">
        <div class="flex items-center justify-between gap-2 mb-3">
          <h3 class="text-sm font-semibold text-vsk-text">Acciones del tray</h3>
          <span class="text-xs text-vsk-text/50">Click derecho del tray</span>
        </div>

        <div class="h-full overflow-y-auto pr-1 space-y-2">
          <div
            v-if="!data?.items?.length"
            class="rounded-corner border border-ui-border bg-ui-surface/50 px-4 py-6 text-center text-sm text-vsk-text/60"
          >
            No hay elementos disponibles para este tray.
          </div>

          <template v-for="item in renderItems" :key="item.id">
            <div
              v-if="item.type === 'separator'"
              class="mx-1 my-3 h-px bg-ui-border/70"
            />

            <button
              v-else
              type="button"
              :disabled="!item.enabled"
              :class="[
                'w-full rounded-corner border px-4 py-3 text-left transition-all duration-150',
                item.depth > 0 ? 'ml-6 w-[calc(100%-1.5rem)]' : '',
                item.enabled
                  ? 'border-ui-border bg-ui-surface/50 hover:bg-primary/12 hover:border-primary/40'
                  : 'border-ui-border/60 bg-ui-surface/30 cursor-default opacity-40',
                item.checked ? 'ring-1 ring-primary/35' : '',
              ]"
              @click="handleItemClick(item)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2 min-w-0" :style="item.depth > 0 ? { paddingLeft: '0.25rem' } : undefined">
                    <span class="text-sm font-medium text-vsk-text truncate">{{ item.label }}</span>
                    <img v-if="item.checked" :src="checkIcon" alt="✓" class="w-3.5 h-3.5" />
                  </div>
                  <p class="mt-1 text-xs text-vsk-text/55 truncate">
                    <span v-if="item.icon">{{ item.icon }}</span>
                  </p>
                </div>

                <div class="shrink-0 flex items-center gap-2 text-[10px] text-vsk-text/55 uppercase tracking-[0.14em]">
                  <span v-if="item.children?.length">submenu</span>
                  <span v-if="!item.enabled">disabled</span>
                </div>
              </div>
            </button>
          </template>
        </div>
      </section>
    </div>
  </AppletFrame>
</template>

<style scoped>
@keyframes scale-in {
  from {
    transform: scale(0.95);
    opacity: 0;
  }

  to {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes scale-out {
  from {
    transform: scale(1);
    opacity: 1;
  }

  to {
    transform: scale(0.95);
    opacity: 0;
  }
}

.enter-active {
  animation: scale-in 200ms ease-out;
}

.leave-active {
  animation: scale-out 200ms ease-in;
}
</style>
