import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

export const setPanelProperties = async (): Promise<void> => {
  try {
    const thisWindow = await getCurrentWindow();
    const { width: screenWidth } = await window.screen;
    await thisWindow.setSize(new LogicalSize(screenWidth, 32));
  } catch (error) {
    console.error("[Panel Error] Error configurando la ventana:", error);
  }
};
