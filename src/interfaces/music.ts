export interface MusicInfo {
	/** El nombre de bus, que es con lo que se le habla al reproductor. */
	player: string;
	/** Cómo se llama para mostrar: «Spotify», «Chromium». */
	playerIdentity: string;
	status: string;
	title: string;
	artist: string;
	album: string;
	artUrl: string;
	/** Duración y posición en **microsegundos**, como las manda MPRIS. */
	length: number;
	position: number;
	trackId: string;
	/**
	 * Lo que el reproductor dice que se puede hacer con él.
	 *
	 * No es adorno: con un vídeo de YouTube, Chromium contesta que no se puede
	 * ir al anterior ni al siguiente, y los dos botones estaban dibujados igual.
	 */
	canGoNext: boolean;
	canGoPrevious: boolean;
	canPlay: boolean;
	canPause: boolean;
	canSeek: boolean;
	canControl: boolean;
	canRaise: boolean;
	/**
	 * Los tres que pueden **no existir**. `null` es «este reproductor no lo
	 * implementa» y el control no se dibuja; `false` es «lo tiene y está
	 * apagado», que es otra cosa.
	 */
	shuffle: boolean | null;
	loopStatus: string | null;
	volume: number | null;
}

/** Un reproductor de la lista con la que se elige a cuál seguir. */
export interface PlayerRef {
	player: string;
	identity: string;
	status: string;
	title: string;
	/** El que se está mostrando. */
	active: boolean;
	/** El que eligió el usuario a mano, si eligió alguno. */
	pinned: boolean;
}

export type MusicStatus = 'playing' | 'paused' | 'stopped';

/** Las tres formas de repetición de MPRIS, en el orden en que rota el botón. */
export const LOOP_STATUSES = ['None', 'Playlist', 'Track'] as const;
export type LoopStatus = (typeof LOOP_STATUSES)[number];
