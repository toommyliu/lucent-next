"use strict";
(() => {
  var __defProp = Object.defineProperty;
  var __name = (target, value) => __defProp(target, "name", { value, configurable: true });

  // packages/runtime/src/host/globals.ts
  var Event = class {
    static {
      __name(this, "Event");
    }
    type;
    cancelable;
    target = null;
    currentTarget = null;
    defaultPrevented = false;
    immediatePropagationStopped = false;
    constructor(type, options = {}) {
      this.type = type;
      this.cancelable = options.cancelable === true;
    }
    preventDefault() {
      if (this.cancelable) this.defaultPrevented = true;
    }
    stopImmediatePropagation() {
      this.immediatePropagationStopped = true;
    }
  };
  var EventTarget = class {
    static {
      __name(this, "EventTarget");
    }
    #listeners = /* @__PURE__ */ new Map();
    addEventListener(type, listener, options = {}) {
      if (listener === null) return;
      const capture = typeof options === "boolean" ? options : options.capture === true;
      const registrations = this.#listeners.get(type) ?? [];
      if (registrations.some(
        (entry) => entry.listener === listener && entry.capture === capture
      ))
        return;
      registrations.push({
        listener,
        capture,
        once: typeof options === "object" && options.once === true
      });
      this.#listeners.set(type, registrations);
    }
    removeEventListener(type, listener, options = {}) {
      const capture = typeof options === "boolean" ? options : options.capture === true;
      const registrations = this.#listeners.get(type);
      const index = registrations?.findIndex(
        (entry) => entry.listener === listener && entry.capture === capture
      ) ?? -1;
      if (index !== -1) registrations?.splice(index, 1);
    }
    dispatchEvent(event) {
      if (event.currentTarget !== null)
        throw new Error("The event is already being dispatched.");
      event.target = this;
      event.currentTarget = this;
      event.immediatePropagationStopped = false;
      try {
        const registrations = this.#listeners.get(event.type) ?? [];
        for (const entry of [...registrations]) {
          if (!registrations.includes(entry)) continue;
          if (entry.once)
            this.removeEventListener(event.type, entry.listener, entry.capture);
          try {
            if (typeof entry.listener === "function")
              entry.listener.call(this, event);
            else entry.listener.handleEvent(event);
          } catch (error) {
            console.error(error);
          }
          if (event.immediatePropagationStopped) break;
        }
      } finally {
        event.currentTarget = null;
      }
      return !event.defaultPrevented;
    }
  };
  var defaultAbortReason = /* @__PURE__ */ __name(() => {
    const error = new Error("This operation was aborted");
    error.name = "AbortError";
    return error;
  }, "defaultAbortReason");
  var abort = /* @__PURE__ */ Symbol("abort");
  var AbortSignal = class _AbortSignal extends EventTarget {
    static {
      __name(this, "AbortSignal");
    }
    #aborted = false;
    #reason;
    #onabort = null;
    #onabortRegistered = false;
    #dispatchOnabort = /* @__PURE__ */ __name((event) => this.#onabort?.call(this, event), "#dispatchOnabort");
    get aborted() {
      return this.#aborted;
    }
    get reason() {
      return this.#reason;
    }
    get onabort() {
      return this.#onabort;
    }
    set onabort(listener) {
      if (listener !== null && !this.#onabortRegistered) {
        this.addEventListener("abort", this.#dispatchOnabort);
        this.#onabortRegistered = true;
      }
      this.#onabort = listener;
    }
    throwIfAborted() {
      if (this.#aborted) throw this.#reason;
    }
    [abort](reason) {
      if (this.#aborted) return;
      this.#aborted = true;
      this.#reason = reason === void 0 ? defaultAbortReason() : reason;
      this.dispatchEvent(new Event("abort"));
    }
    static abort(reason) {
      const signal = new _AbortSignal();
      signal[abort](reason);
      return signal;
    }
  };
  var AbortController2 = class {
    static {
      __name(this, "AbortController");
    }
    #signal = new AbortSignal();
    get signal() {
      return this.#signal;
    }
    abort(reason) {
      this.#signal[abort](reason);
    }
  };
  var TextEncoder = class {
    static {
      __name(this, "TextEncoder");
    }
    encoding = "utf-8";
    encode(input = "") {
      const bytes = unescape(encodeURIComponent(input.toWellFormed()));
      return Uint8Array.from(bytes, (byte) => byte.charCodeAt(0));
    }
  };
  var format = /* @__PURE__ */ __name((value) => {
    if (typeof value === "string") return value;
    if (value instanceof Error) return value.stack ?? String(value);
    try {
      const ancestors = [];
      return JSON.stringify(value, function(_key, entry) {
        if (entry === null || typeof entry !== "object") return entry;
        while (ancestors.length > 0 && ancestors.at(-1) !== this)
          ancestors.pop();
        if (ancestors.includes(entry)) return "[Circular]";
        ancestors.push(entry);
        return entry;
      }) ?? String(value);
    } catch {
      try {
        return String(value);
      } catch {
        return "[Unserializable]";
      }
    }
  }, "format");
  var host = typeof __host === "undefined" ? void 0 : __host;
  var log = /* @__PURE__ */ __name((level) => (...values) => host?.log(level, values.map(format).join(" ")), "log");
  var runtimeConsole = Object.freeze({
    log: log("info"),
    info: log("info"),
    warn: log("warn"),
    error: log("error"),
    debug: log("debug")
  });
  if (typeof __host !== "undefined") {
    Object.assign(globalThis, {
      Event,
      EventTarget,
      AbortController: AbortController2,
      AbortSignal,
      TextEncoder,
      console: runtimeConsole
    });
  }

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Pipeable.js
  var pipeArguments = /* @__PURE__ */ __name((self, args2) => {
    switch (args2.length) {
      case 0:
        return self;
      case 1:
        return args2[0](self);
      case 2:
        return args2[1](args2[0](self));
      case 3:
        return args2[2](args2[1](args2[0](self)));
      case 4:
        return args2[3](args2[2](args2[1](args2[0](self))));
      case 5:
        return args2[4](args2[3](args2[2](args2[1](args2[0](self)))));
      case 6:
        return args2[5](args2[4](args2[3](args2[2](args2[1](args2[0](self))))));
      case 7:
        return args2[6](args2[5](args2[4](args2[3](args2[2](args2[1](args2[0](self)))))));
      case 8:
        return args2[7](args2[6](args2[5](args2[4](args2[3](args2[2](args2[1](args2[0](self))))))));
      case 9:
        return args2[8](args2[7](args2[6](args2[5](args2[4](args2[3](args2[2](args2[1](args2[0](self)))))))));
      default: {
        let ret = self;
        for (let i = 0, len = args2.length; i < len; i++) {
          ret = args2[i](ret);
        }
        return ret;
      }
    }
  }, "pipeArguments");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Function.js
  var dual = /* @__PURE__ */ __name(function(arity, body) {
    if (typeof arity === "function") {
      return function() {
        return arity(arguments) ? body.apply(this, arguments) : (self) => body(self, ...arguments);
      };
    }
    switch (arity) {
      case 0:
      case 1:
        throw new RangeError(`Invalid arity ${arity}`);
      case 2:
        return function(a, b) {
          if (arguments.length >= 2) {
            return body(a, b);
          }
          return function(self) {
            return body(self, a);
          };
        };
      case 3:
        return function(a, b, c) {
          if (arguments.length >= 3) {
            return body(a, b, c);
          }
          return function(self) {
            return body(self, a, b);
          };
        };
      default:
        return function() {
          if (arguments.length >= arity) {
            return body.apply(this, arguments);
          }
          const args2 = arguments;
          return function(self) {
            return body(self, ...args2);
          };
        };
    }
  }, "dual");
  var identity = /* @__PURE__ */ __name((a) => a, "identity");
  var constant = /* @__PURE__ */ __name((value) => () => value, "constant");
  var constTrue = /* @__PURE__ */ constant(true);
  var constUndefined = /* @__PURE__ */ constant(void 0);
  var constVoid = constUndefined;

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/equal.js
  var getAllObjectKeys = /* @__PURE__ */ __name((obj) => {
    const keys = new Set(Reflect.ownKeys(obj));
    if (obj.constructor === Object) return keys;
    if (obj instanceof Error) {
      keys.delete("stack");
    }
    const proto = Object.getPrototypeOf(obj);
    let current = proto;
    while (current !== null && current !== Object.prototype) {
      const ownKeys = Reflect.ownKeys(current);
      for (let i = 0; i < ownKeys.length; i++) {
        keys.add(ownKeys[i]);
      }
      current = Object.getPrototypeOf(current);
    }
    if (keys.has("constructor") && typeof obj.constructor === "function" && proto === obj.constructor.prototype) {
      keys.delete("constructor");
    }
    return keys;
  }, "getAllObjectKeys");
  var byReferenceInstances = /* @__PURE__ */ new WeakSet();
  var viewBytes = /* @__PURE__ */ __name((view) => new Uint8Array(view.buffer, view.byteOffset, view.byteLength), "viewBytes");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/hash.js
  var backEdges = 0;
  var addBackEdge = /* @__PURE__ */ __name(() => {
    backEdges++;
  }, "addBackEdge");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Predicate.js
  function isString(input) {
    return typeof input === "string";
  }
  __name(isString, "isString");
  function isNumber(input) {
    return typeof input === "number";
  }
  __name(isNumber, "isNumber");
  function isFunction(input) {
    return typeof input === "function";
  }
  __name(isFunction, "isFunction");
  function isObjectKeyword(input) {
    return typeof input === "object" && input !== null || isFunction(input);
  }
  __name(isObjectKeyword, "isObjectKeyword");
  var hasProperty = /* @__PURE__ */ dual(2, (self, property) => isObjectKeyword(self) && property in self);
  function isIterable(input) {
    return hasProperty(input, Symbol.iterator) || isString(input);
  }
  __name(isIterable, "isIterable");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Hash.js
  var symbol = "~effect/Hash";
  var hash = /* @__PURE__ */ __name((self) => {
    switch (typeof self) {
      case "number":
        return number(self);
      case "bigint":
        return string(self.toString(10));
      case "string":
        return string(self);
      case "function":
      case "object": {
        if (self === null) {
          break;
        } else if (self instanceof Date) {
          if (Number.isNaN(self.getTime())) {
            return string("Invalid Date");
          }
          return string(self.toISOString());
        } else if (self instanceof RegExp) {
          return string(self.toString());
        } else {
          if (byReferenceInstances.has(self)) {
            return random(self);
          }
          const cached2 = hashCache.get(self);
          if (cached2 !== void 0) {
            return cached2;
          }
          if (visitedObjects.has(self)) {
            addBackEdge();
            return string("[Circular]");
          }
          visitedObjects.add(self);
          const seen = backEdges;
          let h;
          try {
            if (symbol in self) {
              h = self[symbol]();
            } else if (typeof self === "function") {
              h = random(self);
            } else if (self instanceof DataView) {
              h = array(viewBytes(self));
            } else if (Array.isArray(self) || ArrayBuffer.isView(self)) {
              h = array(self);
            } else if (self instanceof Map) {
              h = hashMap(self);
            } else if (self instanceof Set) {
              h = hashSet(self);
            } else {
              h = structure(self);
            }
          } finally {
            visitedObjects.delete(self);
          }
          if (seen === backEdges) {
            hashCache.set(self, h);
          }
          return h;
        }
      }
    }
    return optimize(mix(string(String(self))));
  }, "hash");
  var random = /* @__PURE__ */ __name((self) => {
    if (!randomHashCache.has(self)) {
      randomHashCache.set(self, optimize(Math.random() * 4294967296 | 0));
    }
    return randomHashCache.get(self);
  }, "random");
  var mix = /* @__PURE__ */ __name((h) => {
    h ^= h >>> 16;
    h = Math.imul(h, 2246822507);
    h ^= h >>> 13;
    h = Math.imul(h, 3266489909);
    return h ^ h >>> 16;
  }, "mix");
  var combine = /* @__PURE__ */ dual(2, (self, b) => mix(Math.imul(self, 2654435761) + Math.imul(b, 2246822507)));
  var optimize = /* @__PURE__ */ __name((n) => n & 3221225471 | n >>> 1 & 1073741824, "optimize");
  var float64 = /* @__PURE__ */ new DataView(/* @__PURE__ */ new ArrayBuffer(8));
  var number = /* @__PURE__ */ __name((n) => {
    const h = n | 0;
    if (h === n) {
      return optimize(h);
    }
    float64.setFloat64(0, n !== n ? NaN : n);
    return optimize(combine(float64.getInt32(0), float64.getInt32(4)));
  }, "number");
  var string = /* @__PURE__ */ __name((str) => {
    let h = 5381, i = str.length;
    while (i) {
      h = h * 33 ^ str.charCodeAt(--i);
    }
    return optimize(h);
  }, "string");
  var structureKeys = /* @__PURE__ */ __name((o, keys) => {
    let h = 12289;
    for (const key of keys) {
      h ^= combine(hash(key), hash(o[key]));
    }
    return optimize(h);
  }, "structureKeys");
  var structure = /* @__PURE__ */ __name((o) => structureKeys(o, getAllObjectKeys(o)), "structure");
  var unordered = /* @__PURE__ */ __name((seed, f) => (iter) => {
    let h = seed;
    for (const element of iter) {
      h ^= f(element);
    }
    return optimize(h);
  }, "unordered");
  var array = /* @__PURE__ */ __name((arr) => {
    let h = 6151;
    for (const element of arr) {
      h = combine(h, hash(element));
    }
    return optimize(h);
  }, "array");
  var hashMap = /* @__PURE__ */ unordered(/* @__PURE__ */ string("Map"), ([k, v]) => combine(hash(k), hash(v)));
  var setSeed = /* @__PURE__ */ string("Set");
  var hashSet = /* @__PURE__ */ unordered(setSeed, (element) => combine(setSeed, hash(element)));
  var randomHashCache = /* @__PURE__ */ new WeakMap();
  var hashCache = /* @__PURE__ */ new WeakMap();
  var visitedObjects = /* @__PURE__ */ new WeakSet();

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Equal.js
  var symbol2 = "~effect/Equal";
  function equals() {
    if (arguments.length === 1) {
      return (self) => compareBoth(self, arguments[0]);
    }
    return compareBoth(arguments[0], arguments[1]);
  }
  __name(equals, "equals");
  function compareBoth(self, that) {
    if (self === that) return true;
    if (self == null || that == null) return false;
    const selfType = typeof self;
    if (selfType !== typeof that) {
      return false;
    }
    if (selfType === "number" && self !== self && that !== that) {
      return true;
    }
    if (selfType !== "object" && selfType !== "function") {
      return false;
    }
    if (byReferenceInstances.has(self) || byReferenceInstances.has(that)) {
      return false;
    }
    return compareObjects(self, that);
  }
  __name(compareBoth, "compareBoth");
  function compareObjects(self, that) {
    const depth = pathLeft.length;
    for (let i = depth; i-- > 0; ) {
      if (pathLeft[i] === self && pathRight[i] === that) return true;
    }
    if (depth) return compareOnPath(self, that);
    let known = results.get(self);
    if (!known) results.set(self, known = /* @__PURE__ */ new WeakMap());
    let result2 = known.get(that);
    if (result2 === void 0) known.set(that, result2 = compareOnPath(self, that));
    return result2;
  }
  __name(compareObjects, "compareObjects");
  function compareOnPath(self, that) {
    pathLeft.push(self);
    pathRight.push(that);
    try {
      return compareStructure(self, that);
    } finally {
      pathLeft.pop();
      pathRight.pop();
    }
  }
  __name(compareOnPath, "compareOnPath");
  var pathLeft = [];
  var pathRight = [];
  var results = /* @__PURE__ */ new WeakMap();
  function compareStructure(self, that) {
    if (hash(self) !== hash(that)) {
      return false;
    } else if (self instanceof Date) {
      if (!(that instanceof Date)) return false;
      const selfTime = self.getTime();
      const thatTime = that.getTime();
      return selfTime === thatTime || Number.isNaN(selfTime) && Number.isNaN(thatTime);
    } else if (self instanceof RegExp) {
      return that instanceof RegExp && self.toString() === that.toString();
    }
    const bothEquals = isEqual(self);
    if (bothEquals !== isEqual(that) || typeof self === "function" && !bothEquals) {
      return false;
    } else if (bothEquals) {
      return self[symbol2](that);
    } else if (Array.isArray(self)) {
      if (!Array.isArray(that) || self.length !== that.length) {
        return false;
      }
      return compareArrays(self, that);
    } else if (ArrayBuffer.isView(self)) {
      const selfIsDataView = self instanceof DataView;
      if (!ArrayBuffer.isView(that) || self.byteLength !== that.byteLength || selfIsDataView !== that instanceof DataView) {
        return false;
      }
      if (selfIsDataView) {
        return compareTypedArrays(viewBytes(self), viewBytes(that));
      }
      return compareTypedArrays(self, that);
    } else if (self instanceof Map) {
      if (!(that instanceof Map) || self.size !== that.size) {
        return false;
      }
      return compareHashed(self, that, entryHash, equalEntries);
    } else if (self instanceof Set) {
      if (!(that instanceof Set) || self.size !== that.size) {
        return false;
      }
      return compareHashed(self, that, hash, compareBoth);
    }
    return compareRecords(self, that);
  }
  __name(compareStructure, "compareStructure");
  function compareArrays(self, that) {
    for (let i = 0; i < self.length; i++) {
      if (!compareBoth(self[i], that[i])) {
        return false;
      }
    }
    return true;
  }
  __name(compareArrays, "compareArrays");
  function compareTypedArrays(self, that) {
    if (self.length !== that.length) {
      return false;
    }
    for (let i = 0; i < self.length; i++) {
      if (self[i] !== that[i]) {
        return false;
      }
    }
    return true;
  }
  __name(compareTypedArrays, "compareTypedArrays");
  function compareRecords(self, that) {
    const selfKeys = getAllObjectKeys(self);
    const thatKeys = getAllObjectKeys(that);
    if (selfKeys.size !== thatKeys.size) {
      return false;
    }
    for (const key of selfKeys) {
      if (!thatKeys.has(key) || !compareBoth(self[key], that[key])) {
        return false;
      }
    }
    return true;
  }
  __name(compareRecords, "compareRecords");
  function compareHashed(self, that, hashOf, equivalent) {
    const groups = /* @__PURE__ */ new Map();
    let remaining = 0;
    for (const item of that) {
      const h = hashOf(item);
      const group = groups.get(h);
      if (group) group.push(item);
      else groups.set(h, [item]);
      remaining++;
    }
    outer: for (const item of self) {
      const group = groups.get(hashOf(item));
      if (group) {
        for (let i = 0; i < group.length; i++) {
          if (equivalent(item, group[i])) {
            group[i] = group[group.length - 1];
            group.pop();
            remaining--;
            continue outer;
          }
        }
      }
      return false;
    }
    return remaining === 0;
  }
  __name(compareHashed, "compareHashed");
  var entryHash = /* @__PURE__ */ __name((entry) => hash(entry[0]), "entryHash");
  var equalEntries = /* @__PURE__ */ __name((self, that) => compareBoth(self[0], that[0]) && compareBoth(self[1], that[1]), "equalEntries");
  var isEqual = /* @__PURE__ */ __name((u) => hasProperty(u, symbol2), "isEqual");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Redactable.js
  var symbolRedactable = /* @__PURE__ */ Symbol.for("~effect/Redactable");
  var isRedactable = /* @__PURE__ */ __name((u) => hasProperty(u, symbolRedactable), "isRedactable");
  function redact(u) {
    if (isRedactable(u)) return getRedacted(u);
    return u;
  }
  __name(redact, "redact");
  function getRedacted(redactable) {
    return redactable[symbolRedactable](globalThis[currentFiberTypeId]?.context ?? emptyContext);
  }
  __name(getRedacted, "getRedacted");
  var currentFiberTypeId = "~effect/Fiber/currentFiber";
  var emptyMap = /* @__PURE__ */ new Map();
  var emptyContext = {
    "~effect/Context": {},
    base: emptyMap,
    depth: 0,
    mapUnsafe: emptyMap,
    pipe() {
      return pipeArguments(this, arguments);
    }
  };

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Formatter.js
  function format2(input, options) {
    const space = options?.space ?? 0;
    const ancestors = /* @__PURE__ */ new WeakSet();
    const gap = !space ? "" : typeof space === "number" ? " ".repeat(space) : space;
    const ind = /* @__PURE__ */ __name((d) => gap.repeat(d), "ind");
    const wrap = /* @__PURE__ */ __name((v, body) => {
      const ctor = v?.constructor;
      return ctor && ctor !== Object.prototype.constructor && ctor.name ? `${ctor.name}(${body})` : body;
    }, "wrap");
    const ownKeys = /* @__PURE__ */ __name((o) => {
      try {
        return Reflect.ownKeys(o);
      } catch {
        return ["[ownKeys threw]"];
      }
    }, "ownKeys");
    function recur(v, d = 0) {
      try {
        return recurUnsafe(v, d);
      } catch {
        if (typeof v === "object" && v !== null || typeof v === "function") ancestors.delete(v);
        return "[inspection threw]";
      }
    }
    __name(recur, "recur");
    function recurUnsafe(v, d = 0) {
      if (typeof v === "string") return JSON.stringify(v);
      if (typeof v === "number" || v == null || typeof v === "boolean" || typeof v === "symbol") return String(v);
      if (typeof v === "bigint") return String(v) + "n";
      if (typeof v === "object" || typeof v === "function") {
        if (ancestors.has(v)) return CIRCULAR;
        ancestors.add(v);
        let output;
        if (symbolRedactable in v) {
          output = recur(getRedacted(v), d);
        } else if (Array.isArray(v)) {
          output = !gap || v.length <= 1 ? `[${v.map((x) => recur(x, d)).join(",")}]` : `[
${ind(d + 1)}${v.map((x) => recur(x, d + 1)).join(",\n" + ind(d + 1))}
${ind(d)}]`;
        } else if (v instanceof Date) {
          output = formatDate(v);
        } else if (!options?.ignoreToString && hasProperty(v, "toString") && typeof v["toString"] === "function" && v["toString"] !== Object.prototype.toString && v["toString"] !== Array.prototype.toString) {
          const s = safeToString(v);
          output = v instanceof Error && v.cause !== void 0 ? `${s} (cause: ${recur(v.cause, d)})` : s;
        } else if (Symbol.iterator in v) {
          output = `${v.constructor.name}(${recur(Array.from(v), d)})`;
        } else {
          const keys = ownKeys(v);
          if (!gap || keys.length <= 1) {
            const body = `{${keys.map((k) => `${formatPropertyKey(k)}:${recur(safeGet(v, k), d)}`).join(",")}}`;
            output = wrap(v, body);
          } else {
            const body = `{
${keys.map((k) => `${ind(d + 1)}${formatPropertyKey(k)}: ${recur(safeGet(v, k), d + 1)}`).join(",\n")}
${ind(d)}}`;
            output = wrap(v, body);
          }
        }
        ancestors.delete(v);
        return output;
      }
      return String(v);
    }
    __name(recurUnsafe, "recurUnsafe");
    return recur(input, 0);
  }
  __name(format2, "format");
  var CIRCULAR = "[Circular]";
  function formatPropertyKey(name) {
    return typeof name === "string" ? JSON.stringify(name) : String(name);
  }
  __name(formatPropertyKey, "formatPropertyKey");
  function formatDate(date) {
    try {
      return date.toISOString();
    } catch {
      return "Invalid Date";
    }
  }
  __name(formatDate, "formatDate");
  function safeToString(input) {
    try {
      const s = input.toString();
      return typeof s === "string" ? s : String(s);
    } catch {
      return "[toString threw]";
    }
  }
  __name(safeToString, "safeToString");
  function safeGet(input, key) {
    try {
      return input[key];
    } catch {
      return "[property access threw]";
    }
  }
  __name(safeGet, "safeGet");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Inspectable.js
  var NodeInspectSymbol = /* @__PURE__ */ Symbol.for("nodejs.util.inspect.custom");
  var toJson = /* @__PURE__ */ __name((input) => {
    try {
      input = redact(input);
      if (hasProperty(input, "toJSON") && isFunction(input["toJSON"]) && input["toJSON"].length === 0) {
        return input.toJSON();
      } else if (Array.isArray(input)) {
        return input.map(toJson);
      }
      return input;
    } catch {
      return "[toJSON threw]";
    }
  }, "toJson");
  var BaseProto = {
    toJSON() {
      return toJson(this);
    },
    [NodeInspectSymbol]() {
      return this.toJSON();
    },
    toString() {
      return format2(this.toJSON());
    }
  };
  var Class = class {
    static {
      __name(this, "Class");
    }
    /**
     * Node.js custom inspection method.
     *
     * **When to use**
     *
     * Use to expose the class JSON representation to Node.js inspection.
     *
     * @since 2.0.0
     */
    [NodeInspectSymbol]() {
      return this.toJSON();
    }
    /**
     * Returns a formatted string representation of this object.
     *
     * **When to use**
     *
     * Use to format the class JSON representation as a string.
     *
     * @since 2.0.0
     */
    toString() {
      return format2(this.toJSON());
    }
  };

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/stackTraceLimit.js
  var getStackTraceLimit = /* @__PURE__ */ __name(() => Error.stackTraceLimit, "getStackTraceLimit");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Utils.js
  var SingleShotGen = class {
    static {
      __name(this, "SingleShotGen");
    }
    constructor(self) {
      this.value = self;
      this.done = false;
    }
    /**
     * Yields the stored value once, then completes with the value sent back in.
     *
     * **When to use**
     *
     * Use to advance a `SingleShotGen` through its single yield and completion
     * step.
     *
     * @since 2.0.0
     */
    next(a) {
      if (this.done) {
        this.value = a;
        return this;
      }
      this.done = true;
      return {
        value: this.value,
        done: false
      };
    }
  };
  var pickInternalCall = /* @__PURE__ */ __name(() => {
    const InternalTypeId = "~effect/Utils/internal";
    const standard = {
      [InternalTypeId]: (body) => {
        return body();
      }
    };
    const forced = {
      [InternalTypeId]: (body) => {
        try {
          return body();
        } finally {
        }
      }
    };
    const isNotOptimizedAway = getStackTraceLimit() !== 0 && standard[InternalTypeId](() => new Error().stack)?.includes(InternalTypeId) === true;
    return isNotOptimizedAway ? standard[InternalTypeId] : forced[InternalTypeId];
  }, "pickInternalCall");
  var internalCall = /* @__PURE__ */ pickInternalCall();

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/record.js
  function assignProperty(self, key, value) {
    if (key === "__proto__") {
      Object.defineProperty(self, key, {
        value,
        writable: true,
        enumerable: true,
        configurable: true
      });
    } else {
      ;
      self[key] = value;
    }
  }
  __name(assignProperty, "assignProperty");
  function assignProperties(self, source) {
    for (const key of Reflect.ownKeys(source)) {
      if (Object.prototype.propertyIsEnumerable.call(source, key)) {
        assignProperty(self, key, source[key]);
      }
    }
  }
  __name(assignProperties, "assignProperties");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/core.js
  var EffectTypeId = `~effect/Effect`;
  var ExitTypeId = `~effect/Exit`;
  var effectVariance = {
    _A: identity,
    _E: identity,
    _R: identity
  };
  var identifier = `${EffectTypeId}/identifier`;
  var args = `${EffectTypeId}/args`;
  var evaluate = `${EffectTypeId}/evaluate`;
  var contA = `${EffectTypeId}/successCont`;
  var contE = `${EffectTypeId}/failureCont`;
  var contAll = `${EffectTypeId}/ensureCont`;
  var Yield = /* @__PURE__ */ Symbol.for("effect/Effect/Yield");
  var PipeInspectableProto = {
    pipe() {
      return pipeArguments(this, arguments);
    },
    toJSON() {
      return {
        ...this
      };
    },
    toString() {
      return format2(this.toJSON(), {
        ignoreToString: true,
        space: 2
      });
    },
    [NodeInspectSymbol]() {
      return this.toJSON();
    }
  };
  var StructuralProto = {
    [symbol]() {
      return structureKeys(this, Object.keys(this));
    },
    [symbol2](that) {
      const selfKeys = Object.keys(this);
      const thatKeys = Object.keys(that);
      if (selfKeys.length !== thatKeys.length) return false;
      for (let i = 0; i < selfKeys.length; i++) {
        if (selfKeys[i] !== thatKeys[i] || !equals(this[selfKeys[i]], that[selfKeys[i]])) {
          return false;
        }
      }
      return true;
    }
  };
  var EffectProto = {
    [EffectTypeId]: effectVariance,
    ...PipeInspectableProto,
    [Symbol.iterator]() {
      return new SingleShotGen(this);
    },
    toJSON() {
      return {
        _id: "Effect",
        op: this[identifier],
        ...args in this ? {
          args: this[args]
        } : void 0
      };
    }
  };
  var isEffect = /* @__PURE__ */ __name((u) => hasProperty(u, EffectTypeId), "isEffect");
  var isExit = /* @__PURE__ */ __name((u) => hasProperty(u, ExitTypeId), "isExit");
  var CauseTypeId = "~effect/Cause";
  var CauseReasonTypeId = "~effect/Cause/Reason";
  var isCause = /* @__PURE__ */ __name((self) => hasProperty(self, CauseTypeId), "isCause");
  var CauseImpl = class {
    static {
      __name(this, "CauseImpl");
    }
    constructor(failures) {
      this[CauseTypeId] = CauseTypeId;
      this.reasons = failures;
    }
    pipe() {
      return pipeArguments(this, arguments);
    }
    toJSON() {
      return {
        _id: "Cause",
        failures: this.reasons.map((f) => f.toJSON())
      };
    }
    toString() {
      return `Cause(${format2(this.reasons)})`;
    }
    [NodeInspectSymbol]() {
      return this.toJSON();
    }
    [symbol2](that) {
      return isCause(that) && this.reasons.length === that.reasons.length && this.reasons.every((e, i) => equals(e, that.reasons[i]));
    }
    [symbol]() {
      return array(this.reasons);
    }
  };
  var annotationsMap = /* @__PURE__ */ new WeakMap();
  var ReasonBase = class {
    static {
      __name(this, "ReasonBase");
    }
    [CauseReasonTypeId];
    annotations;
    _tag;
    constructor(_tag, annotations, originalError) {
      this[CauseReasonTypeId] = CauseReasonTypeId;
      this._tag = _tag;
      if (annotations !== constEmptyAnnotations && typeof originalError === "object" && originalError !== null && annotations.size > 0) {
        const prevAnnotations = annotationsMap.get(originalError);
        if (prevAnnotations) {
          annotations = new Map([...prevAnnotations, ...annotations]);
        }
        annotationsMap.set(originalError, annotations);
      }
      this.annotations = annotations;
    }
    annotate(annotations, options) {
      if (annotations.mapUnsafe.size === 0) return this;
      const newAnnotations = new Map(this.annotations);
      annotations.mapUnsafe.forEach((value, key) => {
        if (options?.overwrite !== true && newAnnotations.has(key)) return;
        newAnnotations.set(key, value);
      });
      const self = Object.assign(Object.create(Object.getPrototypeOf(this)), this);
      self.annotations = newAnnotations;
      return self;
    }
    pipe() {
      return pipeArguments(this, arguments);
    }
    toString() {
      return format2(this);
    }
    [NodeInspectSymbol]() {
      return this.toString();
    }
  };
  var constEmptyAnnotations = /* @__PURE__ */ new Map();
  var Fail = class extends ReasonBase {
    static {
      __name(this, "Fail");
    }
    constructor(error, annotations = constEmptyAnnotations) {
      super("Fail", annotations, error);
      this.error = error;
    }
    toString() {
      return `Fail(${format2(this.error)})`;
    }
    toJSON() {
      return {
        _tag: "Fail",
        error: this.error
      };
    }
    [symbol2](that) {
      return isFailReason(that) && equals(this.error, that.error) && equals(this.annotations, that.annotations);
    }
    [symbol]() {
      return combine(string(this._tag))(combine(hash(this.error))(hash(this.annotations)));
    }
  };
  var causeFromReasons = /* @__PURE__ */ __name((reasons) => new CauseImpl(reasons), "causeFromReasons");
  var dedupeReasons = /* @__PURE__ */ __name((self, that) => {
    const buckets = /* @__PURE__ */ new Map();
    const out = [];
    for (const reason of self.concat(that)) {
      const hash2 = hash(reason);
      const bucket = buckets.get(hash2);
      if (bucket === void 0) {
        buckets.set(hash2, [reason]);
      } else if (bucket.some((previous) => equals(previous, reason))) {
        continue;
      } else {
        bucket.push(reason);
      }
      out.push(reason);
    }
    return out;
  }, "dedupeReasons");
  var causeCombine = /* @__PURE__ */ dual(2, (self, that) => {
    if (self.reasons.length === 0) {
      return that;
    } else if (that.reasons.length === 0) {
      return self;
    }
    const newCause = new CauseImpl(dedupeReasons(self.reasons, that.reasons));
    return equals(self, newCause) ? self : newCause;
  });
  var causeFail = /* @__PURE__ */ __name((error) => new CauseImpl([new Fail(error)]), "causeFail");
  var Die = class extends ReasonBase {
    static {
      __name(this, "Die");
    }
    constructor(defect, annotations = constEmptyAnnotations) {
      super("Die", annotations, defect);
      this.defect = defect;
    }
    toString() {
      return `Die(${format2(this.defect)})`;
    }
    toJSON() {
      return {
        _tag: "Die",
        defect: this.defect
      };
    }
    [symbol2](that) {
      return isDieReason(that) && equals(this.defect, that.defect) && equals(this.annotations, that.annotations);
    }
    [symbol]() {
      return combine(string(this._tag))(combine(hash(this.defect))(hash(this.annotations)));
    }
  };
  var causeDie = /* @__PURE__ */ __name((defect) => new CauseImpl([new Die(defect)]), "causeDie");
  var causeAnnotate = /* @__PURE__ */ dual((args2) => isCause(args2[0]), (self, annotations, options) => {
    if (annotations.mapUnsafe.size === 0) return self;
    return new CauseImpl(self.reasons.map((f) => f.annotate(annotations, options)));
  });
  var isFailReason = /* @__PURE__ */ __name((self) => self._tag === "Fail", "isFailReason");
  var isDieReason = /* @__PURE__ */ __name((self) => self._tag === "Die", "isDieReason");
  var isInterruptReason = /* @__PURE__ */ __name((self) => self._tag === "Interrupt", "isInterruptReason");
  function defaultEvaluate(_fiber) {
    return exitDie(`Effect.evaluate: Not implemented`);
  }
  __name(defaultEvaluate, "defaultEvaluate");
  var makePrimitiveProto = /* @__PURE__ */ __name((options) => ({
    ...EffectProto,
    [identifier]: options.op,
    [evaluate]: options[evaluate] ?? defaultEvaluate,
    [contA]: options[contA],
    [contE]: options[contE],
    [contAll]: options[contAll]
  }), "makePrimitiveProto");
  var makePrimitive = /* @__PURE__ */ __name((options) => {
    const Proto2 = makePrimitiveProto(options);
    const PrimitiveImpl = /* @__PURE__ */ __name(function(value) {
      this[args] = value;
    }, "PrimitiveImpl");
    PrimitiveImpl.prototype = Proto2;
    return function(value) {
      return new PrimitiveImpl(value);
    };
  }, "makePrimitive");
  var makeExit = /* @__PURE__ */ __name((options) => {
    const Proto2 = {
      [ExitTypeId]: ExitTypeId,
      _tag: options.op,
      get [options.prop]() {
        return this[args];
      },
      ...makePrimitiveProto(options),
      toString() {
        return `${options.op}(${format2(this[args])})`;
      },
      toJSON() {
        return {
          _id: "Exit",
          _tag: options.op,
          [options.prop]: this[args]
        };
      },
      [symbol2](that) {
        return isExit(that) && that._tag === this._tag && equals(this[args], that[args]);
      },
      [symbol]() {
        return combine(string(options.op), hash(this[args]));
      }
    };
    const ExitPrimitive = /* @__PURE__ */ __name(function(value) {
      this[args] = value;
    }, "ExitPrimitive");
    ExitPrimitive.prototype = Proto2;
    return function(value) {
      return new ExitPrimitive(value);
    };
  }, "makeExit");
  var exitSucceed = /* @__PURE__ */ makeExit({
    op: "Success",
    prop: "value",
    [evaluate](fiber2) {
      const cont = fiber2.getCont(contA);
      return cont ? cont[contA](this[args], fiber2, this) : fiber2.yieldWith(this);
    }
  });
  var StackTraceKey = {
    key: "effect/Cause/StackTrace"
  };
  var InterruptorStackTrace = {
    key: "effect/Cause/InterruptorStackTrace"
  };
  var exitFailCause = /* @__PURE__ */ makeExit({
    op: "Failure",
    prop: "cause",
    [evaluate](fiber2) {
      let cause = this[args];
      let annotated = false;
      if (fiber2.cache.stackFrame) {
        cause = causeAnnotate(cause, {
          mapUnsafe: /* @__PURE__ */ new Map([[StackTraceKey.key, fiber2.cache.stackFrame]])
        });
        annotated = true;
      }
      let cont = fiber2.getCont(contE);
      const interruptedCause = fiber2._interruptedCause;
      if (interruptedCause && fiber2.interruptible) {
        let skippedHandler = false;
        while (cont && fiber2.interruptible) {
          skippedHandler ||= identifier in cont;
          cont = fiber2.getCont(contE);
        }
        if (skippedHandler) {
          cause = causeFromReasons(cause.reasons.filter((reason) => reason._tag !== "Fail"));
        }
        cause = causeCombine(cause, interruptedCause);
        annotated = true;
      }
      return cont ? cont[contE](cause, fiber2, annotated ? void 0 : this) : fiber2.yieldWith(annotated ? exitFailCause(cause) : this);
    }
  });
  var exitFail = /* @__PURE__ */ __name((e) => exitFailCause(causeFail(e)), "exitFail");
  var exitDie = /* @__PURE__ */ __name((defect) => exitFailCause(causeDie(defect)), "exitDie");
  var withFiber = /* @__PURE__ */ makePrimitive({
    op: "WithFiber",
    [evaluate](fiber2) {
      return this[args](fiber2);
    }
  });
  var withFiberSucceed = /* @__PURE__ */ makePrimitive({
    op: "WithFiberSucceed",
    [evaluate](fiber2) {
      const value = this[args](fiber2);
      const cont = fiber2.getCont(contA);
      return cont ? cont[contA](value, fiber2) : fiber2.yieldWith(exitSucceed(value));
    }
  });
  var YieldableError = /* @__PURE__ */ (function() {
    class YieldableError2 extends globalThis.Error {
      static {
        __name(this, "YieldableError");
      }
    }
    const proto = /* @__PURE__ */ makePrimitiveProto({
      op: "YieldableError",
      [evaluate]() {
        return exitFail(this);
      }
    });
    delete proto.toString;
    Object.assign(YieldableError2.prototype, proto);
    return YieldableError2;
  })();
  var Error2 = /* @__PURE__ */ (function() {
    const plainArgsSymbol = /* @__PURE__ */ Symbol.for("effect/Data/Error/plainArgs");
    return class Base extends YieldableError {
      static {
        __name(this, "Base");
      }
      constructor(args2) {
        super(args2?.message, args2?.cause ? {
          cause: args2.cause
        } : void 0);
        if (args2) {
          assignProperties(this, args2);
          Object.defineProperty(this, plainArgsSymbol, {
            value: args2,
            enumerable: false
          });
        }
      }
      toJSON() {
        return {
          ...this[plainArgsSymbol],
          ...this
        };
      }
    };
  })();
  var TaggedError = /* @__PURE__ */ __name((tag) => {
    class Base extends Error2 {
      static {
        __name(this, "Base");
      }
      _tag = tag;
    }
    ;
    Base.prototype.name = tag;
    return Base;
  }, "TaggedError");
  var NoSuchElementErrorTypeId = "~effect/Cause/NoSuchElementError";
  var NoSuchElementError = class extends (/* @__PURE__ */ TaggedError("NoSuchElementError")) {
    static {
      __name(this, "NoSuchElementError");
    }
    [NoSuchElementErrorTypeId] = NoSuchElementErrorTypeId;
    constructor(message) {
      super({
        message
      });
    }
  };
  var DoneTypeId = "~effect/Cause/Done";
  var DoneVoid = {
    [DoneTypeId]: DoneTypeId,
    _tag: "Done",
    value: void 0
  };

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Effectable.js
  var Prototype = /* @__PURE__ */ __name((options) => makePrimitiveProto({
    op: options.label,
    [evaluate]: options.evaluate
  }), "Prototype");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/result.js
  var TypeId = "~effect/Result";
  var CommonProto = {
    [TypeId]: {
      /* v8 ignore next 2 */
      _A: /* @__PURE__ */ __name((_) => _, "_A"),
      _E: /* @__PURE__ */ __name((_) => _, "_E")
    },
    ...PipeInspectableProto,
    [Symbol.iterator]() {
      return new SingleShotGen(this);
    }
  };
  var SuccessProto = /* @__PURE__ */ Object.assign(/* @__PURE__ */ Object.create(CommonProto), {
    _tag: "Success",
    _op: "Success",
    [symbol2](that) {
      return isResult(that) && isSuccess(that) && equals(this.success, that.success);
    },
    [symbol]() {
      return combine(hash(this._tag))(hash(this.success));
    },
    toString() {
      return `success(${format2(this.success)})`;
    },
    toJSON() {
      return {
        _id: "Result",
        _tag: this._tag,
        value: toJson(this.success)
      };
    }
  });
  var FailureProto = /* @__PURE__ */ Object.assign(/* @__PURE__ */ Object.create(CommonProto), {
    _tag: "Failure",
    _op: "Failure",
    [symbol2](that) {
      return isResult(that) && isFailure(that) && equals(this.failure, that.failure);
    },
    [symbol]() {
      return combine(hash(this._tag))(hash(this.failure));
    },
    toString() {
      return `failure(${format2(this.failure)})`;
    },
    toJSON() {
      return {
        _id: "Result",
        _tag: this._tag,
        failure: toJson(this.failure)
      };
    }
  });
  var isResult = /* @__PURE__ */ __name((input) => hasProperty(input, TypeId), "isResult");
  var isFailure = /* @__PURE__ */ __name((result2) => result2._tag === "Failure", "isFailure");
  var isSuccess = /* @__PURE__ */ __name((result2) => result2._tag === "Success", "isSuccess");
  var FailureImpl = /* @__PURE__ */ __name(function(failure) {
    this.failure = failure;
  }, "FailureImpl");
  FailureImpl.prototype = FailureProto;
  var fail = /* @__PURE__ */ __name((failure) => new FailureImpl(failure), "fail");
  var SuccessImpl = /* @__PURE__ */ __name(function(success) {
    this.success = success;
  }, "SuccessImpl");
  SuccessImpl.prototype = SuccessProto;
  var succeed = /* @__PURE__ */ __name((success) => new SuccessImpl(success), "succeed");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Context.js
  var ServiceTypeId = "~effect/Context/Service";
  var Service = /* @__PURE__ */ __name(function() {
    function KeyClass() {
    }
    __name(KeyClass, "KeyClass");
    const self = KeyClass;
    Object.setPrototypeOf(self, ServiceProto);
    const init = /* @__PURE__ */ __name((key, options) => {
      self.key = key;
      if (options?.defaultValue) {
        self[ReferenceTypeId] = ReferenceTypeId;
        self.defaultValue = options.defaultValue;
      }
      if (options?.make) {
        ;
        self.make = options.make;
      }
      if (options?.fiberCached) {
        cacheKeys.add(key);
      }
      return self;
    }, "init");
    return arguments.length > 0 ? init(arguments[0], arguments[1]) : init;
  }, "Service");
  var ServiceProto = {
    [ServiceTypeId]: ServiceTypeId,
    .../* @__PURE__ */ Prototype({
      label: "Service",
      evaluate(fiber2) {
        return exitSucceed(get(fiber2.context, this));
      }
    }),
    toJSON() {
      return {
        _id: "Service",
        key: this.key
      };
    },
    of(self) {
      return self;
    },
    context(self) {
      return make(this, self);
    },
    use(f) {
      return withFiber((fiber2) => f(get(fiber2.context, this)));
    },
    useSync(f) {
      return withFiber((fiber2) => exitSucceed(f(get(fiber2.context, this))));
    }
  };
  var cacheKeys = /* @__PURE__ */ new Set();
  var ReferenceTypeId = "~effect/Context/Reference";
  var TypeId2 = "~effect/Context";
  var MaxDepth = 8;
  var FlattenAfterBaseHits = 8;
  var makeImpl = /* @__PURE__ */ __name((cacheRoot, base, overlay, depth) => {
    const self = Object.create(Proto);
    self.cacheRoot = cacheRoot ?? self;
    self.base = base;
    self.overlay = overlay;
    self.depth = depth;
    self._flat = void 0;
    self.baseHits = 0;
    return self;
  }, "makeImpl");
  var applyOverlays = /* @__PURE__ */ __name((map2, overlay) => {
    if (!overlay) return;
    applyOverlays(map2, overlay.parent);
    map2.set(overlay.key, overlay.value);
  }, "applyOverlays");
  var flatten = /* @__PURE__ */ __name((self) => {
    if (self._flat) return self._flat;
    if (!self.overlay) return self._flat = self.base;
    const map2 = new Map(self.base);
    applyOverlays(map2, self.overlay);
    return self._flat = map2;
  }, "flatten");
  var notFound = /* @__PURE__ */ Symbol();
  var lookup = /* @__PURE__ */ __name((self, key) => {
    const impl = self;
    for (let overlay = impl.overlay; overlay; overlay = overlay.parent) {
      if (overlay.key === key) return overlay.value;
    }
    const value = impl.base.get(key);
    if (value === void 0 && !impl.base.has(key)) return notFound;
    if (impl.overlay && ++impl.baseHits >= impl.base.size && impl.baseHits >= FlattenAfterBaseHits) {
      impl.base = flatten(impl);
      impl.overlay = void 0;
      impl.depth = 0;
    }
    return value;
  }, "lookup");
  var makeUnsafe = /* @__PURE__ */ __name((mapUnsafe) => makeImpl(void 0, mapUnsafe, void 0, 0), "makeUnsafe");
  var Proto = {
    get mapUnsafe() {
      return flatten(this);
    },
    ...PipeInspectableProto,
    [TypeId2]: {
      _Services: /* @__PURE__ */ __name((_) => _, "_Services")
    },
    toJSON() {
      return {
        _id: "Context",
        services: Array.from(this.mapUnsafe).map(([key, value]) => ({
          key,
          value
        }))
      };
    },
    [symbol2](that) {
      if (!isContext(that)) return false;
      const self = this.mapUnsafe;
      const other = that.mapUnsafe;
      if (self.size !== other.size) return false;
      for (const [key, value] of self) {
        if (!other.has(key) || !equals(value, other.get(key))) return false;
      }
      return true;
    },
    [symbol]() {
      return number(this.mapUnsafe.size);
    }
  };
  var hasSameCache = /* @__PURE__ */ __name((self, that) => self.cacheRoot === that.cacheRoot, "hasSameCache");
  var isContext = /* @__PURE__ */ __name((u) => hasProperty(u, TypeId2), "isContext");
  var isReference = /* @__PURE__ */ __name((u) => !!u[ReferenceTypeId], "isReference");
  var empty = /* @__PURE__ */ __name(() => emptyContext2, "empty");
  var emptyContext2 = /* @__PURE__ */ makeUnsafe(/* @__PURE__ */ new Map());
  var make = /* @__PURE__ */ __name((key, service2) => makeUnsafe(/* @__PURE__ */ new Map([[key.key, service2]])), "make");
  var add = /* @__PURE__ */ dual(3, (self, key, service2) => addUnsafe(self, key.key, service2));
  var addUnsafe = /* @__PURE__ */ __name((self, key, service2) => {
    const impl = self;
    const cacheRoot = cacheKeys.has(key) ? void 0 : impl.cacheRoot;
    if (impl.depth >= MaxDepth) {
      const map2 = new Map(impl._flat ?? impl.base);
      if (!impl._flat) applyOverlays(map2, impl.overlay);
      map2.set(key, service2);
      return makeImpl(cacheRoot, map2, void 0, 0);
    }
    return makeImpl(cacheRoot, impl.base, {
      key,
      value: service2,
      parent: impl.overlay
    }, impl.depth + 1);
  }, "addUnsafe");
  var getOrUndefinedUnsafe = /* @__PURE__ */ __name((self, key) => {
    const value = lookup(self, key);
    return value === notFound ? void 0 : value;
  }, "getOrUndefinedUnsafe");
  var getUnsafe = /* @__PURE__ */ dual(2, (self, service2) => {
    const value = lookup(self, service2.key);
    if (value === notFound) {
      if (isReference(service2)) return getDefaultValue(service2);
      throw serviceNotFoundError(service2);
    }
    return value;
  });
  var get = getUnsafe;
  var defaultValueCacheKey = "~effect/Context/defaultValue";
  var getDefaultValue = /* @__PURE__ */ __name((ref) => {
    if (defaultValueCacheKey in ref) {
      return ref[defaultValueCacheKey];
    }
    return ref[defaultValueCacheKey] = ref.defaultValue();
  }, "getDefaultValue");
  var serviceNotFoundError = /* @__PURE__ */ __name((service2) => {
    const error = new Error(`Service not found${service2.key ? `: ${String(service2.key)}` : ""}`);
    if (error.stack) {
      const lines = error.stack.split("\n");
      lines.splice(1, 3);
      error.stack = lines.join("\n");
    }
    return error;
  }, "serviceNotFoundError");
  var Reference = Service;

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Duration.js
  var TypeId3 = "~effect/Duration";
  var bigint0 = /* @__PURE__ */ BigInt(0);
  var bigint1 = /* @__PURE__ */ BigInt(1);
  var bigint2 = /* @__PURE__ */ BigInt(2);
  var bigint10 = /* @__PURE__ */ BigInt(10);
  var bigint1e3 = /* @__PURE__ */ BigInt(1e3);
  var roundTiesAwayFromZero = /* @__PURE__ */ __name((input) => BigInt(Math.sign(input) * Math.round(Math.abs(input))), "roundTiesAwayFromZero");
  var roundMillisToNanos = /* @__PURE__ */ __name((millis2) => roundTiesAwayFromZero(millis2 * 1e6), "roundMillisToNanos");
  var fromNanosNumber = /* @__PURE__ */ __name((nanos2) => make2(Number.isFinite(nanos2) ? roundTiesAwayFromZero(nanos2) : nanos2), "fromNanosNumber");
  var parseNanos = /* @__PURE__ */ __name((input, scale) => {
    const decimalIndex = input.indexOf(".");
    if (decimalIndex === -1) return BigInt(input) * scale;
    const isNegative = input[0] === "-";
    const fractional = input.slice(decimalIndex + 1);
    const fractionalScale = bigint10 ** BigInt(fractional.length);
    const scaled = (BigInt(input.slice(isNegative ? 1 : 0, decimalIndex)) * fractionalScale + BigInt(fractional)) * scale;
    return divideTiesAwayFromZero(isNegative ? -scaled : scaled, fractionalScale);
  }, "parseNanos");
  var divideTiesAwayFromZero = /* @__PURE__ */ __name((numerator, denominator) => {
    const absolute = numerator < bigint0 ? -numerator : numerator;
    const rounded = absolute / denominator + (absolute % denominator * bigint2 >= denominator ? bigint1 : bigint0);
    return numerator < bigint0 ? -rounded : rounded;
  }, "divideTiesAwayFromZero");
  var DURATION_REGEXP = /^(-?\d+(?:\.\d+)?)\s+(nanos?|micros?|millis?|seconds?|minutes?|hours?|days?|weeks?)$/;
  var fromInputUnsafe = /* @__PURE__ */ __name((input) => {
    switch (typeof input) {
      case "number":
        return millis(input);
      case "bigint":
        return nanos(input);
      case "string": {
        if (input === "Infinity") {
          return infinity;
        }
        if (input === "-Infinity") {
          return negativeInfinity;
        }
        const match4 = DURATION_REGEXP.exec(input);
        if (!match4) break;
        const [_, valueStr, unit] = match4;
        if (unit === "nano" || unit === "nanos") {
          return nanos(parseNanos(valueStr, bigint1));
        }
        if (unit === "micro" || unit === "micros") {
          return nanos(parseNanos(valueStr, bigint1e3));
        }
        const value = Number(valueStr);
        switch (unit) {
          case "milli":
          case "millis":
            return millis(value);
          case "second":
          case "seconds":
            return seconds(value);
          case "minute":
          case "minutes":
            return minutes(value);
          case "hour":
          case "hours":
            return hours(value);
          case "day":
          case "days":
            return days(value);
          case "week":
          case "weeks":
            return weeks(value);
        }
        break;
      }
      case "object": {
        if (input === null) break;
        if (TypeId3 in input) return input;
        if (Array.isArray(input)) {
          if (input.length !== 2 || !input.every(isNumber)) {
            return invalid(input);
          }
          if (Number.isNaN(input[0]) || Number.isNaN(input[1])) {
            return zero;
          }
          if (input[0] === -Infinity || input[1] === -Infinity) {
            return negativeInfinity;
          }
          if (input[0] === Infinity || input[1] === Infinity) {
            return infinity;
          }
          return fromNanosNumber(input[0] * 1e9 + input[1]);
        }
        const obj = input;
        let millis2 = 0;
        if (obj.weeks) millis2 += obj.weeks * 6048e5;
        if (obj.days) millis2 += obj.days * 864e5;
        if (obj.hours) millis2 += obj.hours * 36e5;
        if (obj.minutes) millis2 += obj.minutes * 6e4;
        if (obj.seconds) millis2 += obj.seconds * 1e3;
        if (obj.milliseconds) millis2 += obj.milliseconds;
        if (!obj.microseconds && !obj.nanoseconds) return make2(millis2);
        return fromNanosNumber(millis2 * 1e6 + (obj.microseconds ?? 0) * 1e3 + (obj.nanoseconds ?? 0));
      }
    }
    return invalid(input);
  }, "fromInputUnsafe");
  var invalid = /* @__PURE__ */ __name((input) => {
    throw new Error(`Invalid Input: ${input}`);
  }, "invalid");
  var zeroDurationValue = {
    _tag: "Millis",
    millis: 0
  };
  var infinityDurationValue = {
    _tag: "Infinity"
  };
  var negativeInfinityDurationValue = {
    _tag: "NegativeInfinity"
  };
  var DurationProto = {
    [TypeId3]: TypeId3,
    [symbol]() {
      switch (this.value._tag) {
        case "Millis": {
          const nanos2 = this.value.millis * 1e6;
          return Number.isFinite(nanos2) ? hash(roundTiesAwayFromZero(nanos2)) : number(this.value.millis);
        }
        case "Nanos":
          return hash(this.value.nanos);
        default:
          return structure(this.value);
      }
    },
    [symbol2](that) {
      return isDuration(that) && equals2(this, that);
    },
    toString() {
      switch (this.value._tag) {
        case "Infinity":
          return "Infinity";
        case "NegativeInfinity":
          return "-Infinity";
        case "Nanos":
          return `${this.value.nanos} nanos`;
        case "Millis":
          return `${this.value.millis} millis`;
      }
    },
    toJSON() {
      switch (this.value._tag) {
        case "Millis":
          return {
            _id: "Duration",
            _tag: "Millis",
            millis: this.value.millis
          };
        case "Nanos":
          return {
            _id: "Duration",
            _tag: "Nanos",
            nanos: String(this.value.nanos)
          };
        case "Infinity":
          return {
            _id: "Duration",
            _tag: "Infinity"
          };
        case "NegativeInfinity":
          return {
            _id: "Duration",
            _tag: "NegativeInfinity"
          };
      }
    },
    [NodeInspectSymbol]() {
      return this.toJSON();
    },
    pipe() {
      return pipeArguments(this, arguments);
    }
  };
  var make2 = /* @__PURE__ */ __name((input) => {
    const duration = Object.create(DurationProto);
    if (typeof input === "number") {
      if (isNaN(input) || input === 0 || Object.is(input, -0)) {
        duration.value = zeroDurationValue;
      } else if (!Number.isFinite(input)) {
        duration.value = input > 0 ? infinityDurationValue : negativeInfinityDurationValue;
      } else if (!Number.isInteger(input)) {
        duration.value = {
          _tag: "Nanos",
          nanos: roundMillisToNanos(input)
        };
      } else {
        duration.value = {
          _tag: "Millis",
          millis: input
        };
      }
    } else if (input === bigint0) {
      duration.value = zeroDurationValue;
    } else {
      duration.value = {
        _tag: "Nanos",
        nanos: input
      };
    }
    return duration;
  }, "make");
  var isDuration = /* @__PURE__ */ __name((u) => hasProperty(u, TypeId3), "isDuration");
  var zero = /* @__PURE__ */ make2(0);
  var infinity = /* @__PURE__ */ make2(Infinity);
  var negativeInfinity = /* @__PURE__ */ make2(-Infinity);
  var nanos = /* @__PURE__ */ __name((nanos2) => make2(nanos2), "nanos");
  var millis = /* @__PURE__ */ __name((millis2) => make2(millis2), "millis");
  var seconds = /* @__PURE__ */ __name((seconds2) => make2(seconds2 * 1e3), "seconds");
  var minutes = /* @__PURE__ */ __name((minutes2) => make2(minutes2 * 6e4), "minutes");
  var hours = /* @__PURE__ */ __name((hours2) => make2(hours2 * 36e5), "hours");
  var days = /* @__PURE__ */ __name((days2) => make2(days2 * 864e5), "days");
  var weeks = /* @__PURE__ */ __name((weeks2) => make2(weeks2 * 6048e5), "weeks");
  var toMillis = /* @__PURE__ */ __name((self) => match(fromInputUnsafe(self), {
    onMillis: identity,
    onNanos: /* @__PURE__ */ __name((nanos2) => Number(nanos2) / 1e6, "onNanos"),
    onInfinity: /* @__PURE__ */ __name(() => Infinity, "onInfinity"),
    onNegativeInfinity: /* @__PURE__ */ __name(() => -Infinity, "onNegativeInfinity")
  }), "toMillis");
  var toNanosUnsafe = /* @__PURE__ */ __name((input) => {
    const self = fromInputUnsafe(input);
    switch (self.value._tag) {
      case "Infinity":
      case "NegativeInfinity":
        throw new Error("Cannot convert infinite duration to nanos");
      case "Nanos":
        return self.value.nanos;
      case "Millis":
        return roundMillisToNanos(self.value.millis);
    }
  }, "toNanosUnsafe");
  var match = /* @__PURE__ */ dual(2, (self, options) => {
    switch (self.value._tag) {
      case "Millis":
        return options.onMillis(self.value.millis);
      case "Nanos":
        return options.onNanos(self.value.nanos);
      case "Infinity":
        return options.onInfinity();
      case "NegativeInfinity":
        return (options.onNegativeInfinity ?? options.onInfinity)();
    }
  });
  var matchPair = /* @__PURE__ */ dual(3, (self, that, options) => {
    if (self.value._tag === "Infinity" || self.value._tag === "NegativeInfinity" || that.value._tag === "Infinity" || that.value._tag === "NegativeInfinity") return options.onInfinity(self, that);
    if (self.value._tag === "Millis") {
      return that.value._tag === "Millis" ? options.onMillis(self.value.millis, that.value.millis) : options.onNanos(toNanosUnsafe(self), that.value.nanos);
    } else {
      return options.onNanos(self.value.nanos, toNanosUnsafe(that));
    }
  });
  var Equivalence = /* @__PURE__ */ __name((self, that) => matchPair(self, that, {
    onMillis: /* @__PURE__ */ __name((self2, that2) => self2 === that2, "onMillis"),
    onNanos: /* @__PURE__ */ __name((self2, that2) => self2 === that2, "onNanos"),
    onInfinity: /* @__PURE__ */ __name((self2, that2) => self2.value._tag === that2.value._tag, "onInfinity")
  }), "Equivalence");
  var equals2 = /* @__PURE__ */ dual(2, (self, that) => Equivalence(self, that));

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Result.js
  var succeed2 = succeed;
  var fail2 = fail;
  var isFailure2 = isFailure;

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Array.js
  var Array2 = globalThis.Array;
  var fromIterable = /* @__PURE__ */ __name((collection) => Array2.isArray(collection) ? collection : Array2.from(collection), "fromIterable");
  var isArray = Array2.isArray;
  var empty2 = /* @__PURE__ */ __name(() => [], "empty");

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Scheduler.js
  var Scheduler = /* @__PURE__ */ Reference("effect/Scheduler", {
    fiberCached: true,
    defaultValue: /* @__PURE__ */ __name(() => new MixedScheduler(), "defaultValue")
  });
  var setMicrotask = /* @__PURE__ */ __name((f) => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (!cancelled) f();
    });
    return () => {
      cancelled = true;
    };
  }, "setMicrotask");
  var setTimer = "setImmediate" in globalThis ? (f) => {
    const timer = globalThis.setImmediate(f);
    return () => globalThis.clearImmediate(timer);
  } : (f) => {
    const timer = setTimeout(f, 0);
    return () => clearTimeout(timer);
  };
  var setImmediate = /* @__PURE__ */ __name((f) => {
    try {
      return setTimer(f);
    } catch {
      return setMicrotask(f);
    }
  }, "setImmediate");
  var PriorityBuckets = class {
    static {
      __name(this, "PriorityBuckets");
    }
    buckets = [];
    scheduleTask(task, priority) {
      const buckets = this.buckets;
      const len = buckets.length;
      let bucket;
      let index = 0;
      for (; index < len; index++) {
        if (buckets[index][0] > priority) break;
        bucket = buckets[index];
      }
      if (bucket && bucket[0] === priority) {
        bucket[1].push(task);
      } else if (index === len) {
        buckets.push([priority, [task]]);
      } else {
        buckets.splice(index, 0, [priority, [task]]);
      }
    }
    drain() {
      const buckets = this.buckets;
      this.buckets = [];
      return buckets;
    }
  };
  var MixedScheduler = class {
    static {
      __name(this, "MixedScheduler");
    }
    executionMode;
    setImmediate;
    constructor(executionMode = "async", setImmediateFn) {
      this.executionMode = executionMode;
      this.setImmediate = setImmediateFn ?? (executionMode === "sync" ? setMicrotask : setImmediate);
    }
    /**
     * Returns whether the fiber has reached its operation budget and should yield.
     *
     * **When to use**
     *
     * Use to decide whether a fiber should yield after consuming its current
     * operation budget.
     *
     * @since 2.0.0
     */
    shouldYield(fiber2) {
      return fiber2.currentOpCount >= fiber2.cache.maxOpsBeforeYield;
    }
    /**
     * Creates a dispatcher that schedules work through this scheduler.
     *
     * **When to use**
     *
     * Use when you need a standalone dispatcher from a scheduler instance, for
     * example in tests that enqueue tasks and then flush them deterministically.
     *
     * @since 4.0.0
     */
    makeDispatcher() {
      return new MixedSchedulerDispatcher(this.setImmediate);
    }
  };
  var MixedSchedulerDispatcher = class {
    static {
      __name(this, "MixedSchedulerDispatcher");
    }
    tasks = /* @__PURE__ */ new PriorityBuckets();
    running = void 0;
    setImmediate;
    constructor(setImmediateFn = setImmediate) {
      this.setImmediate = setImmediateFn;
    }
    /**
     * @since 2.0.0
     */
    scheduleTask(task, priority) {
      this.tasks.scheduleTask(task, priority);
      if (this.running === void 0) {
        this.running = this.setImmediate(this.afterScheduled);
      }
    }
    /**
     * @since 2.0.0
     */
    afterScheduled = /* @__PURE__ */ __name(() => {
      this.running = void 0;
      this.runTasks();
    }, "afterScheduled");
    /**
     * @since 2.0.0
     */
    runTasks() {
      const buckets = this.tasks.drain();
      for (let i = 0; i < buckets.length; i++) {
        const toRun = buckets[i][1];
        for (let j = 0; j < toRun.length; j++) {
          toRun[j]();
        }
      }
    }
    /**
     * @since 2.0.0
     */
    flush() {
      while (this.tasks.buckets.length > 0) {
        if (this.running !== void 0) {
          this.running();
          this.running = void 0;
        }
        this.runTasks();
      }
    }
  };
  var MaxOpsBeforeYield = /* @__PURE__ */ Reference("effect/Scheduler/MaxOpsBeforeYield", {
    fiberCached: true,
    defaultValue: /* @__PURE__ */ __name(() => 2048, "defaultValue")
  });
  var PreventSchedulerYield = /* @__PURE__ */ Reference("effect/Scheduler/PreventSchedulerYield", {
    fiberCached: true,
    defaultValue: /* @__PURE__ */ __name(() => false, "defaultValue")
  });

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Tracer.js
  var ParentSpanKey = "effect/Tracer/ParentSpan";
  var TracerKey = "effect/Tracer";

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/metric.js
  var FiberRuntimeMetricsKey = "effect/Metric/FiberRuntimeMetrics";

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/references.js
  var CurrentStackFrame = /* @__PURE__ */ Reference("effect/References/CurrentStackFrame", {
    fiberCached: true,
    defaultValue: constUndefined
  });
  var TracerEnabled = /* @__PURE__ */ Reference("effect/References/TracerEnabled", {
    fiberCached: true,
    defaultValue: constTrue
  });
  var CurrentLogLevel = /* @__PURE__ */ Reference("effect/References/CurrentLogLevel", {
    fiberCached: true,
    defaultValue: /* @__PURE__ */ __name(() => "Info", "defaultValue")
  });
  var MinimumLogLevel = /* @__PURE__ */ Reference("effect/References/MinimumLogLevel", {
    fiberCached: true,
    defaultValue: /* @__PURE__ */ __name(() => "Info", "defaultValue")
  });

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/internal/effect.js
  var Interrupt = class extends ReasonBase {
    static {
      __name(this, "Interrupt");
    }
    constructor(fiberId2, annotations = constEmptyAnnotations) {
      super("Interrupt", annotations, "Interrupted");
      this.fiberId = fiberId2;
    }
    toString() {
      return `Interrupt(${this.fiberId})`;
    }
    toJSON() {
      return {
        _tag: "Interrupt",
        fiberId: this.fiberId
      };
    }
    [symbol2](that) {
      return isInterruptReason(that) && this.fiberId === that.fiberId && this.annotations === that.annotations;
    }
    [symbol]() {
      return combine(string(`${this._tag}:${this.fiberId}`))(random(this.annotations));
    }
  };
  var causeInterrupt = /* @__PURE__ */ __name((fiberId2) => new CauseImpl([new Interrupt(fiberId2)]), "causeInterrupt");
  var findError = /* @__PURE__ */ __name((self) => {
    for (let i = 0; i < self.reasons.length; i++) {
      const reason = self.reasons[i];
      if (reason._tag === "Fail") {
        return succeed2(reason.error);
      }
    }
    return fail2(self);
  }, "findError");
  var hasInterrupts = /* @__PURE__ */ __name((self) => self.reasons.some(isInterruptReason), "hasInterrupts");
  var causeSquash = /* @__PURE__ */ __name((self) => {
    let die2;
    for (const reason of self.reasons) {
      if (reason._tag === "Fail") return reason.error;
      if (reason._tag === "Die") die2 ??= reason;
    }
    if (die2 !== void 0) return die2.defect;
    if (self.reasons.length > 0) return new globalThis.Error("All fibers interrupted without error");
    return new globalThis.Error("Empty cause");
  }, "causeSquash");
  var FiberTypeId = "~effect/Fiber";
  var fiberVariance = {
    _A: identity,
    _E: identity
  };
  var fiberIdStore = {
    id: 0
  };
  var AsyncResource = /* @__PURE__ */ (() => {
    try {
      return globalThis.process?.getBuiltinModule?.("node:async_hooks")?.AsyncResource;
    } catch {
      return void 0;
    }
  })();
  var captureAsyncContext = /* @__PURE__ */ __name(() => AsyncResource === void 0 ? void 0 : new AsyncResource("effect/Fiber"), "captureAsyncContext");
  var getCurrentFiber = /* @__PURE__ */ __name(() => globalThis[currentFiberTypeId], "getCurrentFiber");
  var FiberImpl = class {
    static {
      __name(this, "FiberImpl");
    }
    constructor(context2, interruptible2 = true) {
      this.setContext(context2);
      this.id = ++fiberIdStore.id;
      this.currentOpCount = 0;
      this.interruptible = interruptible2;
      this._stack = [];
      this._observers = void 0;
      this._exit = void 0;
      this._children = void 0;
      this._interruptedCause = void 0;
      this._yielded = void 0;
      this._running = false;
      this._deferredInterrupt = false;
      this._parent = void 0;
      this._asyncContext = void 0;
      this.cache.runtimeMetrics?.recordFiberStart(this.context);
    }
    get [FiberTypeId]() {
      return fiberVariance;
    }
    get currentDispatcher() {
      return this._dispatcher ??= this.cache.scheduler.makeDispatcher();
    }
    getRef(ref) {
      return get(this.context, ref);
    }
    addObserver(cb) {
      if (this._exit) {
        cb(this._exit);
        return constVoid;
      }
      if (this._observers === void 0) {
        this._observers = [cb];
      } else {
        this._observers.push(cb);
      }
      return () => this.removeObserver(cb);
    }
    removeObserver(cb) {
      if (this._exit || this._observers === void 0) return;
      const index = this._observers.indexOf(cb);
      if (index >= 0) {
        this._observers.splice(index, 1);
      }
    }
    interruptUnsafe(fiberId2, annotations) {
      if (this._exit) {
        return;
      }
      let cause = causeInterrupt(fiberId2);
      if (this.cache.stackFrame) {
        cause = causeAnnotate(cause, make(StackTraceKey, this.cache.stackFrame));
      }
      if (annotations) {
        cause = causeAnnotate(cause, annotations);
      }
      this._interruptedCause = this._interruptedCause ? causeCombine(this._interruptedCause, cause) : cause;
      if (this.interruptible) {
        if (this._running) {
          this._deferredInterrupt = true;
        } else {
          this.evaluate(failCause(this._interruptedCause));
        }
      }
    }
    pollUnsafe() {
      return this._exit;
    }
    evaluate(effect) {
      if (this._exit) {
        return;
      } else if (this._asyncContext !== void 0) {
        const asyncContext = this._asyncContext;
        this._asyncContext = void 0;
        return asyncContext.runInAsyncScope(this.evaluate, this, effect);
      } else if (this._yielded !== void 0) {
        const yielded = this._yielded;
        this._yielded = void 0;
        yielded();
      }
      const exit2 = this.runLoop(effect);
      if (exit2 === Yield) {
        this._asyncContext = captureAsyncContext();
        return;
      }
      const interruptChildren = fiberMiddleware.interruptChildren && fiberMiddleware.interruptChildren(this);
      if (interruptChildren !== void 0) {
        return this.evaluate(flatMap(interruptChildren, () => exit2));
      }
      this._exit = exit2;
      this.cache.runtimeMetrics?.recordFiberEnd(this.context, this._exit);
      if (this._parent) {
        this._parent._children?.delete(this);
        this._parent = void 0;
      }
      if (this._observers !== void 0) {
        const observers = this._observers;
        this._observers = void 0;
        for (let i = 0; i < observers.length; i++) {
          observers[i](exit2);
        }
      }
      this._stack.length = 0;
      this._children = void 0;
      this.context = empty();
    }
    runLoop(effect) {
      const prevFiber = globalThis[currentFiberTypeId];
      globalThis[currentFiberTypeId] = this;
      const prevRunning = this._running;
      this._running = true;
      let yielding = false;
      let current = effect;
      this.currentOpCount = 0;
      try {
        while (true) {
          try {
            if (this._deferredInterrupt) {
              this._deferredInterrupt = false;
              current = failCause(this._interruptedCause);
            }
            this.currentOpCount++;
            const cache = this.cache;
            if (!yielding && !cache.preventYield && cache.scheduler.shouldYield(this)) {
              yielding = true;
              const prev = current;
              current = flatMap(yieldNow, () => prev);
            }
            current = cache.tracerContext ? cache.tracerContext(current, this) : current[evaluate](this);
            if (current === Yield) {
              const yielded = this._yielded;
              if (ExitTypeId in yielded) {
                this._deferredInterrupt = false;
                this._yielded = void 0;
                return yielded;
              } else if (this._deferredInterrupt) {
                this._yielded = void 0;
                yielded();
                continue;
              }
              return Yield;
            }
          } catch (error) {
            if (!hasProperty(current, evaluate)) {
              return exitDie(`Fiber.runLoop: Not a valid effect: ${String(current)}`);
            }
            current = exitDie(error);
          }
        }
      } finally {
        this._running = prevRunning;
        globalThis[currentFiberTypeId] = prevFiber;
      }
    }
    getCont(symbol3) {
      if (this._deferredInterrupt) {
        this._deferredInterrupt = false;
        return deferredInterruptCont;
      }
      while (true) {
        const op = this._stack.pop();
        if (!op) return void 0;
        const all3 = op[contAll];
        if (all3 !== void 0) {
          const cont = all3.call(op, this);
          if (cont) {
            ;
            cont[symbol3] = cont;
            return cont;
          }
        }
        if (op[symbol3]) return op;
      }
    }
    // Passes a success value straight to the next continuation instead of
    // returning an Exit for the run loop to unwrap. Each call counts as an
    // operation, and every `maxInlineSteps`th call returns an Exit instead, so
    // continuations calling continuations never nest deeper than that.
    succeedWith(value) {
      if ((++this.currentOpCount & maxInlineSteps - 1) === 0) {
        return exitSucceed(value);
      }
      const cont = this.getCont(contA);
      return cont ? cont[contA](value, this) : this.yieldWith(exitSucceed(value));
    }
    yieldWith(value) {
      this._yielded = value;
      return Yield;
    }
    children() {
      return this._children ??= /* @__PURE__ */ new Set();
    }
    pipe() {
      return pipeArguments(this, arguments);
    }
    setContext(context2) {
      const previous = this.context;
      this.context = context2;
      if (previous !== void 0 && hasSameCache(previous, context2)) return;
      const root = context2.cacheRoot;
      const cache = root._fiberCache ??= makeFiberContextCache(context2);
      if (this.cache?.scheduler !== cache.scheduler) {
        this._dispatcher = void 0;
      }
      this.cache = cache;
    }
    get currentSpanLocal() {
      const span = this.cache.span;
      return span?._tag === "Span" ? span : void 0;
    }
  };
  var makeFiberContextCache = /* @__PURE__ */ __name((context2) => {
    const currentTracer = getOrUndefinedUnsafe(context2, TracerKey);
    return {
      scheduler: get(context2, Scheduler),
      tracer: currentTracer,
      tracerContext: currentTracer ? currentTracer["context"] : void 0,
      tracerEnabled: get(context2, TracerEnabled),
      span: getOrUndefinedUnsafe(context2, ParentSpanKey),
      logLevel: get(context2, CurrentLogLevel),
      minimumLogLevel: get(context2, MinimumLogLevel),
      stackFrame: get(context2, CurrentStackFrame),
      runtimeMetrics: getOrUndefinedUnsafe(context2, FiberRuntimeMetricsKey),
      maxOpsBeforeYield: get(context2, MaxOpsBeforeYield),
      preventYield: get(context2, PreventSchedulerYield)
    };
  }, "makeFiberContextCache");
  var deferredInterruptCont = {
    [contA](_value, fiber2) {
      return failCause(fiber2._interruptedCause);
    },
    [contE](_cause, fiber2) {
      return failCause(fiber2._interruptedCause);
    }
  };
  var maxInlineSteps = 32;
  var fiberMiddleware = {
    interruptChildren: void 0
  };
  var fiberStackAnnotations = /* @__PURE__ */ __name((fiber2) => {
    if (!fiber2.cache.stackFrame) return void 0;
    const annotations = /* @__PURE__ */ new Map();
    annotations.set(InterruptorStackTrace.key, fiber2.cache.stackFrame);
    return makeUnsafe(annotations);
  }, "fiberStackAnnotations");
  var fiberAwaitAll = /* @__PURE__ */ __name((self) => callback((resume) => {
    const iter = self[Symbol.iterator]();
    const exits = [];
    let cancel = void 0;
    function loop() {
      let result2 = iter.next();
      while (!result2.done) {
        if (result2.value._exit) {
          exits.push(result2.value._exit);
          result2 = iter.next();
          continue;
        }
        cancel = result2.value.addObserver((exit2) => {
          exits.push(exit2);
          loop();
        });
        return;
      }
      resume(succeed3(exits));
    }
    __name(loop, "loop");
    loop();
    return sync(() => cancel?.());
  }), "fiberAwaitAll");
  var fiberInterruptAll = /* @__PURE__ */ __name((fibers) => withFiber((parent) => {
    const annotations = fiberStackAnnotations(parent);
    let fiberArr = empty2();
    for (const fiber2 of fibers) {
      fiber2.interruptUnsafe(parent.id, annotations);
      fiberArr.push(fiber2);
    }
    return asVoid(fiberAwaitAll(fiberArr));
  }), "fiberInterruptAll");
  var succeed3 = exitSucceed;
  var failCause = exitFailCause;
  var fail3 = exitFail;
  var sync = /* @__PURE__ */ makePrimitive({
    op: "Sync",
    [evaluate](fiber2) {
      const value = this[args]();
      const cont = fiber2.getCont(contA);
      return cont ? cont[contA](value, fiber2) : fiber2.yieldWith(exitSucceed(value));
    }
  });
  var suspend = /* @__PURE__ */ makePrimitive({
    op: "Suspend",
    [evaluate](_fiber) {
      return this[args]();
    }
  });
  var yieldNowWith = /* @__PURE__ */ makePrimitive({
    op: "Yield",
    [evaluate](fiber2) {
      let resumed = false;
      fiber2.currentDispatcher.scheduleTask(() => {
        if (resumed) return;
        fiber2.evaluate(exitVoid);
      }, this[args] ?? 0);
      return fiber2.yieldWith(() => {
        resumed = true;
      });
    }
  });
  var yieldNow = /* @__PURE__ */ yieldNowWith(0);
  var void_ = /* @__PURE__ */ succeed3(void 0);
  var try_ = /* @__PURE__ */ __name((options) => {
    const evaluate2 = typeof options === "function" ? options : options.try;
    const catcher = typeof options === "function" ? (cause) => new UnknownError(cause, "An error occurred in Effect.try") : options.catch;
    return suspend(() => {
      try {
        return succeed3(evaluate2());
      } catch (err) {
        return fail3(catcher(err));
      }
    });
  }, "try_");
  var callbackOptions = /* @__PURE__ */ (function() {
    const Proto2 = /* @__PURE__ */ makePrimitiveProto({
      op: "Async",
      [evaluate](fiber2) {
        let resumed = false;
        let yielded = false;
        const controller = this.withSignal ? new AbortController() : void 0;
        const onCancel = this.register.call(fiber2.cache.scheduler, (effect) => {
          if (resumed) return;
          resumed = true;
          if (yielded) {
            fiber2.evaluate(effect);
          } else {
            yielded = effect;
          }
        }, controller?.signal);
        if (yielded !== false) return yielded;
        yielded = true;
        fiber2._yielded = () => {
          resumed = true;
        };
        if (controller === void 0 && onCancel === void 0) {
          return Yield;
        }
        fiber2._stack.push(asyncFinalizer(() => {
          resumed = true;
          controller?.abort();
          return onCancel ?? exitVoid;
        }));
        return Yield;
      }
    });
    const AsyncImpl = /* @__PURE__ */ __name(function(register, withSignal) {
      this.register = register;
      this.withSignal = withSignal;
    }, "AsyncImpl");
    AsyncImpl.prototype = Proto2;
    return function(register, withSignal) {
      return new AsyncImpl(register, withSignal);
    };
  })();
  var asyncFinalizer = /* @__PURE__ */ makePrimitive({
    op: "AsyncFinalizer",
    [contAll](fiber2) {
      if (fiber2.interruptible) {
        fiber2.interruptible = false;
        fiber2._stack.push(setInterruptibleTrue);
      }
    },
    [contE](cause, _fiber) {
      return hasInterrupts(cause) ? flatMap(combineFinalizerCause(exitFailCause(cause), this[args]()), () => failCause(cause)) : failCause(cause);
    }
  });
  var callback = /* @__PURE__ */ __name((register) => callbackOptions(register, register.length >= 2), "callback");
  var never = /* @__PURE__ */ callback(constVoid);
  var gen = /* @__PURE__ */ __name((...args2) => {
    if (args2.length === 1) {
      const body2 = args2[0];
      return suspend(() => fromIteratorUnsafe(body2()));
    }
    const [options, body] = args2;
    return suspend(() => fromIteratorUnsafe(body.call(options.self)));
  }, "gen");
  var fromIteratorUnsafe = /* @__PURE__ */ (function() {
    const Proto2 = /* @__PURE__ */ makePrimitiveProto({
      op: "Iterator",
      [contA](value, fiber2) {
        const iter = this.iterator;
        while (true) {
          const state = iter.next(value);
          if (state.done) return succeed3(state.value);
          if (!effectIsExit(state.value)) {
            fiber2._stack.push(this);
            return state.value;
          } else if (state.value._tag === "Failure") {
            return state.value;
          }
          value = state.value.value;
        }
      },
      [evaluate](fiber2) {
        return this[contA](this.initial, fiber2);
      }
    });
    const IteratorImpl = /* @__PURE__ */ __name(function(iterator, initial) {
      this.iterator = iterator;
      this.initial = initial;
    }, "IteratorImpl");
    IteratorImpl.prototype = Proto2;
    return function(iterator, initial) {
      return new IteratorImpl(iterator, initial);
    };
  })();
  var as = /* @__PURE__ */ dual(2, (self, value) => new ContImpl(self, returnPayload, succeed3(value)));
  var evaluateCont = /* @__PURE__ */ __name(function(fiber2) {
    fiber2._stack.push(this);
    return this[args];
  }, "evaluateCont");
  var OnSuccessProto = /* @__PURE__ */ makePrimitiveProto({
    op: "OnSuccess",
    [evaluate]: evaluateCont
  });
  var ContImpl = /* @__PURE__ */ __name(function(self, cont, payload) {
    this[args] = self;
    this[contA] = cont;
    this.payload = payload;
  }, "ContImpl");
  ContImpl.prototype = OnSuccessProto;
  var returnPayload = /* @__PURE__ */ __name(function() {
    return this.payload;
  }, "returnPayload");
  var continuationMarksStack = /* @__PURE__ */ (() => {
    if (getStackTraceLimit() === 0) return false;
    const marker = "~effect/Effect/stackProbe";
    const probe = {
      [marker]: /* @__PURE__ */ __name(function stackProbe() {
        return new Error().stack;
      }, "stackProbe")
    };
    return probe[marker]()?.includes("[as " + marker + "]") === true;
  })();
  var mapCont = /* @__PURE__ */ __name(function(value, fiber2) {
    const f = this.payload;
    return fiber2.succeedWith(continuationMarksStack ? f(value) : internalCall(() => f(value)));
  }, "mapCont");
  var andThenCont = /* @__PURE__ */ __name(function(value) {
    const f = this.payload;
    return f(value);
  }, "andThenCont");
  var andThen = /* @__PURE__ */ dual(2, (self, f) => new ContImpl(self, isEffect(f) ? returnPayload : andThenCont, f));
  var asVoid = /* @__PURE__ */ __name((self) => new ContImpl(self, returnPayload, exitVoid), "asVoid");
  var flatMap = /* @__PURE__ */ dual(2, (self, f) => new ContImpl(self, andThenCont, f));
  var effectIsExit = /* @__PURE__ */ __name((effect) => effect[ExitTypeId] !== void 0, "effectIsExit");
  var map = /* @__PURE__ */ dual(2, (self, f) => new ContImpl(self, mapCont, f));
  var exitIsSuccess = /* @__PURE__ */ __name((self) => self._tag === "Success", "exitIsSuccess");
  var exitVoid = /* @__PURE__ */ exitSucceed(void 0);
  var forever = /* @__PURE__ */ dual((args2) => isEffect(args2[0]), (self, options) => whileLoop({
    while: constTrue,
    body: constant(options?.disableYield ? self : flatMap(self, (_) => yieldNow)),
    step: constVoid
  }));
  var catchCause = /* @__PURE__ */ dual(2, (self, f) => new OnFailureImpl(self, f.length !== 1 ? (cause) => f(cause) : f));
  var OnFailureProto = /* @__PURE__ */ makePrimitiveProto({
    op: "OnFailure",
    [evaluate]: evaluateCont
  });
  var OnFailureImpl = /* @__PURE__ */ __name(function(self, f) {
    this[args] = self;
    this[contE] = f;
  }, "OnFailureImpl");
  OnFailureImpl.prototype = OnFailureProto;
  var catchCauseFilter = /* @__PURE__ */ dual(3, (self, filter2, f) => catchCause(self, (cause) => {
    const eb = filter2(cause);
    return isFailure2(eb) ? failCause(eb.failure) : f(eb.success, cause);
  }));
  var catch_ = /* @__PURE__ */ dual(2, (self, f) => catchCauseFilter(self, findError, (e) => f(e)));
  var result = /* @__PURE__ */ __name((self) => matchEager(self, {
    onFailure: fail2,
    onSuccess: succeed2
  }), "result");
  var OnSuccessAndFailureProto = /* @__PURE__ */ makePrimitiveProto({
    op: "OnSuccessAndFailure",
    [evaluate]: evaluateCont
  });
  var OnSuccessAndFailureImpl = /* @__PURE__ */ __name(function(self, onSuccess, onFailure) {
    this[args] = self;
    this[contA] = onSuccess;
    this[contE] = onFailure;
  }, "OnSuccessAndFailureImpl");
  OnSuccessAndFailureImpl.prototype = OnSuccessAndFailureProto;
  var matchSuccess = /* @__PURE__ */ __name(function(value, fiber2) {
    const handlers = this.payload;
    return fiber2.succeedWith(continuationMarksStack ? handlers.onSuccess(value) : internalCall(() => handlers.onSuccess(value)));
  }, "matchSuccess");
  var makeMatch = /* @__PURE__ */ __name((op, onFailure) => {
    const Proto2 = makePrimitiveProto({
      op,
      [evaluate]: evaluateCont,
      [contA]: matchSuccess,
      [contE]: onFailure
    });
    const MatchImpl2 = /* @__PURE__ */ __name(function(self, handlers) {
      this[args] = self;
      this.payload = handlers;
    }, "MatchImpl");
    MatchImpl2.prototype = Proto2;
    return MatchImpl2;
  }, "makeMatch");
  var MatchImpl = /* @__PURE__ */ makeMatch("Match", function(cause, fiber2) {
    const fail4 = cause.reasons.find(isFailReason);
    if (fail4 === void 0) return failCause(cause);
    const handlers = this.payload;
    return fiber2.succeedWith(continuationMarksStack ? handlers.onFailure(fail4.error) : internalCall(() => handlers.onFailure(fail4.error)));
  });
  var match3 = /* @__PURE__ */ dual(2, (self, options) => new MatchImpl(self, options));
  var matchEager = /* @__PURE__ */ dual(2, (self, options) => {
    if (effectIsExit(self)) {
      if (self._tag === "Success") return exitSucceed(options.onSuccess(self.value));
      const error = findError(self.cause);
      if (isFailure2(error)) return self;
      return exitSucceed(options.onFailure(error.success));
    }
    return match3(self, options);
  });
  var exitPrimitive = /* @__PURE__ */ makePrimitive({
    op: "Exit",
    [evaluate](fiber2) {
      fiber2._stack.push(this);
      return this[args];
    },
    [contA](value, fiber2, exit2) {
      return fiber2.succeedWith(exit2 ?? exitSucceed(value));
    },
    [contE](cause, fiber2, exit2) {
      return fiber2.succeedWith(exit2 ?? exitFailCause(cause));
    }
  });
  var combineFinalizerCause = /* @__PURE__ */ __name((exit_, finalizer) => exitIsSuccess(exit_) ? finalizer : catchCause(finalizer, (cause) => failCause(causeCombine(exit_.cause, cause))), "combineFinalizerCause");
  var uninterruptible = /* @__PURE__ */ __name((self) => withFiber((fiber2) => {
    if (!fiber2.interruptible) return self;
    fiber2.interruptible = false;
    fiber2._stack.push(setInterruptibleTrue);
    return self;
  }), "uninterruptible");
  var setInterruptible = /* @__PURE__ */ makePrimitive({
    op: "SetInterruptible",
    [contAll](fiber2) {
      fiber2.interruptible = this[args];
      if (fiber2._interruptedCause && fiber2.interruptible) {
        return () => failCause(fiber2._interruptedCause);
      }
    }
  });
  var setInterruptibleTrue = /* @__PURE__ */ setInterruptible(true);
  var all = /* @__PURE__ */ __name((arg, options) => {
    if (isIterable(arg)) {
      return options?.mode === "result" ? forEach(arg, result, options) : forEach(arg, identity, options);
    } else if (options?.discard) {
      return options.mode === "result" ? forEach(Object.values(arg), result, options) : forEach(Object.values(arg), identity, options);
    }
    return suspend(() => {
      const out = {};
      return as(forEach(Object.entries(arg), ([key, effect]) => map(options?.mode === "result" ? result(effect) : effect, (value) => {
        assignProperty(out, key, value);
      }), {
        discard: true,
        concurrency: options?.concurrency
      }), out);
    });
  }, "all");
  var whileLoop = /* @__PURE__ */ makePrimitive({
    op: "While",
    [contA](value, fiber2) {
      this[args].step(value);
      if (this[args].while()) {
        fiber2._stack.push(this);
        return this[args].body();
      }
      return exitVoid;
    },
    [evaluate](fiber2) {
      if (this[args].while()) {
        fiber2._stack.push(this);
        return this[args].body();
      }
      return exitVoid;
    }
  });
  var forEach = /* @__PURE__ */ dual((args2) => typeof args2[1] === "function", (iterable, f, options) => suspend(() => {
    const concurrency = resolveConcurrency(options?.concurrency);
    if (concurrency === 1) {
      return forEachSequential(iterable, f, options);
    }
    const items = fromIterable(iterable);
    let length = items.length;
    if (length === 0) {
      return options?.discard ? void_ : succeed3([]);
    }
    const out = options?.discard ? void 0 : new Array(length);
    const eff = forEachConcurrent({
      f,
      out
    }, items, {
      concurrency
    });
    return eff ? as(eff, out) : succeed3(out);
  }));
  var forEachSequential = /* @__PURE__ */ __name((iterable, f, options) => suspend(() => {
    const out = options?.discard ? void 0 : [];
    const iterator = iterable[Symbol.iterator]();
    let state = iterator.next();
    let index = 0;
    return as(whileLoop({
      while: /* @__PURE__ */ __name(() => !state.done, "while"),
      body: /* @__PURE__ */ __name(() => f(state.value, index++), "body"),
      step: /* @__PURE__ */ __name((b) => {
        if (out) out.push(b);
        state = iterator.next();
      }, "step")
    }), out);
  }), "forEachSequential");
  var resolveConcurrency = /* @__PURE__ */ __name((concurrency) => concurrency === "unbounded" ? Number.POSITIVE_INFINITY : Math.max(1, concurrency ?? 1), "resolveConcurrency");
  var iterateConcurrentImpl = /* @__PURE__ */ __name((options) => {
    const onItem = options.onItem;
    const step = options.step;
    return (state, items, opts) => {
      let index = 0;
      const end = opts.end ?? items.length;
      const concurrency = opts.concurrency;
      let done = false;
      let parentFiber;
      let fibers;
      let resume;
      let terminal;
      let effect;
      const failDefect = /* @__PURE__ */ __name((error) => {
        const defect = exitDie(error);
        terminal = defect;
        done = true;
        return fibers && fibers.size > 0 ? flatMap(uninterruptible(fiberInterruptAll(Array.from(fibers))), () => terminal ?? defect) : defect;
      }, "failDefect");
      const go = /* @__PURE__ */ __name(() => {
        let paused = false;
        for (; !terminal && index < end; index++) {
          const item = items[index];
          const eff = effect ?? onItem(state, item, index);
          if (effectIsExit(eff)) {
            terminal = step(state, item, eff, index);
            if (terminal) break;
          } else if (!parentFiber) {
            return callback((cb) => {
              parentFiber = getCurrentFiber();
              fibers = /* @__PURE__ */ new Set();
              effect = eff;
              resume = cb;
              let result2;
              try {
                result2 = go();
              } catch (error) {
                return cb(failDefect(error));
              }
              if (result2) return cb(result2);
              return suspend(() => {
                terminal ??= exitVoid;
                return flatMap(fibers ? fiberInterruptAll(fibers) : void_, () => terminal?._tag === "Failure" ? terminal : void_);
              });
            });
          } else {
            effect = void 0;
            const fiber2 = forkUnsafe(parentFiber, eff, true, true, "inherit");
            if (fiber2._exit) {
              terminal = step(state, item, fiber2._exit, index);
              if (terminal) break;
              continue;
            }
            fibers.add(fiber2);
            const currentIndex = index;
            fiber2.addObserver((exit2) => {
              fibers.delete(fiber2);
              try {
                if (terminal) {
                  if (exit2._tag === "Failure") {
                    const reasons = exit2.cause.reasons.filter((reason) => reason._tag !== "Interrupt");
                    if (reasons.length > 0) {
                      const cause = causeFromReasons(reasons);
                      terminal = exitFailCause(terminal._tag === "Failure" ? causeCombine(terminal.cause, cause) : cause);
                    }
                  }
                } else {
                  const result2 = step(state, item, exit2, currentIndex);
                  if (result2) {
                    terminal = result2;
                    go();
                  }
                }
                if (paused) {
                  const eff2 = go();
                  if (eff2) resume(eff2);
                } else if (done && fibers.size === 0) {
                  resume(terminal ?? void_);
                }
              } catch (error) {
                resume(failDefect(error));
              }
            });
            if (fibers.size < concurrency) continue;
            paused = true;
            index++;
            return;
          }
        }
        done = true;
        if (terminal) {
          if (fibers && fibers.size > 0) {
            const annotations = fiberStackAnnotations(parentFiber);
            fibers.forEach((f) => f.interruptUnsafe(parentFiber.id, annotations));
            return;
          }
          if (resume || terminal._tag === "Failure") {
            return terminal;
          }
        } else if (resume) {
          if (!fibers) {
            return exitVoid;
          } else if (fibers.size === 0) {
            resume(void_);
          }
        }
      }, "go");
      return go();
    };
  }, "iterateConcurrentImpl");
  var forEachConcurrent = /* @__PURE__ */ iterateConcurrentImpl({
    onItem(state, item, index) {
      return state.f(item, index);
    },
    step(state, _, exit2, index) {
      if (exit2._tag === "Failure") return exit2;
      else if (state.out) {
        state.out[index] = exit2.value;
      }
    }
  });
  var forkUnsafe = /* @__PURE__ */ __name((parent, effect, immediate = false, daemon = false, uninterruptible2 = false) => {
    const parentRuntime = parent;
    const interruptible2 = uninterruptible2 === "inherit" ? parentRuntime.interruptible : !uninterruptible2;
    const child = new FiberImpl(parentRuntime.context, interruptible2);
    if (!daemon) {
      parentRuntime.children().add(child);
      child._parent = parentRuntime;
    }
    if (immediate) {
      child.evaluate(effect);
    } else {
      child._asyncContext = captureAsyncContext();
      parentRuntime.currentDispatcher.scheduleTask(() => child.evaluate(effect), 0);
    }
    return child;
  }, "forkUnsafe");
  var runForkWith = /* @__PURE__ */ __name((context2) => (effect, options) => {
    const fiber2 = new FiberImpl(options?.scheduler ? add(context2, Scheduler, options.scheduler) : context2, options?.uninterruptible !== true);
    fiber2.evaluate(effect);
    if (fiber2._exit) return fiber2;
    if (options?.signal) {
      if (options.signal.aborted) {
        fiber2.interruptUnsafe();
      } else {
        const abort2 = /* @__PURE__ */ __name(() => fiber2.interruptUnsafe(), "abort");
        options.signal.addEventListener("abort", abort2, {
          once: true
        });
        fiber2.addObserver(() => options.signal.removeEventListener("abort", abort2));
      }
    }
    if (options?.onFiberStart) {
      options.onFiberStart(fiber2);
    }
    return fiber2;
  }, "runForkWith");
  var runPromiseExitWith = /* @__PURE__ */ __name((context2) => {
    const runFork2 = runForkWith(context2);
    return (effect, options) => {
      const fiber2 = runFork2(effect, options);
      return new Promise((resolve) => {
        fiber2.addObserver((exit2) => resolve(exit2));
      });
    };
  }, "runPromiseExitWith");
  var runPromiseWith = /* @__PURE__ */ __name((context2) => {
    const runPromiseExit2 = runPromiseExitWith(context2);
    return (effect, options) => runPromiseExit2(effect, options).then((exit2) => {
      if (exit2._tag === "Failure") {
        throw causeSquash(exit2.cause);
      }
      return exit2.value;
    });
  }, "runPromiseWith");
  var runPromise = /* @__PURE__ */ runPromiseWith(/* @__PURE__ */ empty());
  var ClockRef = /* @__PURE__ */ Reference("effect/Clock", {
    defaultValue: /* @__PURE__ */ __name(() => new ClockImpl(), "defaultValue")
  });
  var MAX_TIMER_MILLIS = 2 ** 31 - 1;
  var ClockImpl = class {
    static {
      __name(this, "ClockImpl");
    }
    currentTimeMillisUnsafe() {
      return Date.now();
    }
    currentTimeMillis = /* @__PURE__ */ sync(() => this.currentTimeMillisUnsafe());
    currentTimeNanosUnsafe() {
      return wallTimeNanos();
    }
    currentTimeNanos = /* @__PURE__ */ sync(() => this.currentTimeNanosUnsafe());
    monotonicTimeNanosUnsafe() {
      return monotonicNowNanos();
    }
    monotonicTimeNanos = /* @__PURE__ */ sync(() => this.monotonicTimeNanosUnsafe());
    sleep(duration) {
      return this.sleepMillis(toMillis(duration));
    }
    sleepMillis(millis2) {
      if (millis2 <= 0) return yieldNow;
      else if (!Number.isFinite(millis2)) return never;
      return callback((resume) => {
        const continuation = millis2 > MAX_TIMER_MILLIS ? this.sleepMillis(millis2 - MAX_TIMER_MILLIS) : void_;
        const handle = setTimeout(() => resume(continuation), Math.min(millis2, MAX_TIMER_MILLIS));
        return sync(() => clearTimeout(handle));
      });
    }
  };
  var nanosPerMilli = /* @__PURE__ */ BigInt(1e6);
  var monotonicNowNanos = /* @__PURE__ */ (function() {
    const processHrtime = globalThis.process?.hrtime;
    if (typeof processHrtime?.bigint === "function") {
      return () => processHrtime.bigint();
    }
    if (typeof performance !== "undefined" && typeof performance.now === "function") {
      return () => BigInt(Math.round(performance.now() * 1e6));
    }
    let previous = /* @__PURE__ */ BigInt(0);
    return () => {
      const current = BigInt(Date.now()) * nanosPerMilli;
      if (current > previous) {
        previous = current;
      }
      return previous;
    };
  })();
  var wallTimeNanos = /* @__PURE__ */ (function() {
    const reanchorThresholdNanos = /* @__PURE__ */ BigInt(1e9);
    let origin;
    return () => {
      const monotonic = monotonicNowNanos();
      const wall = BigInt(Date.now()) * nanosPerMilli;
      if (origin === void 0) {
        origin = wall - monotonic;
      } else {
        const projected = origin + monotonic;
        const skew = wall > projected ? wall - projected : projected - wall;
        if (skew > reanchorThresholdNanos) {
          origin = wall - monotonic;
        }
      }
      return origin + monotonic;
    };
  })();
  var clockWith = /* @__PURE__ */ __name((f) => withFiber((fiber2) => f(fiber2.getRef(ClockRef))), "clockWith");
  var sleep = /* @__PURE__ */ __name((duration) => clockWith((clock) => clock.sleep(fromInputUnsafe(duration))), "sleep");
  var TimeoutErrorTypeId = "~effect/Cause/TimeoutError";
  var TimeoutError = class extends (/* @__PURE__ */ TaggedError("TimeoutError")) {
    static {
      __name(this, "TimeoutError");
    }
    [TimeoutErrorTypeId] = TimeoutErrorTypeId;
    constructor(message) {
      super({
        message
      });
    }
  };
  var IllegalArgumentErrorTypeId = "~effect/Cause/IllegalArgumentError";
  var IllegalArgumentError = class extends (/* @__PURE__ */ TaggedError("IllegalArgumentError")) {
    static {
      __name(this, "IllegalArgumentError");
    }
    [IllegalArgumentErrorTypeId] = IllegalArgumentErrorTypeId;
    constructor(message) {
      super({
        message
      });
    }
  };
  var ExceededCapacityErrorTypeId = "~effect/Cause/ExceededCapacityError";
  var ExceededCapacityError = class extends (/* @__PURE__ */ TaggedError("ExceededCapacityError")) {
    static {
      __name(this, "ExceededCapacityError");
    }
    [ExceededCapacityErrorTypeId] = ExceededCapacityErrorTypeId;
    constructor(message) {
      super({
        message
      });
    }
  };
  var AsyncFiberErrorTypeId = "~effect/Cause/AsyncFiberError";
  var AsyncFiberError = class extends (/* @__PURE__ */ TaggedError("AsyncFiberError")) {
    static {
      __name(this, "AsyncFiberError");
    }
    [AsyncFiberErrorTypeId] = AsyncFiberErrorTypeId;
    constructor(fiber2) {
      super({
        message: "An asynchronous Effect was executed with Effect.runSync",
        fiber: fiber2
      });
    }
  };
  var UnknownErrorTypeId = "~effect/Cause/UnknownError";
  var UnknownError = class extends (/* @__PURE__ */ TaggedError("UnknownError")) {
    static {
      __name(this, "UnknownError");
    }
    [UnknownErrorTypeId] = UnknownErrorTypeId;
    constructor(cause, message) {
      super({
        message,
        cause
      });
    }
  };
  var LoggerTypeId = "~effect/Logger";
  var LoggerProto = {
    [LoggerTypeId]: {
      _Message: identity,
      _Output: identity
    },
    pipe() {
      return pipeArguments(this, arguments);
    }
  };
  var colors = {
    bold: "1",
    red: "31",
    green: "32",
    yellow: "33",
    blue: "34",
    cyan: "36",
    white: "37",
    gray: "90",
    black: "30",
    bgBrightRed: "101"
  };
  var logLevelColors = {
    None: [],
    All: [],
    Trace: [colors.gray],
    Debug: [colors.blue],
    Info: [colors.green],
    Warn: [colors.yellow],
    Error: [colors.red],
    Fatal: [colors.bgBrightRed, colors.black]
  };

  // node_modules/.pnpm/effect@4.0.3/node_modules/effect/dist/Effect.js
  var all2 = all;
  var suspend2 = suspend;
  var sync2 = sync;
  var gen2 = gen;
  var try_2 = try_;
  var andThen2 = andThen;
  var catch_2 = catch_;
  var sleep2 = sleep;
  var forever2 = forever;
  var runPromise2 = runPromise;

  // packages/runtime/src/game/automation/SettingsPolicy.ts
  var run = suspend2(() => {
    const host3 = __host;
    const readiness = try_2({
      try: /* @__PURE__ */ __name(() => JSON.parse(host3.read('{"kind":"readiness"}')), "try"),
      catch: /* @__PURE__ */ __name((cause) => cause, "catch")
    });
    const loaded = /* @__PURE__ */ __name((method) => try_2({
      try: /* @__PURE__ */ __name(() => {
        const reply = host3.swf(method, "[]");
        return reply !== void 0 && JSON.parse(reply) === true;
      }, "try"),
      catch: /* @__PURE__ */ __name((cause) => cause, "catch")
    }), "loaded");
    const tick = /* @__PURE__ */ __name((actionsOnly) => gen2(function* () {
      if ((yield* readiness).missing.length !== 0) return;
      if (!(yield* loaded("auth.isLoggedIn")) || !(yield* loaded("world.isLoaded")) || !(yield* loaded("player.isLoaded"))) {
        return;
      }
      yield* try_2({
        try: /* @__PURE__ */ __name(() => host3.command(JSON.stringify({ kind: "reapplySettings", actionsOnly })), "try"),
        catch: /* @__PURE__ */ __name((cause) => cause, "catch")
      });
    }), "tick");
    const repeat = /* @__PURE__ */ __name((actionsOnly, interval) => forever2(
      tick(actionsOnly).pipe(
        catch_2(
          (error) => sync2(() => console.warn("settings policy tick failed", error))
        ),
        andThen2(sleep2(interval))
      )
    ), "repeat");
    return all2([repeat(false, "1 second"), repeat(true, "500 millis")], {
      concurrency: "unbounded",
      discard: true
    });
  });

  // packages/runtime/src/host/operations.ts
  var operations = /* @__PURE__ */ new Map();
  var completeOperation = /* @__PURE__ */ __name((id, answer, error) => {
    const pending = operations.get(id);
    if (pending === void 0) return;
    operations.delete(id);
    clearTimeout(pending.timeout);
    if (answer !== null) pending.resolve(answer);
    else pending.reject(new Error(error ?? "The host operation failed"));
  }, "completeOperation");

  // packages/runtime/src/host/loop.ts
  var host2 = __host;
  var timers = /* @__PURE__ */ new Map();
  var nextId = 1;
  var schedule = /* @__PURE__ */ __name((callback2, delay2, args2, repeat) => {
    const ms = Math.max(0, Number(delay2) || 0);
    const timer = {
      id: nextId++,
      at: host2.now() + ms,
      callback: /* @__PURE__ */ __name(() => callback2(...args2), "callback"),
      ...repeat ? { repeat: ms } : {}
    };
    timers.set(timer.id, timer);
    return timer.id;
  }, "schedule");
  var clear = /* @__PURE__ */ __name((id) => {
    if (id !== void 0) timers.delete(id);
  }, "clear");
  var describe = /* @__PURE__ */ __name((cause) => {
    try {
      return cause instanceof Error ? cause.stack ?? String(cause) : String(cause);
    } catch {
      return "a value that cannot be converted to text was thrown";
    }
  }, "describe");
  var turn = /* @__PURE__ */ __name((itemsJson) => {
    const now = host2.now();
    const due = [...timers.values()].filter((timer) => timer.at <= now).sort((a, b) => a.at - b.at || a.id - b.id);
    const errors = [];
    const invoke = /* @__PURE__ */ __name((callback2) => {
      try {
        callback2();
      } catch (cause) {
        errors.push(describe(cause));
      }
    }, "invoke");
    for (const item of JSON.parse(itemsJson)) {
      if (item.type === "loader") {
        invoke(() => {
          const callback2 = Reflect.get(window, item.name);
          if (typeof callback2 === "function") callback2.apply(window, item.args);
        });
        for (const event of item.projected) {
          invoke(() => window.onProjectionEvent?.(event));
        }
      } else if (item.type === "operation") {
        completeOperation(item.id, item.answer, item.error);
      } else {
        invoke(() => window.onProjectionInvalidated?.());
      }
    }
    for (const timer of due) {
      if (timers.get(timer.id) !== timer) continue;
      if (timer.repeat === void 0) {
        timers.delete(timer.id);
      } else {
        timers.set(timer.id, { ...timer, at: host2.now() + timer.repeat });
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
    setTimeout: /* @__PURE__ */ __name((callback2, delay2, ...args2) => schedule(callback2, delay2, args2, false), "setTimeout"),
    setInterval: /* @__PURE__ */ __name((callback2, delay2, ...args2) => schedule(callback2, delay2, args2, true), "setInterval"),
    clearTimeout: clear,
    clearInterval: clear
  });
  window.swf = new Proxy({}, {
    get(_target, method) {
      if (typeof method === "string") {
        return (...args2) => {
          const reply = host2.swf(method, JSON.stringify(args2));
          return reply === void 0 ? void 0 : JSON.parse(reply);
        };
      }
      return void 0;
    }
  });

  // packages/runtime/src/bot.ts
  runPromise2(run);
  Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
})();
