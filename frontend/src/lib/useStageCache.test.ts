import { describe, expect, it } from "vitest";
import { createStageCache } from "./useStageCache";

/** A fetcher whose answers are given by hand, in any order. */
function manual() {
  const calls: { id: string; resolve: (v: string) => void; reject: (e: Error) => void }[] = [];
  const fetcher = (id: string) =>
    new Promise<string>((resolve, reject) => {
      calls.push({ id, resolve, reject });
    });
  return { calls, fetcher };
}

describe("createStageCache", () => {
  it("shares a request on its way and a recent reading, unless fresh", async () => {
    const { calls, fetcher } = manual();
    const cache = createStageCache(fetcher);
    cache.prefetch("a");
    const click = cache.load("a");
    expect(calls).toHaveLength(1);
    calls[0].resolve("A1");
    expect((await click).data).toBe("A1");

    await cache.load("a");
    expect(calls).toHaveLength(1);

    const live = cache.load("a", true);
    expect(calls).toHaveLength(2);
    calls[1].resolve("A2");
    expect((await live).data).toBe("A2");
  });

  it("never lets an older answer replace a newer one", async () => {
    const { calls, fetcher } = manual();
    const cache = createStageCache(fetcher);
    const first = cache.load("a", true);
    const second = cache.load("a", true);
    calls[1].resolve("new");
    await second;
    calls[0].resolve("old");
    await first;
    expect(cache.get("a")?.data).toBe("new");
  });

  it("prefetches only what isn't kept, and tells subscribers when something is stored", async () => {
    const { calls, fetcher } = manual();
    const cache = createStageCache(fetcher);
    let heard = 0;
    cache.subscribe(() => heard++);
    const v = cache.version();
    const load = cache.load("a");
    calls[0].resolve("A");
    await load;
    expect(heard).toBe(1);
    expect(cache.version()).not.toBe(v);
    cache.prefetch("a");
    expect(calls).toHaveLength(1);
  });

  it("keeps nothing for a failed read and lets the next one try again", async () => {
    const { calls, fetcher } = manual();
    const cache = createStageCache(fetcher);
    const load = cache.load("a");
    calls[0].reject(new Error("down"));
    await expect(load).rejects.toThrow("down");
    expect(cache.get("a")).toBeUndefined();
    cache.prefetch("a");
    expect(calls).toHaveLength(2);
  });
});
