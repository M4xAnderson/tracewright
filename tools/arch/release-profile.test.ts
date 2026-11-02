import { describe, expect, test } from "bun:test";
import { checkReleaseProfile } from "./release-profile.ts";

describe("checkReleaseProfile", () => {
  test("accepts a release profile that unwinds", () => {
    expect(checkReleaseProfile('[profile.release]\npanic = "unwind"\n')).toEqual([]);
  });

  test("rejects a release profile that aborts", () => {
    expect(checkReleaseProfile('[profile.release]\npanic = "abort"\n')).toEqual([
      'profile.release must set panic = "unwind" (ADR 0006, D-72)',
      'profile.release sets panic = "abort"; a worker panic would kill the app (ADR 0006, D-72)',
    ]);
  });

  test("rejects a workspace that leaves the release panic strategy implicit", () => {
    expect(checkReleaseProfile("[workspace]\n")).toEqual([
      'profile.release must set panic = "unwind" (ADR 0006, D-72)',
    ]);
  });

  test("rejects any other profile that aborts", () => {
    const toml =
      '[profile.release]\npanic = "unwind"\n\n[profile.dist]\ninherits = "release"\npanic = "abort"\n';
    expect(checkReleaseProfile(toml)).toEqual([
      'profile.dist sets panic = "abort"; a worker panic would kill the app (ADR 0006, D-72)',
    ]);
  });
});
