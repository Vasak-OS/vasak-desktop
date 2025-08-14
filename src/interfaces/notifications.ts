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
