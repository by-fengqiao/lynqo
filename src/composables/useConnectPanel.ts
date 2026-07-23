import type { InjectionKey } from "vue";

/**
 * Injection key for opening the connect-device panel hosted by
 * DesktopLayout. Pages rendered inside the layout (e.g. HomePage's
 * "查看连接地址" link) inject this to trigger the panel without
 * prop drilling through the router view.
 */
export const openConnectPanelKey: InjectionKey<() => void> =
  Symbol("open-connect-panel");
