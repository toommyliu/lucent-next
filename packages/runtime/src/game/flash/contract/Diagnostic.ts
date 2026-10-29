export type DiagnosticPhase =
  | "callback-decode"
  | "callback-dispatch"
  | "invoke"
  | "packet-decode";

export interface Diagnostic {
  readonly arguments?: readonly unknown[];
  readonly cause: unknown;
  readonly operation: string;
  readonly phase: DiagnosticPhase;
  readonly timestamp: number;
}

export const diagnosticTimestamp = (): number =>
  performance.timeOrigin + performance.now();

export const makeDiagnostic = (
  phase: DiagnosticPhase,
  operation: string,
  cause: unknown,
  args?: readonly unknown[],
): Diagnostic => ({
  ...(args === undefined ? {} : { arguments: args }),
  cause,
  operation,
  phase,
  timestamp: diagnosticTimestamp(),
});
