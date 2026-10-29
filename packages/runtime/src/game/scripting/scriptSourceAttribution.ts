const SCRIPT_SOURCE_PATTERN = /lucent-script:\/\/(?:loose|package)\/([^\s:?#]+)/g;
const SCRIPT_FRAME_PATTERN = /lucent-script:\/\/(?:loose|package)\/[^\s:?#]+:\d+:\d+/;

const decodeSegment = (value: string): string => {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
};

export const displayScriptSourceText = (text: string): string =>
  text.replace(SCRIPT_SOURCE_PATTERN, (_url, identity: string) =>
    identity.split("/").filter(Boolean).map(decodeSegment).join("/"),
  );

const errorChain = (error: Error): readonly Error[] => {
  const chain: Error[] = [];
  const seen = new Set<Error>();
  let current: Error | undefined = error;
  while (current !== undefined && !seen.has(current)) {
    seen.add(current);
    chain.push(current);
    const cause: unknown = Reflect.get(current, "cause");
    current = cause instanceof Error ? cause : undefined;
  }
  return chain;
};

export const attributedScriptErrorMessage = (error: Error): string => {
  const message = displayScriptSourceText(error.message);
  if (SCRIPT_FRAME_PATTERN.test(error.message)) return message;
  for (const entry of errorChain(error)) {
    const frame = entry.stack?.match(SCRIPT_FRAME_PATTERN);
    if (frame) return `${displayScriptSourceText(frame[0])}: ${message}`;
  }
  return message;
};

export const attributedScriptErrorDetails = (
  error: Error,
): string | undefined => {
  const details = errorChain(error)
    .map((entry) =>
      displayScriptSourceText(
        entry.stack?.trim() === ""
          ? entry.message
          : (entry.stack ?? entry.message),
      ),
    )
    .filter((entry) => entry.trim() !== "");
  return details.length === 0 ? undefined : details.join("\nCaused by:\n");
};
