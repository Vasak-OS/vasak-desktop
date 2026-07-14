<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { onBeforeUnmount, onMounted, type Ref, ref } from 'vue';
import NotificationArea from '@/components/areas/control-center/NotificationArea.vue';
import UserControlCenterCard from '@/components/cards/UserControlCenterCard.vue';
import BluetoothControl from '@/components/controls/BluetoothControl.vue';
import BrightnessControl from '@/components/controls/BrightnessControl.vue';
import NetworkControl from '@/components/controls/NetworkControl.vue';
import SearchButtonControl from '@/components/controls/SearchButtonControl.vue';
import ThemeToggle from '@/components/controls/ThemeToggle.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import { toggleControlCenter } from '@/services/window.service';

const bluetoothInitialized: Ref<boolean> = ref(false);
const hasPlayedEntrance = ref(false);
const leaving = ref(false);

/** Track timeout handles for cleanup */
let closeTimeout: ReturnType<typeof setTimeout> | null = null;

const closeAfterAnimation = () => {
	if (leaving.value) return; // Prevent double-close
	leaving.value = true;

	// Cancel entrance animation if still running
	const main = document.querySelector('main');
	main?.getAnimations().forEach(a => a.cancel());

	// Play exit animation, then close window
	closeTimeout = setTimeout(() => {
		try { toggleControlCenter(); } catch { /* window already closed */ }
	}, 200);
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
	hasPlayedEntrance.value = true;
	bluetoothInitialized.value = await isBluetoothPluginInitialized();
	document.addEventListener('keydown', onKeydown);
	window.addEventListener('blur', onBlur);
});

onBeforeUnmount(() => {
	// Clear all timeouts
	if (closeTimeout !== null) {
		clearTimeout(closeTimeout);
		closeTimeout = null;
	}
	// Remove all event listeners registered during mount
	document.removeEventListener('keydown', onKeydown);
	window.removeEventListener('blur', onBlur);
});
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <main
      :class="['bg-ui-bg/80 h-screen w-screen rounded-corner flex flex-row flex-wrap justify-between p-1 border border-ui-border', { 'leave-active': leaving }]"
    >
      <div class="flex flex-col w-full gap-2 p-2">
        <UserControlCenterCard />
        <NotificationArea />
      </div>
      <div class="flex flex-wrap w-full justify-around items-end self-end p-2">
        <MusicWidget class="w-full" />
        <div class="flex justify-between gap-2 w-full">
          <SearchButtonControl />
          <NetworkControl />
          <BluetoothControl v-if="bluetoothInitialized" />
          <ThemeToggle />
        </div>
        <div class="flex flex-col gap-2 w-full mt-4">
          <BrightnessControl />
          <VolumeControl />
        </div>
      </div>
    </main>
  </Transition>
</template>

<style scoped>
@keyframes slide-in-right {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes slide-out-right {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(100%);
    opacity: 0;
  }
}

.enter-active {
  animation: slide-in-right 200ms ease-out;
}

.leave-active {
  animation: slide-out-right 200ms ease-in;
}
</style>
