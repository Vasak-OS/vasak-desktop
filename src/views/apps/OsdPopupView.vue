<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { useReactiveIcon } from '@/tools/composables/useReactiveIcon';

const currentWindow = getCurrentWindow();
const route = useRoute();
const iconName = ref('dialog-information');
const currentValue = ref(0);
const maximum = ref(100);
const label = ref('');

const { t } = useI18n();

/**
 * The backend sends a locale key, not text — it has no notion of the user's
 * language. Keys that carry a percentage use `{0}`, which we can fill from the
 * value and maximum that arrive alongside.
 */
const displayLabel = computed(() => {
	if (!label.value) return '';

	const translated = t(label.value);
	if (!translated.includes('{0}')) return translated;

	const max = maximum.value || 100;
	const percent = Math.round((currentValue.value / max) * 100);
	return translated.replace('{0}', String(percent));
});
const visible = ref(false);
const iconSrc = useReactiveIcon(iconName);

let hideTimeout: ReturnType<typeof setTimeout> | null = null;
let hideWindowTimeout: ReturnType<typeof setTimeout> | null = null;
let unlisten: (() => void) | null = null;

interface OsdPayload {
	icon: string;
	value: number;
	maximum: number;
	label: string;
}

function scheduleHide() {
	if (hideTimeout) clearTimeout(hideTimeout);
	if (hideWindowTimeout) clearTimeout(hideWindowTimeout);

	hideTimeout = setTimeout(() => {
		visible.value = false;
		hideWindowTimeout = setTimeout(() => {
			try {
				currentWindow.hide();
			} catch {
				/* window already hidden */
			}
		}, 200);
	}, 1500);
}

function updateOsd(data: OsdPayload) {
	iconName.value = data.icon;
	currentValue.value = data.value;
	maximum.value = data.maximum;
	label.value = data.label;
	visible.value = true;
	scheduleHide();
}

onMounted(async () => {
	const queryIcon = route.query.icon as string | undefined;
	const queryValue = route.query.value as string | undefined;
	const queryMaximum = route.query.maximum as string | undefined;
	const queryLabel = route.query.label as string | undefined;

	if (queryIcon && queryLabel) {
		updateOsd({
			icon: queryIcon,
			value: queryValue ? Number(queryValue) : 0,
			maximum: queryMaximum ? Number(queryMaximum) : 100,
			label: queryLabel,
		});
	}

	unlisten = await listen<OsdPayload>('osd:show', (event) => {
		updateOsd(event.payload);
	});
});

onUnmounted(() => {
	if (hideTimeout) clearTimeout(hideTimeout);
	if (hideWindowTimeout) clearTimeout(hideWindowTimeout);
	if (unlisten) unlisten();
});
</script>

<template>
	<Transition name="osd">
		<div
			v-if="visible"
			class="w-screen h-screen flex flex-col items-center justify-center gap-3 bg-ui-bg/80 border border-ui-border rounded-corner-window overflow-hidden px-8 py-6"
		>
			<div class="w-16 h-16 flex items-center justify-center">
				<img :src="iconSrc" :alt="displayLabel" class="w-14 h-14" />
			</div>
			<span class="text-sm font-medium text-tx-main text-center whitespace-nowrap">{{ displayLabel }}</span>
			<div
				v-if="maximum > 1"
				class="w-full h-1 bg-ui-surface rounded-corner overflow-hidden"
			>
				<div
					class="h-full bg-primary rounded-corner"
					:style="{ width: Math.min((currentValue / maximum) * 100, 100) + '%' }"
				/>
			</div>
		</div>
	</Transition>
</template>

<style scoped>
.osd-enter-active {
	transition: opacity 0.15s ease-out, transform 0.15s ease-out;
}

.osd-leave-active {
	transition: opacity 0.2s ease-in;
}

.osd-enter-from {
	opacity: 0;
	transform: scale(0.9);
}

.osd-leave-to {
	opacity: 0;
}
</style>
