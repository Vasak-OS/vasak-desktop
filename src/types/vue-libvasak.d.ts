declare module '@vasakgroup/vue-libvasak' {
  import { Component, App } from 'vue';

  export interface WindowFrameProps {
    class?: string
    [key: string]: any
  }

  export interface SideBarProps {
    class?: string
    [key: string]: any
  }

  export interface SideButtonProps {
    class?: string
    [key: string]: any
  }

  export const WindowFrame: Component<WindowFrameProps>;
  export const SideBar: Component<SideBarProps>;
  export const SideButton: Component<SideButtonProps>;

  // Plugin de Vue
  export interface VueLibVasakPlugin {
    install(app: App): void
  }

  declare const plugin: VueLibVasakPlugin;
  export default plugin;
}
