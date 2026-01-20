import { readTextFile, readDir } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import type {
	UserDirectory,
	FileEntry,
	FileIconMapping,
} from '@/interfaces/file';

const iconMappings: FileIconMapping[] = [
	// Images
	{
		ext: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'],
		icon: 'image-x-generic',
		emoji: '🖼️',
	},
	// Videos
	{
		ext: ['mp4', 'webm', 'mkv', 'avi', 'mov'],
		icon: 'video-x-generic',
		emoji: '🎬',
	},
	// Audio
	{ ext: ['mp3', 'wav', 'flac', 'ogg'], icon: 'audio-x-generic', emoji: '🎵' },
	// Documents
	{ ext: ['txt', 'md', 'log'], icon: 'text-x-generic', emoji: '📄' },
	{ ext: ['pdf'], icon: 'application-pdf', emoji: '📕' },
	// Archives
	{
		ext: ['zip', 'tar', 'gz', '7z', 'rar'],
		icon: 'package-x-generic',
		emoji: '🗜️',
	},
	// Code
	{
		ext: ['rs', 'ts', 'js', 'vue', 'py', 'c', 'cpp'],
		icon: 'text-x-script',
		emoji: '💻',
	},
	{ ext: ['html', 'css'], icon: 'text-html', emoji: '💻' },
	// Executables
	{ ext: ['exe', 'sh', 'bin'], icon: 'application-x-executable', emoji: '⚙️' },
];

export function getIconNameForFile(filename: string, isDir: boolean): string {
	if (isDir) return 'folder';

	const parts = filename.split('.');
	const ext = parts.length > 1 ? parts.pop()!.toLowerCase() : '';

	const mapping = iconMappings.find((m) => m.ext.includes(ext));
	return mapping?.icon || 'text-x-generic';
}

export function getFileEmoji(filename: string, isDir: boolean): string {
	if (isDir) return '📁';

	const parts = filename.split('.');
	const ext = parts.length > 1 ? parts.pop()!.toLowerCase() : '';

	const mapping = iconMappings.find((m) => m.ext.includes(ext));
	return mapping?.emoji || '📄';
}

export async function getFileIconSource(
	filename: string,
	isDir: boolean,
): Promise<string | undefined> {
	const iconName = getIconNameForFile(filename, isDir);

	try {
		const source = await getIconSource(iconName);
		if (source && source.startsWith('/')) {
			return convertFileSrc(source);
		}
		return source;
	} catch (e) {
		console.warn(`Failed to load icon ${iconName}`, e);
		return undefined;
	}
}

export function isImageFile(filename: string): boolean {
	return /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(filename);
}

export function isVideoFile(filename: string): boolean {
	return /\.(mp4|webm|mkv|avi|mov)$/i.test(filename);
}

export function getFileMimeType(
	filename: string,
): 'image' | 'video' | 'audio' | 'text' | 'application' | 'unknown' {
	const parts = filename.split('.');
	const ext = parts.length > 1 ? parts.pop()!.toLowerCase() : '';

	if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext))
		return 'image';
	if (['mp4', 'webm', 'mkv', 'avi', 'mov'].includes(ext)) return 'video';
	if (['mp3', 'wav', 'flac', 'ogg'].includes(ext)) return 'audio';
	if (['txt', 'md', 'log'].includes(ext)) return 'text';
	if (['pdf', 'zip', 'tar', 'gz', '7z', 'rar'].includes(ext))
		return 'application';

	return 'unknown';
}

export async function loadDirectory(
	dirPath: string,
	showHidden: boolean = false,
): Promise<FileEntry[]> {
	try {
		const entries = await readDir(dirPath);

		const processedFiles = await Promise.all(
			entries
				.filter((entry) => showHidden || !entry.name.startsWith('.'))
				.map(async (entry) => {
					const filePath = `${dirPath}/${entry.name}`;
					const fileEntry: FileEntry = {
						name: entry.name,
						path: filePath,
						isDirectory: entry.isDirectory,
						isHidden: entry.name.startsWith('.'),
					};

					// Load icon
					try {
						fileEntry.icon = await getFileIconSource(
							entry.name,
							entry.isDirectory,
						);
					} catch (e) {
						console.warn(`Failed to load icon for ${entry.name}`, e);
					}

					// Generate preview for images and videos
					if (!entry.isDirectory) {
						if (isImageFile(entry.name)) {
							fileEntry.previewUrl = convertFileSrc(filePath);
							fileEntry.mimeType = 'image';
						} else if (isVideoFile(entry.name)) {
							fileEntry.previewUrl = convertFileSrc(filePath);
							fileEntry.mimeType = 'video';
						}
					}

					return fileEntry;
				}),
		);

		return processedFiles;
	} catch (err: any) {
		console.error('Error reading directory:', err);
		throw err;
	}
}

export async function loadDirectoryBackend(
	dirPath: string,
	showHidden: boolean = false,
): Promise<FileEntry[]> {
	try {
    interface BackendFileEntry {
      name: string;
      is_dir: boolean;
      size: string;
      path: string;
    }

    const entries = await invoke<BackendFileEntry[]>('read_directory', {
    	path: dirPath,
    	showHidden,
    });

    const processedFiles = await Promise.all(
    	entries.map(async (entry) => {
    		const fileEntry: FileEntry = {
    			name: entry.name,
    			path: entry.path,
    			isDirectory: entry.is_dir,
    			isHidden: entry.name.startsWith('.'),
    			size: entry.size,
    		};

    		// Load icon
    		try {
    			fileEntry.icon = await getFileIconSource(entry.name, entry.is_dir);
    		} catch (e) {
    			console.warn(`Failed to load icon for ${entry.name}`, e);
    		}

    		// Generate preview for images and videos
    		if (!entry.is_dir) {
    			if (isImageFile(entry.name)) {
    				fileEntry.previewUrl = convertFileSrc(entry.path);
    				fileEntry.mimeType = 'image';
    			} else if (isVideoFile(entry.name)) {
    				fileEntry.previewUrl = convertFileSrc(entry.path);
    				fileEntry.mimeType = 'video';
    			}
    		}

    		return fileEntry;
    	}),
    );

    return processedFiles;
	} catch (err: any) {
		console.error('Error reading directory:', err);
		throw err;
	}
}

/**
 * Reads and parses the XDG user-dirs.dirs configuration file
 * to get the user's special directories (Documents, Downloads, etc.)
 * @param homePath - The user's home directory path
 * @returns Array of user directories with name, icon, and path
 */
export async function getUserDirectories(
	homePath: string,
): Promise<UserDirectory[]> {
	const directories: UserDirectory[] = [];

	try {
		const configPath = await join(homePath, '.config', 'user-dirs.dirs');
		const content = await readTextFile(configPath);

		// Map XDG var to icon
		const iconMap: Record<string, string> = {
			XDG_DOCUMENTS_DIR: 'folder-documents',
			XDG_DOWNLOAD_DIR: 'folder-download',
			XDG_MUSIC_DIR: 'folder-music',
			XDG_PICTURES_DIR: 'folder-pictures',
			XDG_VIDEOS_DIR: 'folder-videos',
			XDG_DESKTOP_DIR: 'user-desktop',
		};

		const lines = content.split('\n');
		for (const line of lines) {
			const trimmed = line.trim();
			if (!trimmed || trimmed.startsWith('#')) continue;

			const match = trimmed.match(/^(XDG_[A-Z_]+_DIR)="(.*)"/);
			if (match) {
				const key = match[1];
				const val = match[2];

				const resolvedPath = val.replace('$HOME', homePath);
				const name = resolvedPath.split('/').pop() || val;
				const icon = iconMap[key] || 'folder';

				directories.push({ name, icon, path: resolvedPath, xdgKey: key });
			}
		}
	} catch (e) {
		console.warn('Failed to load user-dirs.dirs, falling back to defaults', e);
		// Fallback to default directories
		directories.push(
			{ name: 'Documents', icon: 'folder-documents', path: `${homePath}/Documents` },
			{ name: 'Downloads', icon: 'folder-download', path: `${homePath}/Downloads` },
			{ name: 'Pictures', icon: 'folder-pictures', path: `${homePath}/Pictures` },
			{ name: 'Music', icon: 'folder-music', path: `${homePath}/Music` },
		);
	}

	return directories;
}
