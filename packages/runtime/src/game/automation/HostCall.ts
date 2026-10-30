export class HostBoundaryFailure extends Error {
  constructor(cause: unknown) {
    super(cause instanceof Error ? cause.message : String(cause), { cause });
  }
}

export const callHost = <K extends keyof typeof __host>(
  method: K,
  ...args: Parameters<(typeof __host)[K]>
): ReturnType<(typeof __host)[K]> => {
  const invoke = __host[method];
  if (typeof invoke !== "function") {
    throw new TypeError(`Host method ${method} is not a function`);
  }
  try {
    return Reflect.apply(invoke, __host, args);
  } catch (cause) {
    throw new HostBoundaryFailure(cause);
  }
};
