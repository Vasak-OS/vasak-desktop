<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import {
	detectDisplayServer as sysDetectDisplayServer,
	logout as sysLogout,
	reboot as sysReboot,
	shutdown as sysShutdown,
	suspend as sysSuspend,
} from '@/services/system.service';
import { useReactiveIcon } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

const currentWindow = getCurrentWindow();
const route = useRoute();
const action = ref<'shutdown' | 'reboot' | 'logout' | 'suspend'>('shutdown');
const leaving = ref(false);
const confirming = ref(false);
const closing = ref(false);

const actionImg = useReactiveIcon(
	computed(() => {
		switch (action.value) {
			case 'shutdown':
				return 'system-shutdown';
			case 'reboot':
				return 'system-reboot';
			case 'logout':
				return 'system-log-out';
			case 'suspend':
				return 'system-suspend';
		}
	})
);

const titleText = computed(() => {
	switch (action.value) {
		case 'shutdown':
			return 'Apagar el sistema';
		case 'reboot':
			return 'Reiniciar el sistema';
		case 'logout':
			return 'Cerrar sesión';
		case 'suspend':
			return 'Suspender el sistema';
	}
});

const confirmText = computed(() => {
	switch (action.value) {
		case 'shutdown':
			return 'Apagar';
		case 'reboot':
			return 'Reiniciar';
		case 'logout':
			return 'Cerrar sesión';
		case 'suspend':
			return 'Suspender';
	}
});

const descriptionText = computed(() => {
	switch (action.value) {
		case 'shutdown':
			return 'Se apagarán todos los programas y el sistema se detendrá.';
		case 'reboot':
			return 'El sistema se reiniciará. Asegúrate de guardar tu trabajo.';
		case 'logout':
			return 'Se cerrará tu sesión actual.';
		case 'suspend':
			return 'El sistema entrará en estado de suspensión de bajo consumo.';
	}
});

const closeAfterAnimation = () => {
	if (closing.value) return;
	closing.value = true;
	leaving.value = true;
	setTimeout(() => {
		try {
			currentWindow.close();
		} catch {
			/* window already closed */
		}
	}, 200);
};

const executeAction = async () => {
	confirming.value = true;
	try {
		const displayServer = await sysDetectDisplayServer();
		switch (action.value) {
			case 'shutdown':
				await sysShutdown();
				break;
			case 'reboot':
				await sysReboot();
				break;
			case 'logout':
				await sysLogout({ displayServer });
				break;
			case 'suspend':
				await sysSuspend({ displayServer });
				break;
		}
	} catch (error) {
		logError(`Error executing ${action.value}:`, error);
		confirming.value = false;
	}
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') {
		closeAfterAnimation();
	} else if (event.key === 'Enter' && !confirming.value) {
		executeAction();
	}
};

let unlistenAction: UnlistenFn | undefined;

onMounted(async () => {
	// The window is hidden rather than destroyed now, so the action can no
	// longer come only from the URL it was created with — it would keep showing
	// whatever was asked for the first time.
	listen<string>('session-action', (event) => {
		action.value = event.payload as 'shutdown' | 'reboot' | 'logout' | 'suspend';
	}).then((fn) => {
		unlistenAction = fn;
	});

	const queryAction = route.query.action as string;
	if (['shutdown', 'reboot', 'logout', 'suspend'].includes(queryAction)) {
		action.value = queryAction as 'shutdown' | 'reboot' | 'logout' | 'suspend';
	}
	document.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
	document.removeEventListener('keydown', onKeydown);
	unlistenAction?.();
});
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <div
      :class="['h-screen w-screen flex items-center justify-center bg-ui-bg/80 border border-ui-border rounded-corner-window overflow-hidden', { 'leave-active': leaving }]"
    >
      <div
        class="flex flex-col w-[380px]"
      >
        <div class="flex flex-col items-center gap-4 px-8 pt-8 pb-4">
          <div class="w-20 h-20 rounded-full bg-primary/15 flex items-center justify-center">
            <img :src="actionImg" :alt="titleText" class="w-12 h-12" />
          </div>
          <h2 class="text-xl font-bold text-vsk-text text-center">{{ titleText }}</h2>
          <p class="text-sm text-vsk-text/70 text-center leading-relaxed">{{ descriptionText }}</p>
        </div>

        <div class="flex gap-3 px-8 pb-8 pt-2">
          <button
            class="flex-1 px-5 py-3 rounded-corner border border-ui-border bg-ui-surface/50 hover:bg-ui-surface transition-colors text-sm font-medium text-vsk-text"
            @click="closeAfterAnimation"
            :disabled="confirming"
          >
            Cancelar
          </button>
          <button
            class="flex-1 px-5 py-3 rounded-corner bg-primary hover:bg-primary/90 transition-colors text-sm font-bold text-tx-on-primary flex items-center justify-center gap-2"
            @click="executeAction"
            :disabled="confirming"
          >
            <div
              v-if="confirming"
              class="w-4 h-4 border-2 border-tx-on-primary/30 border-t-tx-on-primary rounded-full animate-spin"
            />
            {{ confirmText }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@keyframes fade-scale-in {
  from {
    transform: scale(0.9);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes fade-scale-out {
  from {
    transform: scale(1);
    opacity: 1;
  }
  to {
    transform: scale(0.9);
    opacity: 0;
  }
}

.enter-active {
  animation: fade-scale-in 200ms ease-out;
}

.leave-active {
  animation: fade-scale-out 200ms ease-in;
}
</style>
