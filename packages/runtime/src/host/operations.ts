import type { Operation, OperationAnswer } from "@lucent/game/protocol";

const operations = new Map<
  string,
  {
    resolve: (answer: OperationAnswer) => void;
    reject: (cause: Error) => void;
    timeout: ReturnType<typeof setTimeout>;
  }
>();

export const operate = (
  operation: Operation,
  host: Pick<typeof __host, "operate">,
): Promise<OperationAnswer> =>
  new Promise((resolve, reject) => {
    const id = host.operate(JSON.stringify(operation));
    const timeout = setTimeout(() => {
      operations.delete(id);
      reject(new Error("The host operation timed out"));
    }, 35_000);
    operations.set(id, { resolve, reject, timeout });
  });

export const completeOperation = (
  id: string,
  answer: OperationAnswer | null,
  error: string | null,
): void => {
  const pending = operations.get(id);
  if (pending === undefined) return;
  operations.delete(id);
  clearTimeout(pending.timeout);
  if (answer !== null) pending.resolve(answer);
  else pending.reject(new Error(error ?? "The host operation failed"));
};
