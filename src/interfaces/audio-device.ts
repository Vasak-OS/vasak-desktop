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
	/**
	 * La `card` dueña de la salida (vasak-desktop#230). En un sink vivo sale de
	 * `device.name`; en una salida disponible-pero-inactiva es la placa cuyo
	 * perfil hay que conmutar al elegirla.
	 */
	card?: string | null;
	/**
	 * El perfil de salida a conmutar (p. ej. `output:hdmi-stereo`). Sólo lo
	 * llevan las salidas inactivas; un sink vivo lo deja sin valor.
	 */
	profile?: string | null;
	/**
	 * `true` cuando es un sink que ya existe; `false` cuando es un perfil de
	 * `card` disponible pero sin sink todavía (HDMI con la placa en otro
	 * perfil). Ausente desde comandos viejos equivale a `true`.
	 */
	active?: boolean;
}
