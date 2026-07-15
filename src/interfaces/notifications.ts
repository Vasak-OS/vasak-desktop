export interface Notification {
	id: number;
	app_name: string;
	app_icon: string;
	summary: string;
	body: string;
	timestamp: number;
	seen: boolean;
	urgency?: string;
	actions?: string[];
	hints?: { [key: string]: string };
}

export interface NotificationGroupData {
	app_name: string;
	app_icon: string;
	notifications: Notification[];
	count: number;
	latest_timestamp: number;
	has_unread: boolean;
}

export type NotificationDelta =
	| { action: 'added'; notification: Notification; dropped_id: number | null }
	| { action: 'removed'; id: number }
	| { action: 'batch_update'; added: Notification[]; removed: number[] }
	| { action: 'cleared' };
