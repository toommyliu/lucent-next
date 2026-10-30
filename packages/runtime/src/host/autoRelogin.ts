import type { AutoReloginRecords } from "@lucent/game/protocol";
import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";

import type { ScriptAutoReloginApi } from "@lucent/scripting-api/contract";

type AutoReloginState = Effect.Success<ReturnType<ScriptAutoReloginApi["getState"]>>;
const normalizeAutoReloginDelay = (delayMs: number): number => Number.isFinite(delayMs) ? Math.min(300_000, Math.max(0, delayMs)) : 3000;

export const publicReloginState = ({ config, status }: AutoReloginRecords): AutoReloginState => ({
  enabled: config.enabled,
  delayMs: config.delayMs,
  captured: status.captured,
  ...(status.username == null ? {} : { username: status.username }),
  attempting: ["awaiting-player-ready", "awaiting-server-select", "backoff", "logging-in", "selecting-server"].includes(status.phase.tag),
  waitingDelay: status.phase.tag === "waiting-delay" || status.phase.tag === "waiting-kick",
  ...(config.server === null ? {} : { server: config.server }),
  ...(status.attemptsRemaining === null ? {} : { attemptsRemaining: status.attemptsRemaining }),
  ...(status.lastError === null ? {} : { lastError: status.lastError }),
});

export const makeAutoReloginService = (host: Pick<typeof __host, "read" | "command">) => {
  const getState = () => Effect.sync(() => publicReloginState(JSON.parse(host.read('{"kind":"autoRelogin"}')) as AutoReloginRecords));
  const set = (payload: unknown) => Effect.sync(() => publicReloginState(JSON.parse(host.command(JSON.stringify(payload))) as AutoReloginRecords));
  return {
    getState,
    getDelayMs: () => getState().pipe(Effect.map((state) => state.delayMs)),
    getServer: () => getState().pipe(Effect.map((state) => state.server)),
    isEnabled: () => getState().pipe(Effect.map((state) => state.enabled)),
    setEnabled: (enabled: boolean) => set({ kind: "setAutoReloginEnabled", enabled }),
    setDelay: (delay: Duration.Input) => set({ kind: "setAutoReloginDelay", delayMs: normalizeAutoReloginDelay(Duration.toMillis(delay)) }),
    setServer: (server: string | undefined) => set({ kind: "setAutoReloginServer", server: server ?? null }),
  };
};
