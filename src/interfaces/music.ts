export interface MusicInfo {
	title: string;
	artist: string;
	player: string;
	artUrl: string;
	status: string;
}

export type MusicStatus = 'playing' | 'paused' | 'stopped';
