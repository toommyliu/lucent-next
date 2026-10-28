"use strict";
(() => {
  var __defProp = Object.defineProperty;
  var __name = (target, value) => __defProp(target, "name", { value, configurable: true });

  // packages/runtime/src/host/index.ts
  var timers = /* @__PURE__ */ new Map();
  var nextId = 1;
  var schedule = /* @__PURE__ */ __name((callback, delay, args, repeat) => {
    const ms = Math.max(0, Number(delay) || 0);
    const timer = {
      id: nextId++,
      at: __host.now() + ms,
      callback: /* @__PURE__ */ __name(() => callback(...args), "callback"),
      ...repeat ? { repeat: ms } : {}
    };
    timers.set(timer.id, timer);
    return timer.id;
  }, "schedule");
  var clear = /* @__PURE__ */ __name((id) => {
    if (id !== void 0) timers.delete(id);
  }, "clear");
  var turn = /* @__PURE__ */ __name((itemsJson) => {
    const now = __host.now();
    const due = [...timers.values()].filter((timer) => timer.at <= now).sort((a, b) => a.at - b.at || a.id - b.id);
    const errors = [];
    const invoke = /* @__PURE__ */ __name((callback) => {
      try {
        callback();
      } catch (cause) {
        errors.push(cause instanceof Error ? cause.stack ?? String(cause) : String(cause));
      }
    }, "invoke");
    for (const item of JSON.parse(itemsJson)) {
      if (item.type === "loader") {
        invoke(() => {
          const callback = Reflect.get(window, item.name);
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
      if (timer.repeat === void 0) {
        timers.delete(timer.id);
      } else {
        timers.set(timer.id, { ...timer, at: __host.now() + timer.repeat });
      }
      invoke(timer.callback);
    }
    return errors;
  }, "turn");
  var wake = /* @__PURE__ */ __name(() => {
    let next = Infinity;
    for (const timer of timers.values()) next = Math.min(next, timer.at);
    return next;
  }, "wake");
  if (typeof window === "undefined") {
    Object.assign(globalThis, { window: globalThis });
  }
  Object.assign(globalThis, {
    __lucent: Object.freeze({ turn, wake }),
    setTimeout: /* @__PURE__ */ __name((callback, delay, ...args) => schedule(callback, delay, args, false), "setTimeout"),
    setInterval: /* @__PURE__ */ __name((callback, delay, ...args) => schedule(callback, delay, args, true), "setInterval"),
    clearTimeout: clear,
    clearInterval: clear
  });
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
