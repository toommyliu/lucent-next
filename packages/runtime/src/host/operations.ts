import type { Operation, OperationAnswer } from "@lucent/game/protocol";

const operations = new Map<
  string,
  {
    resolve: (answer: OperationAnswer) => void;
    reject: (cause: Error) => void;
  }
>();

export const operate = (operation: Operation): Promise<OperationAnswer> =>
  new Promise((resolve, reject) => {
    const id = __host.operate(JSON.stringify(operation));
    operations.set(id, { resolve, reject });
  });

export const completeOperation = (
  id: string,
  answer: OperationAnswer | null,
  error: string | null,
): void => {
  const pending = operations.get(id);
  if (pending === undefined) return;
  operations.delete(id);
  if (answer !== null) pending.resolve(answer);
  else pending.reject(new Error(error ?? "The host operation failed"));
};
