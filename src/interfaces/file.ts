export interface FileEntry {
	name: string;
	path: string;
	isDirectory: boolean;
	isHidden: boolean;
	size?: string;
	icon?: string;
	previewUrl?: string;
	mimeType?: string;
	loadError?: boolean;
}

export interface FileIconMapping {
	ext: string[];
	icon: string;
}

export interface UserDirectory {
	name: string;
	icon: string;
	path: string;
	xdgKey?: string;
}
