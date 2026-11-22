/**
 * Los dobles de lo que `PanelView.vue` importa desde `@/…`, para probar sólo la
 * píldora del asistente: todo componente es mudo y los composables, de mentira.
 */
import { defineComponent, ref } from 'vue';

export { GROUP_CLASSES } from '../../src/tools/panel-position';

export const silent = (name: string) => defineComponent({ name, setup: () => () => null });

export const panelAssistant = {
	available: (): Promise<boolean> => Promise.resolve(true),
	calls: [] as string[],
	reset() {
		this.available = () => Promise.resolve(true);
		this.calls = [];
	},
};

export const assistantAvailable = () => panelAssistant.available();
export const toggleAssistant = async () => {
	panelAssistant.calls.push('toggleAssistant');
};
export const listConnectDevices = async () => [];
export const toggleConnectMenu = async () => {};
export const getAllNotifications = async () => [];
export const reportMenuButton = async () => {};
export const toggleControlCenter = async () => {};
export const toggleMenu = async () => {};
export const useOpenApplet = () => ({ isOpen: ref(false) });
export const usePanelAutohide = () => ({ hidden: ref(false), reveal() {}, requestHide() {} });
export const usePanelConfig = () => ({
	position: ref('top'),
	vertical: ref(false),
	style: ref('pills'),
	showWeather: ref(false),
	showMusic: ref(false),
	showKeyboard: ref(false),
	showWindows: ref(false),
	showWorkspaces: ref(false),
	barClasses: ref(''),
	surfaceClass: ref(''),
	hasSurface: ref(false),
	animationClass: ref(''),
	autohide: ref(false),
	menuIcon: ref('start-here'),
});
export const usePanelDensity = () => ref('regular');
export const watchPanelDensity = () => {};
export const usePanelInputRegion = () => {};
export const usePanelTrapezoidClip = () => {};
export const useSharedEvent = () => {};
export const containsNewNotifications = () => false;
export const panelHideClass = () => '';
export const panelRegionMode = () => 'pills';
export const logError = () => {};
