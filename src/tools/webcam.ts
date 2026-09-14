/**
 * Las reglas del interruptor de la webcam del teléfono.
 *
 * El interruptor del centro de control no pregunta nada: se prende y transmite.
 * La elección fina —cámara, resolución, cuadros por segundo— va en Ajustes,
 * donde hay lugar para tres selectores; acá hace falta un valor por defecto que
 * sea el correcto casi siempre, porque el camino de uso es «enchufo el
 * teléfono, prendo, abro la videollamada».
 *
 * Vive aparte del componente porque decide cuándo se puede **apagar** una
 * cámara que está transmitiendo, y equivocarse ahí deja una cámara encendida
 * sin forma de cortarla. Eso no se prueba mirando una tarjeta.
 */

import type { ConnectCamera, ConnectWebcamState } from '@/interfaces/connect';

/**
 * La cámara que conviene usar de las que tiene el teléfono.
 *
 * **La trasera primero.** Un teléfono usado como webcam se apoya contra el
 * monitor con la pantalla hacia afuera, así que la que apunta a la persona es
 * la de atrás — y encima es el mejor sensor de los dos. La frontal sirve cuando
 * el teléfono se sostiene en la mano, que no es este caso.
 *
 * Si no hay ninguna trasera se usa la primera que haya, en lugar de no ofrecer
 * nada: un teléfono con una sola cámara clasificada como `external` igual puede
 * transmitir, y negarse sería inventar un requisito.
 */
export function camaraPorDefecto(camaras: readonly ConnectCamera[]): ConnectCamera | undefined {
	return camaras.find((camara) => camara.facing === 'back') ?? camaras[0];
}

/**
 * Lo más grande que conviene pedirle a la cámara del teléfono.
 *
 * Esto es una webcam: se mira dentro de una videollamada, que reescala a 720p
 * de todas formas. Cada píxel de más es latencia y batería del teléfono, y no
 * se ve.
 */
const TOPE_DE_ANCHO = 1920;
const TOPE_DE_ALTO = 1080;

/** `1280x720` → `[1280, 720]`, o `undefined` si no tiene esa forma. */
function medidas(tamanio: string): [number, number] | undefined {
	const [ancho, alto] = tamanio.split('x');
	const w = Number(ancho);
	const h = Number(alto);
	if (!Number.isInteger(w) || !Number.isInteger(h) || w <= 0 || h <= 0) return undefined;
	return [w, h];
}

/**
 * El tamaño con el que arrancar, de los que el teléfono enumeró.
 *
 * **Hay que pedir uno.** Sin tamaño, el teléfono elige el máximo de su sensor, y
 * ése no pasa por su propio codificador de vídeo: en un motorola edge 40 la
 * trasera da `4096x3072` y scrcpy muere con `IllegalArgumentException` en
 * `MediaCodec.configure`. El teléfono elige por la cámara, no por el
 * codificador, y nadie comprueba que lo elegido se pueda codificar — así que el
 * interruptor no prendía en ningún teléfono cuyo modo máximo supere lo que su
 * codificador acepta, que hoy es casi cualquiera.
 *
 * **De la lista enumerada y no de una tabla de resoluciones comunes.** Un tamaño
 * que el sensor no tiene falla igual de feo que uno que el codificador no
 * acepta, y la lista es justo lo que el teléfono contestó cuando se le preguntó.
 *
 * Si todos los modos pasan el tope se devuelve el más chico: es lo único que
 * queda por intentar, y es mejor que rendirse antes de probar.
 */
export function tamanioPorDefecto(camara: ConnectCamera | undefined): string {
	const medidos = (camara?.sizes ?? [])
		.map((tamanio) => ({ tamanio, medidas: medidas(tamanio) }))
		.flatMap(({ tamanio, medidas }) => (medidas ? [{ tamanio, pixeles: medidas }] : []));

	if (medidos.length === 0) return '';

	const entran = medidos.filter(
		({ pixeles: [ancho, alto] }) => ancho <= TOPE_DE_ANCHO && alto <= TOPE_DE_ALTO
	);

	const area = ({ pixeles: [ancho, alto] }: (typeof medidos)[number]) => ancho * alto;

	if (entran.length > 0) {
		return entran.reduce((mayor, uno) => (area(uno) > area(mayor) ? uno : mayor)).tamanio;
	}

	return medidos.reduce((menor, uno) => (area(uno) < area(menor) ? uno : menor)).tamanio;
}

/**
 * Si la webcam la está alimentando **este** teléfono.
 *
 * El serial importa: con dos teléfonos enchufados, el que transmite es uno solo
 * —el dispositivo de vídeo admite un productor— y la tarjeta del otro no puede
 * mostrar su interruptor encendido.
 */
export function encendidaEn(estado: ConnectWebcamState | null, serial?: string): boolean {
	return estado?.active === true && !!serial && estado.serial === serial;
}

/**
 * En qué situación está la cámara, para poder decirlo con una sola frase.
 *
 * `desconocido` existe y no es un detalle: mientras la respuesta del demonio
 * viene en camino —o si la lectura falló— **no se sabe nada**, y hay que
 * callarse. Sin este valor, un estado ausente se lee como uno vacío, un
 * dispositivo vacío significa «falta el módulo v4l2loopback», y la tarjeta
 * termina mandando a reiniciar por una consulta que simplemente no volvió.
 */
export type DiagnosticoWebcam = 'desconocido' | 'sin-modulo' | 'ocupada' | 'encendida' | 'lista';

/**
 * Qué le pasa a la cámara desde el punto de vista de este teléfono.
 *
 * El orden importa: sin módulo no hay nada más que decir —ni siquiera se puede
 * estar transmitiendo—, y «encendida» se decide antes que «ocupada» porque
 * ocupada significa «la tiene otro».
 */
export function diagnosticoWebcam(
	estado: ConnectWebcamState | null,
	serial?: string
): DiagnosticoWebcam {
	if (estado === null) return 'desconocido';
	if (estado.device === '') return 'sin-modulo';
	if (encendidaEn(estado, serial)) return 'encendida';
	if (estado.active) return 'ocupada';
	return 'lista';
}

/** Lo que hace falta saber para decidir si el interruptor se puede tocar. */
export interface SituacionWebcam {
	estado: ConnectWebcamState | null;
	/** El teléfono de esta tarjeta. */
	serial?: string;
	/** Si el teléfono terminó de autorizarse y está listo para trabajar. */
	telefonoListo: boolean;
	/** Si hay un encendido o apagado a mitad de camino. */
	enCurso: boolean;
}

/**
 * Si el interruptor se puede tocar.
 *
 * **Apagar puede casi siempre.** Si esta cámara está transmitiendo, el
 * interruptor responde aunque el teléfono haya dejado de estar «listo»: dejar
 * una cámara encendida sin forma de cortarla es lo peor que puede hacer esta
 * tarjeta, y es la clase de estado en el que se cae solo —el teléfono se
 * bloquea, el cable se mueve— justo cuando alguien quiere apagarla.
 *
 * Prender pide las tres condiciones: teléfono listo, módulo del kernel cargado
 * —sin él no hay dónde escribir— y que no haya otra cámara transmitiendo, que
 * el demonio rechazaría con `WebcamBusy`.
 *
 * Lo único que bloquea las dos direcciones es una operación en curso.
 */
export function interruptorHabilitado(situacion: SituacionWebcam): boolean {
	if (situacion.enCurso) return false;
	if (encendidaEn(situacion.estado, situacion.serial)) return true;

	return (
		situacion.telefonoListo &&
		(situacion.estado?.device ?? '') !== '' &&
		situacion.estado?.active !== true
	);
}
