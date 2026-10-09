import { afterEach, describe, expect, test } from 'bun:test';
import {
	applyAppletChanged,
	OPEN_APPLET_CLASSES,
	openAppletForTests,
	openAppletState,
} from './useOpenApplet';

afterEach(() => applyAppletChanged({ applet: null }));

describe('el botón cuyo applet está abierto', () => {
	test('se realza sólo el suyo', () => {
		const audio = openAppletState('audio');
		const network = openAppletState('network');

		applyAppletChanged({ applet: 'audio' });

		expect(audio.isOpen.value).toBe(true);
		expect(audio.openClasses.value).toEqual({ [OPEN_APPLET_CLASSES]: true });
		expect(network.isOpen.value).toBe(false);
		expect(network.openClasses.value).toEqual({ [OPEN_APPLET_CLASSES]: false });
	});

	test('el realce sigue al backend: abrir otro lo pasa, cerrar lo apaga', () => {
		// Un applet también se cierra con Escape o al perder el foco, y el panel
		// no ve ninguna de las dos cosas: el realce tiene que salir del aviso y
		// no de lo que el botón crea haber abierto.
		const audio = openAppletState('audio');
		const bluetooth = openAppletState('bluetooth');

		applyAppletChanged({ applet: 'audio' });
		applyAppletChanged({ applet: 'bluetooth' });
		expect(audio.isOpen.value).toBe(false);
		expect(bluetooth.isOpen.value).toBe(true);

		applyAppletChanged({ applet: null });
		expect(bluetooth.isOpen.value).toBe(false);
	});

	test('un aviso sin applet se lee como «nada abierto»', () => {
		applyAppletChanged({ applet: 'tray' });
		applyAppletChanged(null);
		expect(openAppletForTests.value).toBeNull();
	});

	test('las clases son las del diseño: el velo del acento de lo elegido', () => {
		expect(OPEN_APPLET_CLASSES.split(' ').sort()).toEqual(['bg-ui-selected-accent']);
	});
});
