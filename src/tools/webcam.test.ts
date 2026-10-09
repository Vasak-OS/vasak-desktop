import { describe, expect, test } from 'bun:test';
import type { ConnectCamera, ConnectCameraFacing, ConnectWebcamState } from '@/interfaces/connect';
import { defaultCamera, defaultSize, isActiveOn, switchEnabled, webcamDiagnosis } from './webcam';

const camera = (id: string, facing: ConnectCameraFacing): ConnectCamera => ({
	id,
	facing,
	sizes: ['1280x720'],
	fps: [30],
});

const idle = (device = '/dev/video42'): ConnectWebcamState => ({
	active: false,
	device,
	serial: '',
	camera_id: '',
	size: '',
});

const streaming = (serial: string): ConnectWebcamState => ({
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
		expect(defaultCamera([camera('1', 'front'), camera('0', 'back')])?.id).toBe('0');
	});

	test('no depende del orden en que las liste el teléfono', () => {
		expect(defaultCamera([camera('0', 'back'), camera('1', 'front')])?.id).toBe('0');
	});

	test('sin trasera se usa la primera que haya', () => {
		// Negarse dejaría sin webcam a un teléfono que puede transmitir.
		expect(defaultCamera([camera('9', 'external'), camera('1', 'front')])?.id).toBe('9');
	});

	test('sin cámaras no hay ninguna', () => {
		// El interruptor tiene que quedar deshabilitado, no llamar al demonio
		// con un id vacío: `StartWebcam` con una cámara que no existe abre el
		// stream y lo mata medio segundo después.
		expect(defaultCamera([])).toBeUndefined();
	});
});

describe('de quién es la cámara encendida', () => {
	test('del teléfono cuyo serial coincide', () => {
		expect(isActiveOn(streaming('ABC123'), 'ABC123')).toBe(true);
	});

	test('con dos teléfonos, el otro no la muestra como encendida', () => {
		// El dispositivo de vídeo admite un productor. La tarjeta del teléfono
		// que no está transmitiendo no puede decir que sí.
		expect(isActiveOn(streaming('ABC123'), 'XYZ789')).toBe(false);
	});

	test('sin nada transmitiendo, no', () => {
		expect(isActiveOn(idle(), 'ABC123')).toBe(false);
	});

	test('sin estado todavía leído, no', () => {
		expect(isActiveOn(null, 'ABC123')).toBe(false);
	});

	test('sin teléfono, no', () => {
		// Si no hay serial no hay a quién atribuirla, y `estado.serial === undefined`
		// sería `true` para un estado con serial vacío.
		expect(isActiveOn(idle(), undefined)).toBe(false);
	});
});

describe('cuándo se puede tocar el interruptor', () => {
	test('prender, con el teléfono listo y el módulo cargado', () => {
		expect(
			switchEnabled({
				state: idle(),
				serial: 'ABC123',
				phoneReady: true,
				inProgress: false,
			})
		).toBe(true);
	});

	test('apagar se puede aunque el teléfono ya no esté listo', () => {
		// El caso importante: el teléfono se bloquea o el cable se mueve
		// mientras la cámara transmite. Si esto devolviera `false`, quedaría una
		// cámara encendida y el interruptor gris.
		expect(
			switchEnabled({
				state: streaming('ABC123'),
				serial: 'ABC123',
				phoneReady: false,
				inProgress: false,
			})
		).toBe(true);
	});

	test('no se puede prender sin el módulo del kernel', () => {
		// Sin v4l2loopback no hay dónde escribir la cámara. La ruta vacía es
		// cómo lo informa el demonio, y llega incluso con `active` en falso
		// justamente para poder decirlo antes de que alguien apriete.
		expect(
			switchEnabled({
				state: idle(''),
				serial: 'ABC123',
				phoneReady: true,
				inProgress: false,
			})
		).toBe(false);
	});

	test('no se puede prender si la está usando otro teléfono', () => {
		// El demonio contestaría `WebcamBusy`; mejor no ofrecerlo.
		expect(
			switchEnabled({
				state: streaming('XYZ789'),
				serial: 'ABC123',
				phoneReady: true,
				inProgress: false,
			})
		).toBe(false);
	});

	test('no se puede prender con el teléfono a medio autorizar', () => {
		expect(
			switchEnabled({
				state: idle(),
				serial: 'ABC123',
				phoneReady: false,
				inProgress: false,
			})
		).toBe(false);
	});

	test('una operación en curso bloquea las dos direcciones', () => {
		// Sin esto, dos clics seguidos mandan dos llamadas y la segunda decide
		// el estado final, que puede ser el contrario del último clic.
		expect(
			switchEnabled({
				state: streaming('ABC123'),
				serial: 'ABC123',
				phoneReady: true,
				inProgress: true,
			})
		).toBe(false);
		expect(
			switchEnabled({
				state: idle(),
				serial: 'ABC123',
				phoneReady: true,
				inProgress: true,
			})
		).toBe(false);
	});

	test('sin estado leído todavía no se ofrece nada', () => {
		expect(
			switchEnabled({
				state: null,
				serial: 'ABC123',
				phoneReady: true,
				inProgress: false,
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
		expect(webcamDiagnosis(null, 'ABC123')).toBe('unknown');
	});

	test('sin dispositivo, falta el módulo del kernel', () => {
		expect(webcamDiagnosis(idle(''), 'ABC123')).toBe('no-module');
	});

	test('lista cuando hay módulo y nada transmitiendo', () => {
		expect(webcamDiagnosis(idle(), 'ABC123')).toBe('ready');
	});

	test('encendida cuando la alimenta este teléfono', () => {
		expect(webcamDiagnosis(streaming('ABC123'), 'ABC123')).toBe('active');
	});

	test('ocupada cuando la alimenta otro', () => {
		expect(webcamDiagnosis(streaming('XYZ789'), 'ABC123')).toBe('busy');
	});
});

describe('el tamaño por defecto', () => {
	/** Una cámara con los tamaños que contestó un motorola edge 40. */
	const withSizes = (sizes: string[]): ConnectCamera => ({
		id: '0',
		facing: 'back',
		sizes,
		fps: [30],
	});

	test('no devuelve el máximo del sensor', () => {
		// Es el fallo que esto viene a arreglar: sin pedir tamaño, el teléfono
		// elegía 4096x3072 y su propio codificador no podía configurarse con
		// eso. El interruptor no prendía nunca.
		const chosen = defaultSize(withSizes(['4096x3072', '3840x2160', '1920x1080', '1280x720']));

		expect(chosen).not.toBe('4096x3072');
		expect(chosen).toBe('1920x1080');
	});

	test('es el mayor que entra en el tope, no el primero que entra', () => {
		// La lista viene de mayor a menor, así que tomar el primero que entra
		// funcionaría por casualidad. Se la da desordenada para que no.
		expect(defaultSize(withSizes(['640x480', '1920x1080', '1280x720']))).toBe('1920x1080');
	});

	test('un modo más alto que el tope no entra aunque sea angosto', () => {
		// 1080x1920 es vertical: el ancho entra y el alto no. Mirar sólo el
		// ancho lo dejaría pasar, y son los mismos píxeles que el codificador
		// rechaza.
		expect(defaultSize(withSizes(['1080x1920', '1280x720']))).toBe('1280x720');
	});

	test('si ninguno entra se pide el más chico, que es lo único que queda', () => {
		// Rendirse antes de probar sería peor: el tope es una preferencia
		// nuestra, no un límite del teléfono.
		expect(defaultSize(withSizes(['4096x3072', '2560x1920']))).toBe('2560x1920');
	});

	test('sin cámara o sin tamaños se deja elegir al teléfono', () => {
		// Cadena vacía es lo que el demonio entiende como «elegí vos», y es
		// deliberado: sin lista enumerada no hay nada mejor que pedir. Inventar
		// una resolución que el teléfono no contestó falla igual de feo —un
		// tamaño que el sensor no tiene se cae como uno que el codificador no
		// acepta—, y negarse a arrancar convertiría un intento que quizá
		// funciona en uno que seguro no. El caso es el único en el que esto
		// queda igual que antes del arreglo, no peor.
		expect(defaultSize(undefined)).toBe('');
		expect(defaultSize(withSizes([]))).toBe('');
	});

	test('un tamaño con forma rara no se pasa como argumento', () => {
		// Los tamaños salen de parsear la salida de scrcpy, y de ahí va derecho
		// a una opción de línea de comandos.
		expect(defaultSize(withSizes(['grande', '1280x720']))).toBe('1280x720');
		expect(defaultSize(withSizes(['x720', '-1x-1']))).toBe('');
	});
});
