export class ArmyError extends Error {
  readonly _tag = "ArmyError";

  constructor(message: string, cause?: unknown) {
    super(message);
    this.name = "ArmyError";
    if (cause !== undefined) {
      Object.defineProperty(this, "cause", {
        configurable: true,
        enumerable: false,
        value: cause,
        writable: true,
      });
    }
  }
}

export class ArmyLoopTauntError extends Error {
  readonly _tag = "ArmyLoopTauntError";

  constructor(message: string, cause?: unknown) {
    super(message);
    this.name = "ArmyLoopTauntError";
    if (cause !== undefined) {
      Object.defineProperty(this, "cause", {
        configurable: true,
        enumerable: false,
        value: cause,
        writable: true,
      });
    }
  }
}
