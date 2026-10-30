import * as Effect from "effect/Effect";

const pending = new Map<string, {
  complete: (answer: unknown, error: string | null) => void;
  cancel: () => void;
}>();

export const serializePayload = (payload: unknown): string => {
  const seen = new Set<object>();
  const visit = (value: unknown): unknown => {
    if (value === null || typeof value === "string" || typeof value === "boolean") return value;
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value !== "object" || value === null) throw new TypeError("Ask payloads must contain only JSON data");
    if (seen.has(value)) throw new TypeError("Ask payloads cannot contain cycles");
    const array = Array.isArray(value);
    const prototype: unknown = Object.getPrototypeOf(value);
    if (prototype !== (array ? Array.prototype : Object.prototype) && prototype !== null) throw new TypeError("Ask payloads cannot contain methods");
    seen.add(value);
    const copy: Record<string, unknown> | unknown[] = array ? new Array(value.length) : Object.create(null) as Record<string, unknown>;
    Object.setPrototypeOf(copy, null);
    for (const key of Reflect.ownKeys(value)) {
      if (array && key === "length") continue;
      const descriptor = Object.getOwnPropertyDescriptor(value, key)!;
      if (typeof key === "symbol" || !("value" in descriptor)) throw new TypeError("Ask payloads cannot contain accessors or symbols");
      Object.defineProperty(copy, key, { value: visit(descriptor.value), enumerable: true });
    }
    seen.delete(value);
    return copy;
  };
  return JSON.stringify(visit(payload));
};

export const ask = (payload: unknown, host: Pick<typeof __host, "ask" | "cancelAsk">) =>
  Effect.callback<unknown>((resume) => {
    const id = host.ask(serializePayload(payload));
    pending.set(id, {
      complete: (answer, error) => resume(error === null ? Effect.succeed(answer) : Effect.die(new Error(error))),
      cancel: () => host.cancelAsk(id),
    });
    return Effect.sync(() => {
      pending.delete(id);
      host.cancelAsk(id);
    });
  });

export const completeAsk = (id: string, answer: unknown, error: string | null): void => {
  const resolve = pending.get(id);
  pending.delete(id);
  resolve?.complete(answer, error);
};

export const failAsks = (cause: Error): void => {
  for (const request of pending.values()) {
    request.cancel();
    request.complete(null, cause.message);
  }
  pending.clear();
};
