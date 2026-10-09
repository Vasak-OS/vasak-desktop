/**
 * Una salida de audio, tal como la manda `get_audio_devices`
 * (`AudioDevice` en `src-tauri/src/structs.rs`).
 *
 * Ojo con los nombres, que vienen cruzados desde Rust: `name` es lo que se
 * muestra («Audio interno Estéreo analógico») y `description` es el nombre del
 * sumidero de PipeWire (`alsa_output.pci-0000_00_1f.3.analog-stereo`). Se
 * leen tal cual porque renombrarlos es cambiar lo que viaja entre los dos lados.
 */
export interface AudioDevice {
	id: string;
	name: string;
	description: string;
	is_default: boolean;
	volume: number;
}
