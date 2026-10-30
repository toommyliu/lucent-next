import type { Operation, OperationAnswer } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";

const operations = new Map<
  string,
  {
    complete: (answer: OperationAnswer | null, error: string | null) => void;
    fail: (cause: Error) => void;
    timeout?: ReturnType<typeof setTimeout>;
  }
>();

export const operate = (
  operation: Operation,
  host: Pick<typeof __host, "prepareOperation" | "operate" | "cancelOperation">,
): Effect.Effect<OperationAnswer, Error> =>
  Effect.callback((resume) => {
    let id: string;
    let json: string;
    try {
      json = JSON.stringify(operation);
      id = host.prepareOperation();
    } catch (cause) {
      resume(Effect.fail(cause instanceof Error ? cause : new Error(String(cause))));
      return;
    }
    const cancel = () => {
      const pending = operations.get(id);
      if (pending === undefined) return;
      operations.delete(id);
      clearTimeout(pending.timeout);
      host.cancelOperation(id);
    };
    const complete = (answer: OperationAnswer | null, error: string | null) => {
      const pending = operations.get(id);
      if (pending === undefined) return;
      operations.delete(id);
      clearTimeout(pending.timeout);
      resume(answer === null
        ? Effect.fail(new Error(error ?? "The host operation failed"))
        : Effect.succeed(answer));
    };
    const fail = (cause: Error) => {
      cancel();
      resume(Effect.fail(cause));
    };
    operations.set(id, { complete, fail });
    try {
      host.operate(id, json);
    } catch (cause) {
      fail(cause instanceof Error ? cause : new Error(String(cause)));
    }
    return Effect.sync(cancel);
  });

export const startOperation = (id: string): void => {
  const pending = operations.get(id);
  if (pending === undefined || pending.timeout !== undefined) return;
  pending.timeout = setTimeout(() => {
    pending.fail(new Error("The host operation timed out"));
  }, 35_000);
};

export const completeOperation = (
  id: string,
  answer: OperationAnswer | null,
  error: string | null,
): void => operations.get(id)?.complete(answer, error);

export const failOperations = (cause: Error): void => {
  for (const pending of operations.values()) {
    pending.fail(cause);
  }
};
