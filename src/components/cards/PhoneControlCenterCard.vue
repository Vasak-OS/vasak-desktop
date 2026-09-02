<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, type Ref, ref } from 'vue';
import SwitchToggle from '@/components/forms/SwitchToggle.vue';
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
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import {
	camaraPorDefecto,
	diagnosticoWebcam,
	encendidaEn,
	interruptorHabilitado,
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

const { phoneIcon } = useIcons({ phoneIcon: 'smartphone' });

const device = computed(() => devices.value[0]);

const webcam: Ref<ConnectWebcamState | null> = ref(null);
const cambiandoWebcam = ref(false);
const errorWebcam = ref('');
const errorEstadoWebcam = ref('');

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
const refrescarWebcam = async () => {
	try {
		webcam.value = await connectWebcamState();
		errorEstadoWebcam.value = '';
	} catch (reason) {
		errorEstadoWebcam.value = String(reason);
	}
};

const refresh = async () => {
	devices.value = await listConnectDevices();
	running.value = devices.value.length > 0 ? await listConnectRunning() : [];
	// Sólo con un teléfono a la vista: sin ninguno la tarjeta no se dibuja, y
	// preguntar por la webcam sería una llamada al bus para nadie.
	if (devices.value.length > 0) await refrescarWebcam();
};

const close = async (app: ConnectRunningApp) => {
	await stopConnectApp(app.serial, app.package);
	running.value = await listConnectRunning();
};

// ── La cámara como webcam ───────────────────────────────────────────────────

/** Si la está usando este teléfono. */
const webcamEncendida = computed(() => encendidaEn(webcam.value, device.value?.serial));

/**
 * Cuándo se muestra la fila de la webcam.
 *
 * Con el teléfono listo, y **además** mientras esta cámara esté transmitiendo
 * aunque deje de estarlo: si el teléfono se bloquea a mitad de una llamada, la
 * fila se llevaría con ella el único interruptor que puede apagar la cámara.
 */
const mostrarWebcam = computed(() => device.value?.state === 'ready' || webcamEncendida.value);

const interruptorHabilitadoAhora = computed(() =>
	interruptorHabilitado({
		estado: webcam.value,
		serial: device.value?.serial,
		telefonoListo: device.value?.state === 'ready',
		enCurso: cambiandoWebcam.value,
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
const detalleWebcam = computed(() => {
	if (cambiandoWebcam.value) return t('views.connect.webcamWorking');

	switch (diagnosticoWebcam(webcam.value, device.value?.serial)) {
		// Callarse hasta saber: sin estado leído no se puede afirmar nada, y el
		// que estaría más a mano —«falta el módulo»— manda a reiniciar el equipo.
		case 'desconocido':
			return '';
		case 'sin-modulo':
			return t('views.connect.webcamNoModule');
		case 'ocupada':
			return t('views.connect.webcamBusy');
		case 'encendida':
			return t('views.connect.webcamActive').replace('{0}', webcam.value?.device ?? '');
		default:
			return t('views.connect.webcamHint');
	}
});

const alternarWebcam = async (encender: boolean) => {
	const serial = device.value?.serial;
	if (!serial || cambiandoWebcam.value) return;

	cambiandoWebcam.value = true;
	errorWebcam.value = '';

	try {
		if (!encender) {
			// `StopWebcam` no lleva serial: corta lo que esté transmitiendo,
			// porque el dispositivo de vídeo admite un solo productor. Así que se
			// relee antes de cortar — si entre que se dibujó el interruptor y el
			// clic la cámara pasó a ser de otro teléfono, apagaríamos la de él.
			await refrescarWebcam();
			if (!encendidaEn(webcam.value, serial)) return;
			await stopConnectWebcam();
		} else {
			// Las cámaras se piden acá y no al abrir la tarjeta: la primera
			// consulta hace que scrcpy le pregunte al teléfono y tarda, y la
			// mayoría de las veces que alguien abre el centro de notificaciones
			// no viene a prender la webcam.
			const camaras = await listConnectCameras(serial);
			const elegida = camaraPorDefecto(camaras);
			if (!elegida) {
				errorWebcam.value = t('views.connect.webcamNoCameras');
				return;
			}
			// Sin tamaño ni cuadros por segundo: que elija el teléfono. Los
			// modos reales se eligen en Ajustes, donde hay lugar para mostrarlos.
			await startConnectWebcam(serial, elegida.id);
		}
	} catch (reason) {
		// El demonio explica bien sus fallos —falta el módulo, otra aplicación
		// tiene la cámara, el teléfono se bloqueó— y perder ese texto es lo que
		// vuelve incontestable un «no prendió».
		errorWebcam.value = String(reason);
	} finally {
		cambiandoWebcam.value = false;
		await refrescarWebcam();
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
useSharedEvent<ConnectWebcamState>('connect-webcam-changed', (estado) => {
	webcam.value = estado;
});
</script>

<template>
  <div v-if="device" class="flex flex-col gap-2 rounded-corner bg-ui-surface/40 p-3 text-tx-main">
    <button type="button" class="flex items-center gap-3 text-left" @click="toggleConnectMenu()">
      <img :src="phoneIcon" alt="" class="h-8 w-8 shrink-0" />
      <div class="min-w-0 flex-1">
        <p class="truncate font-semibold text-tx-main">{{ device.model }}</p>
        <p class="truncate text-xs text-tx-muted">
          <span v-if="device.state === 'unauthorized'" class="text-status-warning">
            {{ t('views.connect.unauthorized') }}
          </span>
          <span v-else-if="device.state === 'ready'">
            {{ device.transport === 'usb' ? 'USB' : device.address }}
            <template v-if="running.length > 0">
              · {{ t('views.connect.openApps').replace('{0}', String(running.length)) }}
            </template>
          </span>
          <span v-else>{{ t('views.connect.connecting') }}</span>
        </p>
      </div>
      <div
        class="h-2.5 w-2.5 shrink-0 rounded-full"
        :class="{
          'bg-status-success': device.state === 'ready',
          'bg-status-warning': device.state === 'unauthorized',
          'bg-tx-muted': device.state !== 'ready' && device.state !== 'unauthorized',
        }"
      ></div>
    </button>

    <!-- The open windows, with a way to close them. A window whose app is on a
         virtual display is easy to lose behind others, and this is the only
         place that knows they exist. -->
    <ul v-if="running.length > 0" class="space-y-1">
      <li
        v-for="app in running"
        :key="app.package"
        class="flex items-center gap-2 rounded-corner px-2 py-1 text-sm hover:bg-primary/10"
      >
        <span class="min-w-0 flex-1 truncate text-tx-main">{{ app.label }}</span>
        <button
          type="button"
          :title="t('views.connect.close')"
          class="shrink-0 rounded-corner px-2 text-xs text-primary hover:bg-primary hover:text-tx-on-primary"
          @click="close(app)"
        >
          {{ t('views.connect.close') }}
        </button>
      </li>
    </ul>

    <!-- La cámara del teléfono como webcam del sistema.
         Va acá y no en Ajustes porque tiene que prenderse *antes* de abrir la
         videollamada, y ésta es la única superficie que aparece exactamente
         cuando hay un teléfono enchufado. La elección de cámara, resolución y
         cuadros por segundo va en Ajustes, que es donde entran tres selectores. -->
    <div
      v-if="mostrarWebcam"
      class="flex flex-col gap-1 border-t border-ui-border/60 pt-2"
    >
      <div class="flex items-center gap-2">
        <span class="min-w-0 flex-1 text-sm text-tx-main">{{ t('views.connect.webcam') }}</span>
        <SwitchToggle
          :label="t('views.connect.webcam')"
          :is-on="webcamEncendida"
          :disabled="!interruptorHabilitadoAhora"
          @toggle="alternarWebcam"
        />
      </div>
      <!-- El error de una acción primero, y el de la lectura del estado
           después: los dos son texto del demonio y ninguno se puede reemplazar
           por el consejo del módulo, que sería un diagnóstico inventado. -->
      <p v-if="errorWebcam || errorEstadoWebcam" class="text-status-error text-xs">
        {{ errorWebcam || errorEstadoWebcam }}
      </p>
      <p v-else-if="detalleWebcam" class="text-tx-muted text-xs">{{ detalleWebcam }}</p>
    </div>
  </div>
</template>
