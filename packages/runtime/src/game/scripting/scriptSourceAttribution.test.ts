import { describe, expect, it } from "@effect/vitest";

import {
  attributedScriptErrorDetails,
  attributedScriptErrorMessage,
  displayScriptSourceText,
} from "./scriptSourceAttribution";

describe("script source attribution", () => {
  it("decodes loose and package paths without changing locations", () => {
    expect(
      displayScriptSourceText(
        "lucent-script://loose/farming/my%20script.js:2:9: broken",
      ),
    ).toBe("farming/my script.js:2:9: broken");
    expect(
      displayScriptSourceText(
        "Failed in lucent-script://package/%40a%2Fb/lib/my%20script.js:18:11",
      ),
    ).toBe("Failed in @a/b/lib/my script.js:18:11");
    expect(
      displayScriptSourceText("Request to https://example.com failed"),
    ).toBe("Request to https://example.com failed");
  });

  it("reads a frame with parentheses in the path", () => {
    const cause = new Error("boom");
    cause.stack =
      "Error: boom\n    at run (lucent-script://package/%40a%2Fb/lib/my(script).js:5:9)";
    expect(attributedScriptErrorMessage(new Error("failed", { cause }))).toBe(
      "@a/b/lib/my(script).js:5:9: failed",
    );
    expect(
      attributedScriptErrorMessage(
        new Error(
          "lucent-script://loose/error.js:1:18: SyntaxError: Unexpected token ';'",
        ),
      ),
    ).toBe("error.js:1:18: SyntaxError: Unexpected token ';'");
  });

  it("displays the cause location and both stacks in details", () => {
    const cause = new Error("boom");
    cause.stack =
      "Error: boom\n    at run (lucent-script://loose/farming/example.js:7:5)";
    const outer = new Error("Script execution failed", { cause });
    outer.stack =
      "Error: Script execution failed\n    at loader (runtime.js:1:2)";
    expect(attributedScriptErrorMessage(outer)).toBe(
      "farming/example.js:7:5: Script execution failed",
    );
    expect(attributedScriptErrorDetails(outer)).toBe(
      "Error: Script execution failed\n    at loader (runtime.js:1:2)\nCaused by:\nError: boom\n    at run (farming/example.js:7:5)",
    );
  });

  it("preserves malformed percent escapes", () => {
    expect(displayScriptSourceText("lucent-script://loose/bad%XX.js:1:2")).toBe(
      "bad%XX.js:1:2",
    );
  });
});
