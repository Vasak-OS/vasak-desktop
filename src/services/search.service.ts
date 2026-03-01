import { invoke } from '@tauri-apps/api/core';

export const executeSearchResult = <T = any>(args: any): Promise<T> => {
	return invoke<T>('execute_search_result', args);
};
