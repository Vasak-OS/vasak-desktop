<script setup lang="ts">
/**
 * La fila de sesión del centro de control, debajo de la tarjeta de usuario
 * (vasak-desktop#190, ancla #174): Bloquear, Cerrar sesión, Reiniciar, Apagar.
 *
 * - **Bloquear** no pregunta: le pide el bloqueo a logind por D-Bus y lo pone
 *   `vasak-lock-screen`, el bloqueo de siempre. Sin logind (o sin sesión
 *   gráfica) se ve no disponible, nunca roto.
 * - **Las otras tres** abren el diálogo de sesión, que pregunta antes de
 *   hacerlas —el mismo camino que los botones del menú—. El diálogo toma el
 *   foco y el centro se cierra solo al perderlo.
 *
 * Son dos `PowerActions` uno al lado del otro y no uno solo porque la librería
 * apaga el grupo entero (`disabled`), no una acción: con uno solo, sin logind
 * se apagarían también Cerrar sesión, Reiniciar y Apagar, que tienen su
 * propio diálogo. El espacio entre los dos es el mismo que entre los botones
 * de cada uno (`gap-2`), así que se ve una sola fila.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { type PowerAction, PowerActions } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { isLockScreenAvailable, lockScreen } from '@/services/session-lock.service';
import { hideControlCenter, toggleSessionPopup } from '@/services/window.service';
import { CONFIRMED_ACTIONS, dialogAction } from '@/tools/session-row';
import { logError } from '@/utils/logger';

const { t } = useI18n();

/** Empieza en no: hasta que logind conteste, el botón no promete nada. */
const lockAvailable = ref(false);

const LOCK: readonly PowerAction[] = ['lock'];

const lockLabels = computed(() => ({
	lock: lockAvailable.value
		? t('views.controlCenter.lock')
		: t('views.controlCenter.lockUnavailable'),
}));

const sessionLabels = computed(() => ({
	logout: t('views.menu.logout'),
	reboot: t('views.menu.reboot'),
	poweroff: t('views.menu.shutdown'),
}));

async function refreshAvailability(): Promise<void> {
	try {
		lockAvailable.value = await isLockScreenAvailable();
	} catch {
		lockAvailable.value = false;
	}
}

/**
 * Bloquea. El centro se cierra antes: al desbloquear no tiene que estar
 * abierto, y el bloqueo lo tapa todo igual.
 */
async function onLock(): Promise<void> {
	if (!lockAvailable.value) return;
	try {
		await hideControlCenter();
	} catch {
		/* ya estaba cerrado */
	}
	try {
		await lockScreen();
	} catch (error) {
		logError('[control-center] no se pudo bloquear la pantalla:', error);
		// Si logind se fue, que el botón lo diga en vez de fallar callado.
		await refreshAvailability();
	}
}

async function onSessionAction(action: PowerAction): Promise<void> {
	const dialog = dialogAction(action);
	if (dialog === null) return;
	try {
		await toggleSessionPopup(dialog);
	} catch (error) {
		logError('[control-center] no se pudo abrir el diálogo de sesión:', error);
	}
}

onMounted(refreshAvailability);
</script>

<template>
  <div class="flex min-w-0 flex-wrap items-start gap-2" data-session-actions>
    <PowerActions
      :actions="LOCK"
      :labels="lockLabels"
      :label="t('views.controlCenter.lock')"
      :disabled="!lockAvailable"
      button-variant="ghost"
      data-lock-action
      @action="onLock"
    />
    <PowerActions
      :actions="CONFIRMED_ACTIONS"
      :labels="sessionLabels"
      :label="t('views.controlCenter.session')"
      button-variant="ghost"
      data-confirmed-actions
      @action="onSessionAction"
    />
  </div>
</template>
