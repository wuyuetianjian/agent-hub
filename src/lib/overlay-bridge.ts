export type OverlayEvent<T> = { event: string; payload: T };
export type OverlayInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;
export interface OverlayBridge {
  invoke: OverlayInvoke;
  listen<T>(
    event: string,
    callback: (event: OverlayEvent<T>) => void,
  ): Promise<() => void>;
}
interface OverlayHost {
  __ORBIT_HOST_INVOKE__?: (request: unknown) => Promise<void>;
}
declare global {
  interface Window extends OverlayHost {
    __ORBIT_OVERLAY__?: OverlayBridge;
  }
}
export function getOverlayBridge(): OverlayBridge | undefined {
  return typeof window === "undefined" ? undefined : window.__ORBIT_OVERLAY__;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object") return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

// The native parent also enforces its command allowlist. This layer rejects
// malformed messages and prevents the transport from invoking itself.
export async function installOverlayHost(
  host: OverlayHost,
  invoke: OverlayInvoke,
): Promise<void> {
  host.__ORBIT_HOST_INVOKE__ = async (request: unknown) => {
    if (!isPlainObject(request)) throw new Error("Invalid overlay request");
    const { id, command, args } = request;
    if (
      typeof id !== "number" ||
      !Number.isSafeInteger(id) ||
      id <= 0 ||
      typeof command !== "string" ||
      !/^[a-z][a-z0-9_]*$/.test(command) ||
      !isPlainObject(args) ||
      (command.startsWith("overlay_") &&
        command !== "overlay_open_url" &&
        command !== "overlay_choose_directory")
    ) {
      throw new Error("Invalid overlay request");
    }
    let ok = true;
    let value: unknown;
    try {
      value = (await invoke(command, args)) ?? null;
    } catch (error) {
      ok = false;
      value = error instanceof Error ? error.message : String(error);
    }
    // Do not catch a failed reply and recursively attempt to reply to it.
    await invoke("overlay_reply", { id, ok, value });
  };
  await invoke("overlay_host_ready");
}
