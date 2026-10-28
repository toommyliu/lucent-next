export {};

declare global {
  var __host: {
    swf(method: string, argsJson: string): string | undefined;
    read(queryJson: string): string;
  };
}

if (typeof window === "undefined") {
  Object.assign(globalThis, { window: globalThis });
}

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
