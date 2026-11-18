import { describe, expect, it } from "vitest";
import { ApiError, DISCORD_LINK_ERROR_TEXT, ERROR_TEXT, LOGIN_ERROR_TEXT } from "./errors";

// The codes here mirror the backend's enums; these check they still list the same variants.

const SOURCES: Record<string, string> = import.meta.glob(
  ["../../../backend/src/error.rs", "../../../backend/src/handlers/auth.rs"],
  { query: "?raw", import: "default", eager: true },
);

function source(path: string): string {
  const key = Object.keys(SOURCES).find((k) => k.endsWith(path));
  if (!key) throw new Error(`${path} not found`);
  return SOURCES[key];
}

const snakeCase = (name: string) => name.replace(/(?<!^)([A-Z])/g, "_$1").toLowerCase();

/** A Rust enum's body. */
function enumBody(path: string, name: string): string {
  const text = source(path);
  const start = text.indexOf(`enum ${name} {`);
  if (start < 0) throw new Error(`enum ${name} not found in ${path}`);
  return text.slice(start, text.indexOf("\n}", start));
}

/** A Rust enum's unit variants as serde's snake_case writes them, in order. */
function variants(path: string, name: string): string[] {
  return [...enumBody(path, name).matchAll(/^\s+([A-Z][A-Za-z0-9]*),$/gm)].map((m) =>
    snakeCase(m[1]),
  );
}

/** Every `group.code` the backend can answer with: ErrorCode's groups and each group's variants. */
function backendCodes(): string[] {
  const groups = [
    ...enumBody("error.rs", "ErrorCode").matchAll(/^\s+([A-Z]\w*)\(#\[from\] (\w+)\),$/gm),
  ];
  if (groups.length === 0) throw new Error("ErrorCode has no groups");
  return groups.flatMap(([, group, inner]) =>
    variants("error.rs", inner).map((code) => `${snakeCase(group)}.${code}`),
  );
}

describe("error codes", () => {
  const texts: Record<string, Record<string, string>> = ERROR_TEXT;
  const frontend = Object.entries(texts).flatMap(([group, codes]) =>
    Object.keys(codes).map((c) => `${group}.${c}`),
  );

  it("are the backend's groups and their variants", () => {
    expect(frontend.sort()).toEqual(backendCodes().sort());
  });

  it("each have their own Turkish text", () => {
    for (const codes of Object.values(texts))
      for (const text of Object.values(codes)) expect(text).toBeTruthy();
  });
});

describe("callback errors", () => {
  it("cover CallbackError between the login and the Discord link pages", () => {
    const shown = new Set([
      ...Object.keys(LOGIN_ERROR_TEXT),
      ...Object.keys(DISCORD_LINK_ERROR_TEXT),
    ]);
    expect([...shown].sort()).toEqual(variants("handlers/auth.rs", "CallbackError").sort());
  });
});

describe("ApiError", () => {
  it("shows a code in Turkish, else the server's message, else the status", () => {
    expect(new ApiError(409, "lobby.full", "that lobby is full").message).toBe(
      ERROR_TEXT.lobby.full,
    );
    expect(new ApiError(409, null, "something new").message).toBe("something new");
    expect(new ApiError(502, null).message).toBe("İstek başarısız oldu (502)");
  });
});
