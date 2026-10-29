import "./globals";

import { turn, wake } from "./loop";
import { run } from "./run";

Object.assign(globalThis, { __lucent: Object.freeze({ run, turn, wake }) });
