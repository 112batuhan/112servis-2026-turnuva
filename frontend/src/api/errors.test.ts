import { describe, expect, it } from "vitest";
import { CLOSE_REASON_TEXT } from "../irc/IrcContext";
import { ApiError, DISCORD_LINK_ERROR_TEXT, ERROR_CODES, ERROR_TEXT, LOGIN_ERROR_TEXT } from "./errors";

// The codes here mirror the backend's enums; these check they still list the same variants.

const SOURCES: Record<string, string> = import.meta.glob(
  ["../../../backend/src/error/code.rs", "../../../backend/src/bancho/relay.rs", "../../../backend/src/handlers/auth.rs"],
  { query: "?raw", import: "default", eager: true },
);

function source(path: string): string {
  const key = Object.keys(SOURCES).find((k) => k.endsWith(path));
  if (!key) throw new Error(`${path} not found`);
  return SOURCES[key];
}

/** A Rust enum's variants as serde's snake_case writes them, in order. */
function variants(path: string, name: string): string[] {
  const text = source(path);
  const start = text.indexOf(`enum ${name} {`);
  const body = text.slice(start, text.indexOf("\n}", start));
  return [...body.matchAll(/^\s+([A-Z][A-Za-z0-9]*),$/gm)].map((m) =>
    m[1].replace(/(?<!^)([A-Z])/g, "_$1").toLowerCase(),
  );
}

describe("error codes", () => {
  it("are ErrorCode's variants, in order", () => {
    expect([...ERROR_CODES]).toEqual(variants("error/code.rs", "ErrorCode"));
  });

  it("each have their own Turkish text", () => {
    for (const code of ERROR_CODES) expect(ERROR_TEXT[code]).toBeTruthy();
  });
});

describe("callback errors", () => {
  it("cover CallbackError between the login and the Discord link pages", () => {
    const shown = new Set([...Object.keys(LOGIN_ERROR_TEXT), ...Object.keys(DISCORD_LINK_ERROR_TEXT)]);
    expect([...shown].sort()).toEqual(variants("handlers/auth.rs", "CallbackError").sort());
  });
});

describe("relay close reasons", () => {
  it("are CloseReason's variants", () => {
    expect(Object.keys(CLOSE_REASON_TEXT).sort()).toEqual(variants("bancho/relay.rs", "CloseReason").sort());
  });
});

describe("ApiError", () => {
  it("shows a code in Turkish, else the server's message, else the status", () => {
    expect(new ApiError(409, "lobby_full", "that lobby is full").message).toBe(ERROR_TEXT.lobby_full);
    expect(new ApiError(409, null, "something new").message).toBe("something new");
    expect(new ApiError(502, null).message).toBe("İstek başarısız oldu (502)");
  });
});
