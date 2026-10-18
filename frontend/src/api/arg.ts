import { post } from "./client";
import type { ArgGuessResult } from "./types";

export const guessArg = (guess: string) => post<ArgGuessResult>("/api/kupa", { guess });
