import { invoke } from '@tauri-apps/api/core';

export async function showOsd(icon: string, value: number, maximum: number, label: string) {
	return invoke('show_osd', { icon, value, maximum, label });
}
