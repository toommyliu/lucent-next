import * as Effect from "effect/Effect";

import { ArmyError, ArmyLoopTauntError } from "../game/army/ArmyErrors";
import type { ScriptDialogsShape } from "../game/scripting/ScriptDialogs";
import type { ScriptServicesShape } from "../game/scripting/ScriptServices";
import { HttpError } from "../shared/http";

export class HostServiceUnavailable extends Error {
  constructor(readonly service: string) {
    super(`The ${service} service is not available yet.`);
    this.name = "HostServiceUnavailable";
  }
}

const defect = (service: string) => () =>
  Effect.die(new HostServiceUnavailable(service));
const failure = (service: string) => () =>
  Effect.fail(new HostServiceUnavailable(service));
const reject = (service: string) => () =>
  Promise.reject(new HostServiceUnavailable(service));
const armyFailure =
  <E>(ErrorClass: new (message: string, cause: unknown) => E) => () => {
    const cause = new HostServiceUnavailable("army");
    return Effect.fail(new ErrorClass(cause.message, cause));
  };

export const unavailableScriptServices = {
  army: {
    equipSet: armyFailure(ArmyError),
    getConfigString: defect("army"),
    getConfigValue: defect("army"),
    getPlayerNumber: defect("army"),
    getSession: defect("army"),
    isLeader: defect("army"),
    isMember: defect("army"),
    isStarted: defect("army"),
    joinMap: armyFailure(ArmyError),
    kill: armyFailure(ArmyError),
    killForItem: armyFailure(ArmyError),
    killForTempItem: armyFailure(ArmyError),
    leave: defect("army"),
    loopTaunt: armyFailure(ArmyLoopTauntError),
    runStep: armyFailure(ArmyError),
    start: armyFailure(ArmyError),
    sync: armyFailure(ArmyError),
    waitForAllInMap: armyFailure(ArmyError),
  },
  environment: {
    getState: defect("environment"),
    clear: failure("environment"),
    addQuest: failure("environment"),
    removeQuest: failure("environment"),
    setQuestReward: failure("environment"),
    clearQuestReward: failure("environment"),
    clearQuests: failure("environment"),
    setQuestAutoRegister: failure("environment"),
    setAutoRegisterRequirements: failure("environment"),
    setAutoRegisterRewards: failure("environment"),
    addItem: failure("environment"),
    removeItem: failure("environment"),
    setAcceptAcMemberOnlyDrops: failure("environment"),
    setAcceptAcNonMemberDrops: failure("environment"),
    setAcceptNonAcMemberOnlyDrops: failure("environment"),
    setAcceptNonAcNonMemberDrops: failure("environment"),
    setRejectUnregisteredDrops: failure("environment"),
    setDropPolicy: failure("environment"),
    setItemRules: failure("environment"),
    clearItems: failure("environment"),
    addBoost: failure("environment"),
    removeBoost: failure("environment"),
    clearBoosts: failure("environment"),
    fetchBoosts: failure("environment"),
    setBoostAutomationEnabled: failure("environment"),
    setDropAutomationEnabled: failure("environment"),
    setItemNotification: failure("environment"),
    setQuestAutomationEnabled: failure("environment"),
    syncToAll: failure("environment"),
  },
  fileSystem: {
    exists: reject("fileSystem"),
    list: reject("fileSystem"),
    readJson: reject("fileSystem"),
    readText: reject("fileSystem"),
    remove: reject("fileSystem"),
    writeJson: reject("fileSystem"),
    writeText: reject("fileSystem"),
  },
  http: () =>
    Effect.succeed({
      HttpError,
      request: (url: string | URL) =>
        Effect.fail(
          new HttpError({
            reason: "request",
            url: String(url),
            detail: new HostServiceUnavailable("http").message,
          }),
        ),
    }),
} satisfies Omit<ScriptServicesShape, "autoZone" | "autoRelogin">;

export const unavailableScriptDialogs = {
  alert: defect("dialogs"),
  beep: defect("beep"),
  confirm: defect("dialogs"),
  getCurrent: defect("dialogs"),
  onCurrent: defect("dialogs"),
  prompt: defect("dialogs"),
  respond: defect("dialogs"),
} satisfies ScriptDialogsShape;
