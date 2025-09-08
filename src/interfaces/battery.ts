export interface BatteryInfo {
  has_battery: boolean;
  percentage: number;
  state: string;
  time_to_empty?: number;
  time_to_full?: number;
  is_present: boolean;
  is_charging: boolean;
  vendor?: string;
  model?: string;
  technology?: string;
  energy?: number;
  energy_full?: number;
  energy_full_design?: number;
  voltage?: number;
  temperature?: number;
  serial?: string;
}
