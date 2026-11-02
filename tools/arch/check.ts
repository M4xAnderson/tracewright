// Runs the architecture rules on this repository: `bun tools/arch/check.ts`. Exits 1 on any
// violation. The rules themselves are pure functions with their own tests next to this file.
import { readdir } from "node:fs/promises";
import { join, relative } from "node:path";
import { checkCrateEdges, type CargoMetadata } from "./crate-edges.ts";
import { checkNetImpls } from "./net-impls.ts";
import { checkReleaseProfile } from "./release-profile.ts";

const root = join(import.meta.dir, "..", "..");
const netRoot = join(root, "crates", "net");

const metadata = (await Bun.$`cargo metadata --format-version 1`
  .cwd(root)
  .quiet()
  .json()) as CargoMetadata;

const netSources = await Promise.all(
  (await readdir(join(netRoot, "src"), { recursive: true }))
    .filter((path) => path.endsWith(".rs"))
    .map(async (path) => {
      const file = join(netRoot, "src", path);
      return { path: relative(netRoot, file), source: await Bun.file(file).text() };
    }),
);

const problems = [
  ...checkCrateEdges(metadata),
  ...checkNetImpls(netSources),
  ...checkReleaseProfile(await Bun.file(join(root, "Cargo.toml")).text()),
];

for (const problem of problems) {
  console.error(problem);
}
if (problems.length > 0) {
  process.exit(1);
}
console.log("architecture rules: ok");
