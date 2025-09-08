export interface TrayItem {
  id: string;
  service_name: string;
  icon_name?: string;
  icon_data?: string;
  title?: string;
  tooltip?: string;
  status: "Active" | "Passive" | "NeedsAttention";
  category:
    | "ApplicationStatus"
    | "Communications"
    | "SystemServices"
    | "Hardware";
  menu_path?: string;
}

export interface TrayMenu {
  id: number;
  label: string;
  enabled: boolean;
  visible: boolean;
  type: "standard" | "separator" | "submenu";
  checked?: boolean;
  icon?: string;
  children?: TrayMenu[];
}
