# What the host must provide

One line per shim in `src/shims/`, then the globals the code reads from the DOM lib. Everything else in `src/` is copied from lucent `main` with import paths changed only.

## Shims

- `shims/bridge.d.ts`: `Window.swf`, the 147 loader ExternalInterface methods (`auth.login`, `player.walkTo`, `world.loadSwf`, ...), plus the callbacks the runtime assigns on `window` (`onConnection`, `onDebug`, `onExtensionResponse`, `onLoaded`, `onProgress`, `packetFromClient`, `packetFromServer`). Needed by `game/flash/bridge/Bridge.ts` (calls), `game/flash/bridge/Gateway.ts` (callbacks), and the `game/flash/api/*` services that name methods by `keyof Window["swf"]` (Auth, Bank, Combat, Drops, House, Inventory, Map, Monsters, Player, Quests, Settings, Shops, Wait). Verbatim copy of lucent's generated `bridge.d.ts`.
- `shims/desktop.d.ts`: `Window.desktop: DesktopBridge`, the Electron preload IPC client (type in `shared/desktopBridge.ts`). Needed by `game/flash/runtime.ts` (`desktop.debug`, `desktop.log.write`), `game/flash/api/Api.ts` (`desktop.debug`), `game/army/Army.ts` and `game/army/ArmyLoopTaunt.ts` (`army`), `game/environment/Environment.ts` (`environment`), `game/automation/Follower.ts` (`gameFollower`, `combatProfiles.getState`), `game/consoleForwarder.ts` (game console observability bridge), `game/scripting/ScriptRunner.ts` and `ScriptEvaluator.ts` (`accountSettings`, `fileSystem`, `http`, `gameView`), `game/scripting/ScriptFileSystem.ts` (`fileSystem`), `game/scripting/ScriptExecutionProtection.ts` (`gameRenderer.beginScriptExecution`, `finishScriptExecution`).
- `shims/tupleInference.ts`: typecheck only, nothing for the host. It works around a TypeScript 6.0.3 check-order quirk: `game/scripting/http/response.ts` (`new Headers(entries.map(([n, v]) => [n, v]))`) fails to infer a tuple unless a `[string, string]` array type is checked first. `tsconfig.json` lists `src/shims/**/*` first for this reason.

## DOM globals without a shim file

The tsconfig uses `lib: dom`. These are typed by the DOM lib and resolved at run time by whatever hosts the code.

- `window` (assigned `onConnection`, `onDebug`, `onExtensionResponse`, `onLoaded`, `onProgress`, `packetFromClient`, `packetFromServer`; read for `swf` and `desktop`): `game/flash/bridge/Gateway.ts`, `Bridge.ts`, and the modules above.
- `AudioContext`, `window.setTimeout`: `game/audio/beep.ts`.
- `globalThis.setTimeout`: `game/groupCommandQueue.ts`.
- `Headers`, `TextDecoder`, `AbortSignal` events: `game/scripting/http/*`.

## Not copied (main-process code the tests imported)

- `game/army/ArmySimulation.test.ts` needs `app/src/main/game/army/ArmyCoordinator`.
- `game/scripting/ScriptHttp.test.ts` needs `app/src/main/http/DesktopHttpClient`, `ipc/DesktopIpcInvoke`, `preloadIpcClient`, `scripting/ScriptHttp`.

The Rust host owns both sides of those tests.
