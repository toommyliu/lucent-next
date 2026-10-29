import "./globals";

import { turn, wake } from "./loop";
import { start, stop } from "./scripts";

Object.assign(globalThis, { __lucent: Object.freeze({ start, stop, turn, wake }) });
