// src/scriptPath.ts
const windowsReservedPathSegment = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$/i;
const unsafeScriptPathCharacters = /[<>:"\\|?*]/;
const hasControlCharacter = (value) => Array.from(value).some((character) => character.charCodeAt(0) < 32);
const parseScriptPathSegments = (value) => {
	if (value === "" || value.startsWith("/") || value.includes("\\")) return null;
	const segments = value.split("/");
	return segments.every((segment) => segment !== "" && segment !== "." && segment !== ".." && !segment.endsWith(".") && !segment.endsWith(" ") && !unsafeScriptPathCharacters.test(segment) && !hasControlCharacter(segment) && !windowsReservedPathSegment.test(segment)) ? segments : null;
};
export { parseScriptPathSegments };
