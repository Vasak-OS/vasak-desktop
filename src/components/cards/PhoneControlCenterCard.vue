<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	ListRow,
	Panel,
	SettingRow,
	StatusDot,
	type StatusDotTone,
	SwitchToggle,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import type { ConnectDevice, ConnectRunningApp, ConnectWebcamState } from '@/interfaces/connect';
import {
	connectWebcamState,
	listConnectCameras,
	listConnectDevices,
	listConnectRunning,
	startConnectWebcam,
	stopConnectApp,
	stopConnectWebcam,
	toggleConnectMenu,
} from '@/services/connect.service';
import { useSharedEvent } from '@/tools/event.bus';
import {
	defaultCamera,
	defaultSize,
	isActiveOn,
	switchEnabled,
	webcamDiagnosis,
} from '@/tools/webcam';

/**
 * The phone's state, in the notification centre.
 *
 * Renders nothing when there is no phone. A card explaining that a feature is
 * unavailable, permanently, in the panel people open to read notifications, is
 * worse than no card.
 */
const { t } = useI18n();

const devices: Ref<ConnectDevice[]> = ref([]);
const running: Ref<ConnectRunningApp[]> = ref([]);

const device = computed(() => devices.value[0]);

/** El punto del estado: listo, sin autorizar, o en camino. */
const deviceTone = computed<StatusDotTone>(() => {
	if (device.value?.state === 'ready') return 'success';
	if (device.value?.state === 'unauthorized') return 'warning';
	return 'neutral';
});

const webcam: Ref<ConnectWebcamState | null> = ref(null);
const togglingWebcam = ref(false);
const webcamError = ref('');
const webcamStateError = ref('');

/**
 * Relee lo que dice el demonio sobre la cámara.
 *
 * **Un fallo de lectura no se convierte en un estado.** Se conserva el último
 * conocido en lugar de dar la cámara por apagada: si estaba encendida, el
 * interruptor tiene que seguir pudiendo apagarla. Y sobre todo no se finge un
 * estado vacío, porque un dispositivo vacío ya significa algo —falta el módulo
 * v4l2loopback, se arregla con un `modprobe` o reiniciando— y decir eso por un
 * error de bus manda a alguien a reiniciar al vacío.
 */
const refreshWebcam = async () => {
	try {
		webcam.value = await connectWebcamState();
		webcamStateError.value = '';
	} catch (reason) {
		webcamStateError.value = String(reason);
	}
};

const refresh = async () => {
	devices.value = await listConnectDevices();
	running.value = devices.value.length > 0 ? await listConnectRunning() : [];
	// Sólo con un teléfono a la vista: sin ninguno la tarjeta no se dibuja, y
	// preguntar por la webcam sería una llamada al bus para nadie.
	if (devices.value.length > 0) await refreshWebcam();
};

const close = async (app: ConnectRunningApp) => {
	await stopConnectApp(app.serial, app.package);
	running.value = await listConnectRunning();
};

// ── La cámara como webcam ───────────────────────────────────────────────────

/** Si la está usando este teléfono. */
const webcamActive = computed(() => isActiveOn(webcam.value, device.value?.serial));

/**
 * Cuándo se muestra la fila de la webcam.
 *
 * Con el teléfono listo, y **además** mientras esta cámara esté transmitiendo
 * aunque deje de estarlo: si el teléfono se bloquea a mitad de una llamada, la
 * fila se llevaría con ella el único interruptor que puede apagar la cámara.
 */
const showWebcam = computed(() => device.value?.state === 'ready' || webcamActive.value);

const webcamSwitchEnabled = computed(() =>
	switchEnabled({
		state: webcam.value,
		serial: device.value?.serial,
		phoneReady: device.value?.state === 'ready',
		inProgress: togglingWebcam.value,
	})
);

/**
 * La línea que acompaña al interruptor.
 *
 * Siempre dice algo: cuando está apagada, que hay que prenderla antes de abrir
 * la videollamada. Eso no es un consejo de más — el módulo va con
 * `exclusive_caps=1`, así que «VasakOS Phone» no aparece en Zoom, Firefox ni
 * OBS hasta que el puente está transmitiendo, y esas aplicaciones enumeran las
 * cámaras al arrancar. Prenderla después significa cerrar y reabrir la llamada.
 */
const webcamDetail = computed(() => {
	if (togglingWebcam.value) return t('views.connect.webcamWorking');

	switch (webcamDiagnosis(webcam.value, device.value?.serial)) {
		// Callarse hasta saber: sin estado leído no se puede afirmar nada, y el
		// que estaría más a mano —«falta el módulo»— manda a reiniciar el equipo.
		case 'unknown':
			return '';
		case 'no-module':
			return t('views.connect.webcamNoModule');
		case 'busy':
			return t('views.connect.webcamBusy');
		case 'active':
			return t('views.connect.webcamActive').replace('{0}', webcam.value?.device ?? '');
		default:
			return t('views.connect.webcamHint');
	}
});

const toggleWebcam = async (turnOn: boolean) => {
	const serial = device.value?.serial;
	if (!serial || togglingWebcam.value) return;

	togglingWebcam.value = true;
	webcamError.value = '';

	try {
		if (!turnOn) {
			// `StopWebcam` no lleva serial: corta lo que esté transmitiendo,
			// porque el dispositivo de vídeo admite un solo productor. Así que se
			// relee antes de cortar — si entre que se dibujó el interruptor y el
			// clic la cámara pasó a ser de otro teléfono, apagaríamos la de él.
			await refreshWebcam();
			if (!isActiveOn(webcam.value, serial)) return;
			await stopConnectWebcam();
		} else {
			// Las cámaras se piden acá y no al abrir la tarjeta: la primera
			// consulta hace que scrcpy le pregunte al teléfono y tarda, y la
			// mayoría de las veces que alguien abre el centro de notificaciones
			// no viene a prender la webcam.
			const cameras = await listConnectCameras(serial);
			const chosen = defaultCamera(cameras);
			if (!chosen) {
				webcamError.value = t('views.connect.webcamNoCameras');
				return;
			}
			// El tamaño se elige acá y no se deja en manos del teléfono: sin
			// pedirle uno, elige el máximo de su sensor, y ése no pasa por su
			// propio codificador. Los cuadros por segundo sí quedan a su
			// criterio, y los modos finos se eligen en Ajustes.
			//
			// Si el teléfono no enumeró ningún tamaño usable, `tamanioPorDefecto`
			// devuelve la cadena vacía y **se intenta igual**, que es lo que el
			// demonio entiende como «elegí vos». No se aborta a propósito: sin
			// lista no hay nada mejor que pedir, y negarse convertiría un
			// arranque que quizá funciona en uno que seguro no. Si falla, el
			// motivo lo pone el teléfono y se ve acá abajo.
			await startConnectWebcam(serial, chosen.id, defaultSize(chosen));
		}
	} catch (reason) {
		// El demonio explica bien sus fallos —falta el módulo, otra aplicación
		// tiene la cámara, el teléfono se bloqueó— y perder ese texto es lo que
		// vuelve incontestable un «no prendió».
		webcamError.value = String(reason);
	} finally {
		togglingWebcam.value = false;
		await refreshWebcam();
	}
};

onMounted(refresh);

useSharedEvent('connect-device-added', refresh);
useSharedEvent('connect-device-changed', refresh);
useSharedEvent('connect-device-removed', refresh);
useSharedEvent('connect-app-closed', refresh);
// La señal del demonio, y no un sondeo: el stream puede terminar sin que nadie
// lo haya pedido —el teléfono se bloquea, otra de sus aplicaciones se queda con
// la cámara— y un interruptor que siga diciendo «encendido» después de eso hace
// creer que hay una cámara alimentando la llamada.
useSharedEvent<ConnectWebcamState>('connect-webcam-changed', (state) => {
	webcam.value = state;
});
</script>

<template>
  <Panel v-if="device" padding="sm" class="gap-2">
    <!-- El teléfono: abre su menú. La fila es `ListRow` de la librería, con el
         estado en un `StatusDot` y no en un punto pintado a mano. -->
    <ListRow
      role="button"
      icon="smartphone"
      @click="toggleConnectMenu()"
    >
      <span class="flex min-w-0 flex-col">
        <span class="truncate text-label-m font-semibold">{{ device.model }}</span>
        <span class="truncate text-body-xs font-normal text-tx-muted">
          <template v-if="device.state === 'unauthorized'">{{ t('views.connect.unauthorized') }}</template>
          <template v-else-if="device.state === 'ready'">
            {{ device.transport === 'usb' ? 'USB' : device.address }}
            <template v-if="running.length > 0">
              · {{ t('views.connect.openApps').replace('{0}', String(running.length)) }}
            </template>
          </template>
          <template v-else>{{ t('views.connect.connecting') }}</template>
        </span>
      </span>
      <template #trailing>
        <StatusDot :tone="deviceTone" size="md" />
      </template>
    </ListRow>

    <!-- The open windows, with a way to close them. A window whose app is on a
         virtual display is easy to lose behind others, and this is the only
         place that knows they exist. -->
    <ul v-if="running.length > 0" class="flex flex-col gap-1">
      <li v-for="app in running" :key="app.package">
        <ListRow :title="app.label" truncate class="py-1">
          <template #trailing>
            <ActionButton
              :label="t('views.connect.close')"
              variant="ghost"
              size="sm"
              @click="close(app)"
            />
          </template>
        </ListRow>
      </li>
    </ul>

    <!-- La cámara del teléfono como webcam del sistema.
         Va acá y no en Ajustes porque tiene que prenderse *antes* de abrir la
         videollamada, y ésta es la única superficie que aparece exactamente
         cuando hay un teléfono enchufado. La elección de cámara, resolución y
         cuadros por segundo va en Ajustes, que es donde entran tres selectores. -->
    <div v-if="showWebcam" class="border-t border-ui-line-weak px-3 pt-2">
      <SettingRow :label="t('views.connect.webcam')">
        <SwitchToggle
          :label="t('views.connect.webcam')"
          :model-value="webcamActive"
          :disabled="!webcamSwitchEnabled"
          @update:model-value="toggleWebcam"
        />
        <!-- El error de una acción primero, y el de la lectura del estado
             después: los dos son texto del demonio y ninguno se puede
             reemplazar por el consejo del módulo, que sería un diagnóstico
             inventado. -->
        <template #footer>
          <p v-if="webcamError || webcamStateError" class="text-body-xs text-status-error">
            {{ webcamError || webcamStateError }}
          </p>
          <p v-else-if="webcamDetail" class="text-body-xs text-tx-muted">{{ webcamDetail }}</p>
        </template>
      </SettingRow>
    </div>
  </Panel>
</template>
