"use strict";
(() => {
  // packages/runtime/src/host/index.ts
  if (typeof window === "undefined") {
    Object.assign(globalThis, { window: globalThis });
  }
  window.swf = new Proxy({}, {
    get(_target, method) {
      if (typeof method === "string") {
        return (...args) => {
          const reply = __host.swf(method, JSON.stringify(args));
          return reply === void 0 ? void 0 : JSON.parse(reply);
        };
      }
      return void 0;
    }
  });
})();
