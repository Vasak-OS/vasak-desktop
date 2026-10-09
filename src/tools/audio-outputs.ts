/**
 * Las salidas de audio, para el chip del reproductor desplegable.
 *
 * Aparte del componente para poder probarlas: este repositorio no monta
 * componentes en las pruebas.
 */
import type { AudioDevice } from '@/interfaces/audio-device';

/**
 * La salida que está sonando.
 *
 * La que PipeWire tiene por omisión; si ninguna lo es —pasa un instante al
 * desconectar unos auriculares—, la primera, que es a la que va a caer el
 * audio. Sin ninguna, nada: el chip no se dibuja.
 */
export function currentOutput(devices: readonly AudioDevice[]): AudioDevice | null {
	return devices.find((device) => device.is_default) ?? devices[0] ?? null;
}

/**
 * El nombre para mostrar, sin la capa de audio que lo publica.
 *
 * «Audio interno Estéreo analógico PipeWire» dice lo mismo sin la última
 * palabra, y en un chip de ciento cincuenta píxeles la palabra que sobra es la
 * que empuja al nombre de verdad fuera de la vista. Es el mismo criterio que el
 * selector del applet de audio.
 */
export function outputLabel(device: AudioDevice): string {
	const cleaned = device.name
		.replaceAll('ALSA', '')
		.replaceAll('PulseAudio', '')
		.replaceAll('PipeWire', '')
		.replace(/\s+/g, ' ')
		.trim();
	return cleaned || device.description || device.id;
}

/**
 * Auriculares o parlante, por el nombre.
 *
 * PipeWire no dice qué forma tiene el aparato, pero un sumidero de Bluetooth o
 * uno que se llama «headphones» o «headset» casi siempre es algo que va en la
 * cabeza, y el icono es lo que se mira antes que el nombre.
 */
export function outputIcon(device: AudioDevice | null): string {
	if (!device) return 'audio-speakers';
	const haystack = `${device.name} ${device.description}`.toLowerCase();
	return /bluez|headphone|headset|auricular/.test(haystack) ? 'audio-headphones' : 'audio-speakers';
}
