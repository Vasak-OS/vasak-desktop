import { TrayItem } from "@/interfaces/tray";
import { invoke } from "@tauri-apps/api/core";

export async function startSNIWatcher(): Promise<void> {
  try {
    await invoke("init_sni_watcher");
  } catch (error) {
    console.error("[TrayPanel Error] Error inicializando SNI watcher:", error);
  }
}

export function getTrayItems(): Promise<TrayItem[]> {
  return invoke("get_tray_items");
}