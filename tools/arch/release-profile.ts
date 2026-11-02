// ADR 0006 and D-72: under `panic = "abort"` a worker panic kills the app before the supervisor
// can join the thread and report Unknown "internal error", so unwind is a precondition of the
// crash policy. The release profile states it explicitly so a later edit cannot drop it silently.

type Profiles = Record<string, { panic?: unknown }>;

/** Checks the workspace root `Cargo.toml` text; returns one message per violation. */
export function checkReleaseProfile(cargoToml: string): string[] {
  const parsed = Bun.TOML.parse(cargoToml) as { profile?: Profiles };
  const profiles = parsed.profile ?? {};
  const problems: string[] = [];
  if (profiles.release?.panic !== "unwind") {
    problems.push('profile.release must set panic = "unwind" (ADR 0006, D-72)');
  }
  for (const [name, profile] of Object.entries(profiles)) {
    if (profile.panic === "abort") {
      problems.push(
        `profile.${name} sets panic = "abort"; a worker panic would kill the app (ADR 0006, D-72)`,
      );
    }
  }
  return problems;
}
