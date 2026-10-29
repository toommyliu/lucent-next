import type { ProjectionEvent } from "@lucent/game/protocol";
import type * as Effect from "effect/Effect";

import type { ApiService } from "../game/flash/api/Api";

declare global {
  var __host: {
    swf(method: string, argsJson: string): string | undefined;
    read(queryJson: string): string;
    command(commandJson: string): string;
    now(): number;
  };
  var __lucent: Readonly<{
    run<A, E>(program: (api: ApiService) => Effect.Effect<A, E>): Promise<A>;
    turn(itemsJson: string): string[];
    wake(): number;
  }>;

  interface Window {
    onProjectionEvent?: (event: ProjectionEvent) => void;
    onProjectionInvalidated?: () => void;
  }
}

type ScriptEvent =
  | {
      type: "loader";
      name: string;
      args: unknown[];
      projected: ProjectionEvent[];
    }
  | { type: "invalidated" };

interface Timer {
  readonly id: number;
  readonly at: number;
  readonly callback: () => void;
  readonly repeat?: number;
}

const timers = new Map<number, Timer>();
let nextId = 1;

const schedule = (
  callback: (...args: unknown[]) => void,
  delay: number | undefined,
  args: unknown[],
  repeat: boolean,
): number => {
  const ms = Math.max(0, Number(delay) || 0);
  const timer: Timer = {
    id: nextId++,
    at: __host.now() + ms,
    callback: () => callback(...args),
    ...(repeat ? { repeat: ms } : {}),
  };
  timers.set(timer.id, timer);
  return timer.id;
};

class Utf8Encoder {
  readonly encoding = "utf-8";

  encode(input = ""): Uint8Array {
    const bytes = unescape(encodeURIComponent(input.toWellFormed()));
    return Uint8Array.from(bytes, (byte) => byte.charCodeAt(0));
  }
}

const clear = (id: number | undefined): void => {
  if (id !== undefined) timers.delete(id);
};

const describe = (cause: unknown): string => {
  try {
    return cause instanceof Error ? (cause.stack ?? String(cause)) : String(cause);
  } catch {
    return "a value that cannot be converted to text was thrown";
  }
};

export const turn = (itemsJson: string): string[] => {
  const now = __host.now();
  const due = [...timers.values()]
    .filter((timer) => timer.at <= now)
    .sort((a, b) => a.at - b.at || a.id - b.id);
  const errors: string[] = [];
  const invoke = (callback: () => void): void => {
    try {
      callback();
    } catch (cause) {
      errors.push(describe(cause));
    }
  };
  for (const item of JSON.parse(itemsJson) as ScriptEvent[]) {
    if (item.type === "loader") {
      invoke(() => {
        const callback: unknown = Reflect.get(window, item.name);
        if (typeof callback === "function") callback.apply(window, item.args);
      });
      for (const event of item.projected) {
        invoke(() => window.onProjectionEvent?.(event));
      }
    } else {
      invoke(() => window.onProjectionInvalidated?.());
    }
  }
  for (const timer of due) {
    if (timers.get(timer.id) !== timer) continue;
    if (timer.repeat === undefined) {
      timers.delete(timer.id);
    } else {
      timers.set(timer.id, { ...timer, at: __host.now() + timer.repeat });
    }
    invoke(timer.callback);
  }
  return errors;
};

export const wake = (): number => {
  let next = Infinity;
  for (const timer of timers.values()) next = Math.min(next, timer.at);
  return next;
};

if (typeof window === "undefined") {
  Object.assign(globalThis, { window: globalThis });
}

Object.assign(globalThis, {
  setTimeout: (callback: (...args: unknown[]) => void, delay?: number, ...args: unknown[]) =>
    schedule(callback, delay, args, false),
  setInterval: (callback: (...args: unknown[]) => void, delay?: number, ...args: unknown[]) =>
    schedule(callback, delay, args, true),
  clearTimeout: clear,
  clearInterval: clear,
  TextEncoder: Utf8Encoder,
});

window.swf = new Proxy({} as Window["swf"], {
  get(_target, method) {
    if (typeof method === "string") {
      return (...args: unknown[]) => {
        const reply = __host.swf(method, JSON.stringify(args));
        return reply === undefined ? undefined : JSON.parse(reply);
      };
    }
    return undefined;
  },
});
