import { invoke } from "@tauri-apps/api/core";
import { BatteryInfo } from "@/interfaces/battery";

export function fetchBatteryInfo(): Promise<BatteryInfo> {
  return invoke<BatteryInfo>("get_battery_info");
}

export function batteryExists(): Promise<boolean> {
  return invoke<boolean>("battery_exists");
}