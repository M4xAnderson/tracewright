import { describe, expect, test } from "bun:test";
import { checkNetImpls } from "./net-impls.ts";

const message = (at: string) =>
  `${at}: impl block in tracewright-net outside encode and validate (ADR 0009)`;

describe("checkNetImpls", () => {
  test("accepts data types with derives", () => {
    const source = [
      "#[derive(Debug, Clone, PartialEq, Eq)]",
      "pub struct Marking(Vec<u32>);",
      "pub enum NetVerdict { Proved, Disproved, Unknown }",
    ].join("\n");
    expect(checkNetImpls([{ path: "src/lib.rs", source }])).toEqual([]);
  });

  test.each(["src/encode.rs", "src/validate.rs", "src/encode/canonical.rs", "src/validate/mod.rs"])(
    "accepts impl blocks in %s",
    (path) => {
      const source = "impl Net {\n    pub fn canonical(&self) -> Vec<u8> { todo() }\n}\n";
      expect(checkNetImpls([{ path, source }])).toEqual([]);
    },
  );

  test.each([
    ["an inherent impl", "impl Net {\n}"],
    ["a trait impl", "impl Display for Marking {\n}"],
    ["a generic impl", "impl<T: Ord> From<T> for Marking {\n}"],
    ["an indented impl in a module", "mod firing {\n    impl Net {}\n}"],
    ["an unsafe impl", "unsafe impl Send for Net {}"],
  ])("rejects %s outside encode and validate", (_name, source) => {
    const line = source.split("\n").findIndex((l) => l.includes("impl")) + 1;
    expect(checkNetImpls([{ path: "src/lib.rs", source }])).toEqual([message(`src/lib.rs:${line}`)]);
  });

  test("ignores impl in comments, strings and impl Trait types", () => {
    const source = [
      "// impl Net { fn fire() } is forbidden here",
      "/* impl Net {}",
      "   impl Net {} */",
      'const NOTE: &str = "impl Net {}";',
      "pub fn ids(net: &Net) -> impl Iterator<Item = u32> + '_ { net.ids() }",
    ].join("\n");
    expect(checkNetImpls([{ path: "src/lib.rs", source }])).toEqual([]);
  });

  test("reports every impl with its line", () => {
    const source = "pub struct Net;\nimpl Net {}\n\nimpl Default for Net {}\n";
    expect(checkNetImpls([{ path: "src/model.rs", source }])).toEqual([
      message("src/model.rs:2"),
      message("src/model.rs:4"),
    ]);
  });
});
