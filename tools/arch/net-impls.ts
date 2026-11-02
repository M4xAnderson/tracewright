// ADR 0009: `tracewright-net` holds data only. Behavior there would be shared by both explorers,
// so the only impl blocks allowed are the canonical encoding and structural validation. Derives
// are attributes, not impl blocks, so they pass.

/** A Rust source file of `crates/net`, with its path relative to the crate root. */
export interface SourceFile {
  path: string;
  source: string;
}

const ALLOWED = /^src\/(encode|validate)(\.rs|\/)/;
const IMPL_ITEM = /^\s*(?:unsafe\s+)?impl\b/;

/** Returns one `path:line` message per impl block outside the allowed modules. */
export function checkNetImpls(files: SourceFile[]): string[] {
  const problems: string[] = [];
  for (const file of files) {
    if (ALLOWED.test(file.path)) {
      continue;
    }
    blankCommentsAndLiterals(file.source)
      .split("\n")
      .forEach((line, index) => {
        if (IMPL_ITEM.test(line)) {
          problems.push(
            `${file.path}:${index + 1}: impl block in tracewright-net outside encode and validate (ADR 0009)`,
          );
        }
      });
  }
  return problems;
}

// Replaces comments, string literals and char literals with spaces and keeps every newline,
// so line numbers stay true and only code can match IMPL_ITEM.
function blankCommentsAndLiterals(source: string): string {
  const out = source.split("");
  const blank = (from: number, to: number) => {
    for (let i = from; i < to && i < out.length; i++) {
      if (out[i] !== "\n") {
        out[i] = " ";
      }
    }
  };
  let i = 0;
  while (i < source.length) {
    const rest = source.slice(i);
    if (rest.startsWith("//")) {
      const end = source.indexOf("\n", i);
      const stop = end === -1 ? source.length : end;
      blank(i, stop);
      i = stop;
    } else if (rest.startsWith("/*")) {
      const stop = blockCommentEnd(source, i);
      blank(i, stop);
      i = stop;
    } else if (/^b?r#*"/.test(rest)) {
      const hashes = (/^b?r(#*)"/.exec(rest) as RegExpExecArray)[1] as string;
      const close = source.indexOf(`"${hashes}`, i + rest.indexOf('"') + 1);
      const stop = close === -1 ? source.length : close + 1 + hashes.length;
      blank(i, stop);
      i = stop;
    } else if (rest.startsWith('"')) {
      let j = i + 1;
      while (j < source.length && source[j] !== '"') {
        j += source[j] === "\\" ? 2 : 1;
      }
      blank(i, j + 1);
      i = j + 1;
    } else if (rest.startsWith("'\\")) {
      const stop = source.indexOf("'", i + 3) + 1 || source.length;
      blank(i, stop);
      i = stop;
    } else if (rest.length >= 3 && rest[0] === "'" && rest[2] === "'") {
      blank(i, i + 3);
      i += 3;
    } else {
      i += 1;
    }
  }
  return out.join("");
}

// Rust block comments nest.
function blockCommentEnd(source: string, start: number): number {
  let depth = 0;
  let i = start;
  while (i < source.length) {
    if (source.startsWith("/*", i)) {
      depth += 1;
      i += 2;
    } else if (source.startsWith("*/", i)) {
      depth -= 1;
      i += 2;
      if (depth === 0) {
        return i;
      }
    } else {
      i += 1;
    }
  }
  return source.length;
}
