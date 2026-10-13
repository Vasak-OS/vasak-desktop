/**
 * Los dobles de lo que importan las píldoras del panel desde `@/…`
 * (vasak-desktop#151): los espacios de trabajo, la distribución de teclado,
 * la red y el volumen. Lo que habla con Tauri o con otras ventanas es un doble
 * que la prueba controla desde `panel`.
 */
import { computed, defineComponent, h, ref } from 'vue';
import type { KeyboardLayout, WorkspaceState } from '../../src/services/compositor.service';
import type { NetworkInfo } from '../../src/services/network.service';
import { DEFAULT_PINNED, type PinnedApp } from '../../src/tools/pinned-apps';

export { calculateVolumePercentage, getVolumeIconName } from '../../src/utils/volume';
// Las funciones puras de la píldora de ventanas y de los anclados son las de
// verdad: lo que la prueba cambia es la configuración que las alimenta.
export { windowsPillCapStyle, windowsPillClasses } from '../../src/tools/windows-pill';
export { pinnedLabel } from '../../src/tools/pinned-apps';

type Handler = (payload: unknown) => void;

export function blankNetwork(): NetworkInfo {
	return {
		name: 'Fibernet-IA',
		ssid: 'Fibernet-IA-5G-Departamento',
		connection_type: 'wireless',
		icon: 'network-wireless-signal-good-symbolic',
		ip_address: '192.168.0.10',
		mac_address: '00:00:00:00:00:00',
		signal_strength: 80,
		security_type: 'wpa2-psk',
		is_connected: true,
	};
}

/** Lo que la prueba pone y lo que mira. */
export const panel = {
	workspaces: ref<WorkspaceState | null>({ count: 6, active: 0, columns: 3, output_id: 1 }),
	layout: ref<KeyboardLayout | null>({ short: 'LA', name: 'Spanish (Latin American)', index: 0, count: 1 }),
	network: ref<NetworkInfo>(blankNetwork()),
	volume: ref({ current: 50, min: 0, max: 100, is_muted: false }),
	vertical: ref(false),
	/** La densidad elegida del panel: distribuida o compacta (`panel-appearance.ts`). */
	panelLayout: ref<'distributed' | 'compact'>('distributed'),
	/** `true` cuando el tipo dibuja una superficie detrás (flotante/barra/dock/trapecio). */
	hasSurface: ref(false),
	// Los interruptores de la sección `panel` (vasak-desktop#251): todos encendidos
	// salvo que la prueba apague uno. Es como los lee el escritorio: ausente = sí.
	showWorkspaces: ref(true),
	showWindows: ref(true),
	showNetworkName: ref(true),
	showVolumeNumber: ref(true),
	showBatteryNumber: ref(true),
	/** Las aplicaciones ancladas: por defecto las dos de siempre. */
	pinnedApps: ref<readonly PinnedApp[]>(DEFAULT_PINNED),
	failSwitch: false,
	handlers: new Map<string, Handler>(),
	calls: [] as Array<{ name: string; args: unknown[] }>,
	emit(name: string, payload: unknown) {
		this.handlers.get(name)?.(payload);
	},
	reset() {
		this.workspaces.value = { count: 6, active: 0, columns: 3, output_id: 1 };
		this.layout.value = { short: 'LA', name: 'Spanish (Latin American)', index: 0, count: 1 };
		this.network.value = blankNetwork();
		this.volume.value = { current: 50, min: 0, max: 100, is_muted: false };
		this.vertical.value = false;
		this.panelLayout.value = 'distributed';
		this.hasSurface.value = false;
		this.showWorkspaces.value = true;
		this.showWindows.value = true;
		this.showNetworkName.value = true;
		this.showVolumeNumber.value = true;
		this.showBatteryNumber.value = true;
		this.pinnedApps.value = DEFAULT_PINNED;
		battery.value = {
			has_battery: true,
			percentage: 100,
			state: 'Full',
			is_present: true,
			is_charging: false,
		};
		this.failSwitch = false;
		density.value = 'full';
		this.handlers.clear();
		this.calls.length = 0;
	},
};

const record = (name: string, ...args: unknown[]) => {
	panel.calls.push({ name, args });
};

export const getWorkspaces = async () => panel.workspaces.value;
export const switchWorkspace = async (index: number) => {
	record('switchWorkspace', index);
	if (panel.failSwitch) throw new Error('Wayfire no contesta');
};
export const getKeyboardLayout = async () => panel.layout.value;
export const nextKeyboardLayout = async () => {
	record('nextKeyboardLayout');
	const current = panel.layout.value;
	if (!current) return null;
	return { ...current, index: (current.index + 1) % current.count, short: 'US', name: 'English (US)' };
};

export const getCurrentNetworkState = async () => panel.network.value;
export const getVpnStatus = async () => ({ state: 'disconnected' });
export const getAudioVolume = async () => panel.volume.value;

/** La batería del equipo: por defecto hay una al 100 %, para ver el número. */
export const battery = ref({
	has_battery: true,
	percentage: 100,
	state: 'Full',
	is_present: true,
	is_charging: false,
});
export const getBatteryInfo = async () => battery.value;

/** El lanzamiento de una aplicación anclada: lo registra para mirarlo. */
export const openApp = async (args: unknown) => {
	record('openApp', args);
};

export const toggleApplet = async (applet: string, button?: unknown) => {
	record('toggleApplet', applet, button);
};

export function useOpenApplet(_applet: string) {
	return { isOpen: computed(() => false), openClasses: computed(() => ({})) };
}

export { showsNames, showsNumbers } from '../../src/tools/panel-density';
export { clockParts } from '../../src/tools/panel-clock';

/** La densidad del panel: llena salvo que la prueba diga otra cosa. */
export const density = ref<'full' | 'compact' | 'tight'>('full');
export function usePanelDensity() {
	return density;
}

export function usePanelConfig() {
	return {
		vertical: panel.vertical,
		layout: panel.panelLayout,
		hasSurface: panel.hasSurface,
		showWorkspaces: panel.showWorkspaces,
		showWindows: panel.showWindows,
		showNetworkName: panel.showNetworkName,
		showVolumeNumber: panel.showVolumeNumber,
		showBatteryNumber: panel.showBatteryNumber,
		pinnedApps: panel.pinnedApps,
	};
}

export function useSharedEvent(name: string, handler: Handler) {
	panel.handlers.set(name, handler);
}

export const logError = () => {};

export const getWindows = async () => [
	{ id: '1', title: 'Firefox', is_minimized: false, icon: 'firefox', app_id: 'firefox' },
	{ id: '2', title: 'Terminal', is_minimized: false, icon: 'utilities-terminal', app_id: 'vasak-terminal' },
];
export const getLauncherEntries = async () => [];
export const launcherForApp = () => undefined;

/** El botón de cada ventana: una caja con su título, para contarlas. */
export default defineComponent({
	name: 'WindowButtonDouble',
	props: { title: { type: String, default: '' } },
	setup(props) {
		return () => h('button', { 'data-window': props.title }, props.title);
	},
});
