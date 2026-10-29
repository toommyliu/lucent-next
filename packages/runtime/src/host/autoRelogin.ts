import type { AutoReloginRecords } from "@lucent/game/protocol";
import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";

import { normalizeAutoReloginDelay } from "../game/automation/AutoRelogin";
import type { AutoReloginState } from "../game/automation/AutoRelogin";
import { ask } from "./asks";

export const publicReloginState = ({ config, status }: AutoReloginRecords): AutoReloginState => ({
  enabled: config.enabled,
  delayMs: config.delayMs,
  captured: status.captured,
  attempting: ["awaiting-player-ready", "awaiting-server-select", "backoff", "logging-in", "selecting-server"].includes(status.phase.tag),
  waitingDelay: status.phase.tag === "waiting-delay" || status.phase.tag === "waiting-kick",
  ...(config.server === null ? {} : { server: config.server }),
  ...(status.attemptsRemaining === null ? {} : { attemptsRemaining: status.attemptsRemaining }),
  ...(status.lastError === null ? {} : { lastError: status.lastError }),
});

export const makeAutoReloginService = (host: Pick<typeof __host, "read" | "ask" | "cancelAsk">) => {
  const getState = () => Effect.sync(() => publicReloginState(JSON.parse(host.read('{"kind":"autoRelogin"}')) as AutoReloginRecords));
  const set = (payload: unknown) => ask(payload, host).pipe(Effect.map((answer) => answer as AutoReloginState));
  return {
    getState,
    getDelayMs: () => getState().pipe(Effect.map((state) => state.delayMs)),
    getServer: () => getState().pipe(Effect.map((state) => state.server)),
    isEnabled: () => getState().pipe(Effect.map((state) => state.enabled)),
    setEnabled: (enabled: boolean) => set({ kind: "autoRelogin.setEnabled", enabled }),
    setDelay: (delay: Duration.Input) => set({ kind: "autoRelogin.setDelay", delayMs: normalizeAutoReloginDelay(Duration.toMillis(delay)) }),
    setServer: (server: string | undefined) => set({ kind: "autoRelogin.setServer", server: server ?? null }),
  };
};
