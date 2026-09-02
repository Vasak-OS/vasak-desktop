import { describe, expect, test } from 'bun:test';
import type { ConnectCamera, ConnectCameraFacing, ConnectWebcamState } from '@/interfaces/connect';
import { camaraPorDefecto, diagnosticoWebcam, encendidaEn, interruptorHabilitado } from './webcam';

const camara = (id: string, facing: ConnectCameraFacing): ConnectCamera => ({
	id,
	facing,
	sizes: ['1280x720'],
	fps: [30],
});

const apagada = (device = '/dev/video42'): ConnectWebcamState => ({
	active: false,
	device,
	serial: '',
	camera_id: '',
	size: '',
});

const transmitiendo = (serial: string): ConnectWebcamState => ({
	active: true,
	device: '/dev/video42',
	serial,
	camera_id: '0',
	size: '1280x720',
});

describe('la cámara por defecto', () => {
	test('es la trasera cuando el teléfono tiene las dos', () => {
		// Es la que apunta a la persona con el teléfono apoyado contra el
		// monitor, y el mejor sensor de los dos.
		expect(camaraPorDefecto([camara('1', 'front'), camara('0', 'back')])?.id).toBe('0');
	});

	test('no depende del orden en que las liste el teléfono', () => {
		expect(camaraPorDefecto([camara('0', 'back'), camara('1', 'front')])?.id).toBe('0');
	});

	test('sin trasera se usa la primera que haya', () => {
		// Negarse dejaría sin webcam a un teléfono que puede transmitir.
		expect(camaraPorDefecto([camara('9', 'external'), camara('1', 'front')])?.id).toBe('9');
	});

	test('sin cámaras no hay ninguna', () => {
		// El interruptor tiene que quedar deshabilitado, no llamar al demonio
		// con un id vacío: `StartWebcam` con una cámara que no existe abre el
		// stream y lo mata medio segundo después.
		expect(camaraPorDefecto([])).toBeUndefined();
	});
});

describe('de quién es la cámara encendida', () => {
	test('del teléfono cuyo serial coincide', () => {
		expect(encendidaEn(transmitiendo('ABC123'), 'ABC123')).toBe(true);
	});

	test('con dos teléfonos, el otro no la muestra como encendida', () => {
		// El dispositivo de vídeo admite un productor. La tarjeta del teléfono
		// que no está transmitiendo no puede decir que sí.
		expect(encendidaEn(transmitiendo('ABC123'), 'XYZ789')).toBe(false);
	});

	test('sin nada transmitiendo, no', () => {
		expect(encendidaEn(apagada(), 'ABC123')).toBe(false);
	});

	test('sin estado todavía leído, no', () => {
		expect(encendidaEn(null, 'ABC123')).toBe(false);
	});

	test('sin teléfono, no', () => {
		// Si no hay serial no hay a quién atribuirla, y `estado.serial === undefined`
		// sería `true` para un estado con serial vacío.
		expect(encendidaEn(apagada(), undefined)).toBe(false);
	});
});

describe('cuándo se puede tocar el interruptor', () => {
	test('prender, con el teléfono listo y el módulo cargado', () => {
		expect(
			interruptorHabilitado({
				estado: apagada(),
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: false,
			})
		).toBe(true);
	});

	test('apagar se puede aunque el teléfono ya no esté listo', () => {
		// El caso importante: el teléfono se bloquea o el cable se mueve
		// mientras la cámara transmite. Si esto devolviera `false`, quedaría una
		// cámara encendida y el interruptor gris.
		expect(
			interruptorHabilitado({
				estado: transmitiendo('ABC123'),
				serial: 'ABC123',
				telefonoListo: false,
				enCurso: false,
			})
		).toBe(true);
	});

	test('no se puede prender sin el módulo del kernel', () => {
		// Sin v4l2loopback no hay dónde escribir la cámara. La ruta vacía es
		// cómo lo informa el demonio, y llega incluso con `active` en falso
		// justamente para poder decirlo antes de que alguien apriete.
		expect(
			interruptorHabilitado({
				estado: apagada(''),
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: false,
			})
		).toBe(false);
	});

	test('no se puede prender si la está usando otro teléfono', () => {
		// El demonio contestaría `WebcamBusy`; mejor no ofrecerlo.
		expect(
			interruptorHabilitado({
				estado: transmitiendo('XYZ789'),
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: false,
			})
		).toBe(false);
	});

	test('no se puede prender con el teléfono a medio autorizar', () => {
		expect(
			interruptorHabilitado({
				estado: apagada(),
				serial: 'ABC123',
				telefonoListo: false,
				enCurso: false,
			})
		).toBe(false);
	});

	test('una operación en curso bloquea las dos direcciones', () => {
		// Sin esto, dos clics seguidos mandan dos llamadas y la segunda decide
		// el estado final, que puede ser el contrario del último clic.
		expect(
			interruptorHabilitado({
				estado: transmitiendo('ABC123'),
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: true,
			})
		).toBe(false);
		expect(
			interruptorHabilitado({
				estado: apagada(),
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: true,
			})
		).toBe(false);
	});

	test('sin estado leído todavía no se ofrece nada', () => {
		expect(
			interruptorHabilitado({
				estado: null,
				serial: 'ABC123',
				telefonoListo: true,
				enCurso: false,
			})
		).toBe(false);
	});
});

describe('el diagnóstico de la cámara', () => {
	test('sin estado leído todavía, no se sabe nada', () => {
		// La regla que importa: un estado ausente **no** es un estado vacío. Sin
		// esto, la tarjeta anunciaba que falta el módulo del kernel mientras la
		// respuesta venía en camino, o cuando la consulta al demonio falló, y el
		// arreglo que ofrecía era reiniciar el equipo.
		expect(diagnosticoWebcam(null, 'ABC123')).toBe('desconocido');
	});

	test('sin dispositivo, falta el módulo del kernel', () => {
		expect(diagnosticoWebcam(apagada(''), 'ABC123')).toBe('sin-modulo');
	});

	test('lista cuando hay módulo y nada transmitiendo', () => {
		expect(diagnosticoWebcam(apagada(), 'ABC123')).toBe('lista');
	});

	test('encendida cuando la alimenta este teléfono', () => {
		expect(diagnosticoWebcam(transmitiendo('ABC123'), 'ABC123')).toBe('encendida');
	});

	test('ocupada cuando la alimenta otro', () => {
		expect(diagnosticoWebcam(transmitiendo('XYZ789'), 'ABC123')).toBe('ocupada');
	});
});
