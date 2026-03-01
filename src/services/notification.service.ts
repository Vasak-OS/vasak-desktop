import { invoke } from '@tauri-apps/api/core';

export const getAllNotifications = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_all_notifications', args);
};

export const deleteNotification = <T = any>(args: any): Promise<T> => {
	return invoke<T>('delete_notification', args);
};

export const clearNotifications = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('clear_notifications', args);
};

export const invokeNotificationAction = <T = any>(args: any): Promise<T> => {
	return invoke<T>('invoke_notification_action', args);
};
