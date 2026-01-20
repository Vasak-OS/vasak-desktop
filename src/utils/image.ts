import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Processes different URL formats (file://, http://, https://, absolute paths)
 * and returns a usable source for img elements
 */
export function processImageUrl(url: string | undefined | null): string | null {
	if (!url || url.trim() === '') {
		return null;
	}

	const cleanUrl = url.trim();

	if (cleanUrl.startsWith('file://')) {
		// Remove file:// or file:/// prefix
		const path = cleanUrl.replace(/^file:\/+/, '/');
		return convertFileSrc(path);
	}

	if (cleanUrl.startsWith('http://') || cleanUrl.startsWith('https://')) {
		// Remote URL: use directly
		return cleanUrl;
	}

	if (cleanUrl.startsWith('/')) {
		// Absolute path: convert with convertFileSrc
		return convertFileSrc(cleanUrl);
	}

	// Other formats: attempt to use directly
	return cleanUrl;
}
