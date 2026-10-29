import { toLiveServer } from "@lucent/game";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";

import type { BridgeService } from "../bridge/Bridge";
import type { HostStateService } from "../state/HostState";
import type { Wait } from "./Wait";

const Credentials = Schema.Struct({
  password: Schema.String,
  username: Schema.String,
});
const ConnectResult = Schema.Union([
  Schema.Struct({
    message: Schema.String,
    ok: Schema.Literal(true),
    serverName: Schema.optionalKey(Schema.String),
    status: Schema.Literal("selected"),
  }),
  Schema.Struct({
    message: Schema.String,
    ok: Schema.Literal(false),
    reason: Schema.optionalKey(Schema.String),
    serverName: Schema.optionalKey(Schema.String),
    status: Schema.Literals(["blocked", "not-found", "not-ready"]),
  }),
]);
const decodeCredentials = Schema.decodeUnknownOption(Credentials);
const temporaryKickFallbackMs = 60_000;
const temporaryKickMaximumMs = 70_000;

export interface ConnectOutcome {
  readonly message: string;
  readonly retryable: boolean;
  readonly serverName?: string;
  readonly status:
    | "blocked"
    | "connected"
    | "connection-error"
    | "connection-failed"
    | "full"
    | "not-found"
    | "not-ready"
    | "timeout";
}

export const makeAuth = (
  bridge: BridgeService,
  host: HostStateService,
  wait: Wait,
) => {
  const cached = host.read({ kind: "auth" });
  const loggedIn = host.command({ kind: "refreshLoggedIn" }).pipe(
    Effect.flatMap(
      Option.match({
        onNone: () => cached.pipe(Effect.map((auth) => auth.loggedIn)),
        onSome: Effect.succeed,
      }),
    ),
  );
  const currentCredentials = host.command({ kind: "refreshCredentials" }).pipe(
    Effect.flatMap(
      Option.match({
        onNone: () => cached,
        onSome: Effect.succeed,
      }),
    ),
  );
  const temporarilyKicked = bridge
    .invoke("auth.isTemporarilyKicked", undefined, Schema.Boolean)
    .pipe(Effect.map(Option.getOrElse(() => false)));

  const getTemporaryKickRemainingMs = () =>
    bridge
      .invokeJson("flash.getGameObject", ["mcLogin.warning.n"], Schema.Number)
      .pipe(
        Effect.map(
          Option.match({
            onNone: () => undefined,
            onSome: (seconds) =>
              Number.isFinite(seconds) && seconds > 0
                ? Math.min(temporaryKickMaximumMs, Math.ceil(seconds) * 1_000)
                : undefined,
          }),
        ),
      );

  const waitForTemporaryKickClear = Effect.gen(function* () {
    if (!(yield* temporarilyKicked)) return true;

    const remainingMs =
      (yield* getTemporaryKickRemainingMs()) ?? temporaryKickFallbackMs;
    const initialSleepMs = Math.min(
      temporaryKickMaximumMs,
      remainingMs + 1_000,
    );
    yield* Effect.sleep(`${initialSleepMs} millis`);

    if (!(yield* temporarilyKicked)) return true;
    const finalWaitMs = Math.max(0, temporaryKickMaximumMs - initialSleepMs);
    if (finalWaitMs === 0) return false;

    return yield* wait.until(
      temporarilyKicked.pipe(Effect.map((kicked) => !kicked)),
      {
        interval: "5 seconds",
        timeout: `${finalWaitMs} millis`,
      },
    );
  });

  const connectTo = (server: string): Effect.Effect<ConnectOutcome> => {
    const requested = server.trim();
    if (requested === "") {
      return Effect.succeed({
        message: "server is required",
        retryable: false,
        status: "not-found",
      });
    }
    const select = bridge
      .invoke("auth.connectTo", [requested], ConnectResult)
      .pipe(
        Effect.map(
          Option.match({
            onNone: (): ConnectOutcome => ({
              message: "server selection is not ready",
              retryable: true,
              serverName: requested,
              status: "not-ready",
            }),
            onSome: (result): ConnectOutcome =>
              result.ok
                ? {
                    message: result.message,
                    retryable: false,
                    serverName: result.serverName ?? requested,
                    status: "connected",
                  }
                : {
                    message: result.message,
                    retryable:
                      result.status === "not-ready" || result.reason === "full",
                    serverName: result.serverName ?? requested,
                    status: result.reason === "full" ? "full" : result.status,
                  },
          }),
        ),
      );

    return Effect.gen(function* () {
      const initial = yield* select;
      if (initial.status !== "not-ready") return initial;

      yield* Effect.sleep("250 millis");
      const settled = yield* wait.untilSome(
        select.pipe(
          Effect.map((outcome) =>
            outcome.status === "not-ready"
              ? Option.none<ConnectOutcome>()
              : Option.some(outcome),
          ),
        ),
        { interval: "500 millis", timeout: "5 seconds" },
      );
      return settled ?? initial;
    });
  };

  const getPassword = () =>
    currentCredentials.pipe(Effect.map((current) => current.password));

  const getServers = () =>
    host.command({ kind: "refreshServers" }).pipe(
      Effect.flatMap(
        Option.match({
          onNone: () => cached.pipe(Effect.map((auth) => auth.servers)),
          onSome: Effect.succeed,
        }),
      ),
      Effect.map((servers) => servers.map(toLiveServer)),
    );

  const getUsername = () =>
    currentCredentials.pipe(Effect.map((current) => current.username));

  const isServerSelectReady = () =>
    bridge
      .invoke("flash.isNull", ["mcLogin.sl.iList"], Schema.Boolean)
      .pipe(Effect.map((value) => !Option.getOrElse(value, () => true)));

  const isLoggedIn = () => loggedIn;

  const isTemporarilyKicked = () => temporarilyKicked;

  const login = (username: string, password: string) => {
    const credentials = decodeCredentials({
      password,
      username: username.trim(),
    });
    if (
      Option.isNone(credentials) ||
      credentials.value.username === "" ||
      credentials.value.password === ""
    ) {
      return Effect.succeed(false);
    }
    return waitForTemporaryKickClear.pipe(
      Effect.flatMap((ready) =>
        ready
          ? host
              .command({
                kind: "login",
                username: credentials.value.username,
                password: credentials.value.password,
              })
              .pipe(Effect.map(Option.getOrElse(() => false)))
          : Effect.succeed(false),
      ),
    );
  };

  const logout = () => host.command({ kind: "logout" }).pipe(Effect.asVoid);

  return {
    connectTo,
    getPassword,
    getServers,
    getUsername,
    isLoggedIn,
    isServerSelectReady,
    isTemporarilyKicked,
    login,
    logout,
  };
};

export type Auth = ReturnType<typeof makeAuth>;
