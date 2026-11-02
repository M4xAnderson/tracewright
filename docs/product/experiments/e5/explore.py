#!/usr/bin/env python3
"""E5 throwaway explicit-state explorer for bounded P/T nets with ports.

Throwaway code for experiment E5 (paper test of assumption A1). Not product code.

Usage: python3 -I explore.py NET.toml [NET.toml ...]

Semantics (v1 rules, ADR 0002/0003):
- every place has a capacity; reaching more tokens than the capacity is a violation of
  "no capacity exceeded"; such markings are not expanded further (behavior beyond the
  capacity is undefined), so other properties can then be at best Unknown, never Proved;
- sync binding: caller transition fused with callee port transition (one copy per caller);
- async binding: generated bag place with a capacity between sender and receiver;
- properties: deadlock_free (optional final predicate), no_capacity_exceeded,
  no_dead_transitions, never P, always_reachable P (from every reachable marking a
  P-marking stays reachable), with P a linear marking predicate with and/or/not.
"""
import re
import sys
import time
import tomllib
from collections import deque

# ---------------------------------------------------------------- predicates
TOK = re.compile(r"\s*(?:(\d+)|([A-Za-z_][A-Za-z0-9_/]*)|(==|!=|>=|<=|=|<|>|\*|\+|-|\(|\)))")


def tokenize(s):
    out, pos = [], 0
    s = s.strip()
    while pos < len(s):
        m = TOK.match(s, pos)
        if not m:
            raise ValueError(f"bad predicate near: {s[pos:]!r}")
        num, ident, op = m.groups()
        out.append(("num", int(num)) if num else ("id", ident) if ident else ("op", op))
        pos = m.end()
    return out


def parse_pred(src, index):
    toks = tokenize(src)
    pos = [0]

    def peek():
        return toks[pos[0]] if pos[0] < len(toks) else (None, None)

    def take():
        t = peek()
        pos[0] += 1
        return t

    def lin():  # returns list of (coef, place_index), const
        terms, const, sign = [], 0, 1
        while True:
            k, v = take()
            coef = 1
            if k == "num":
                if peek() == ("op", "*"):
                    take()
                    coef = v
                    k, v = take()
                else:
                    const += sign * v
                    v = None
            if v is not None:
                if k != "id" or v not in index:
                    raise ValueError(f"unknown place {v!r} in {src!r}")
                terms.append((sign * coef, index[v]))
            if peek() in (("op", "+"), ("op", "-")):
                sign = 1 if take()[1] == "+" else -1
            else:
                return terms, const

    def atom():
        if peek() == ("op", "("):
            take()
            e = disj()
            take()
            return e
        if peek() == ("id", "not"):
            take()
            e = atom()
            return lambda m: not e(m)
        lt, lc = lin()
        _, op = take()
        rt, rc = lin()
        terms = lt + [(-c, i) for c, i in rt]
        k = rc - lc
        cmp = {"=": int.__eq__, "==": int.__eq__, "!=": int.__ne__, ">=": int.__ge__,
               "<=": int.__le__, "<": int.__lt__, ">": int.__gt__}[op]
        return lambda m: cmp(sum(c * m[i] for c, i in terms), k)

    def conj():
        e = atom()
        while peek() == ("id", "and"):
            take()
            a, b = e, atom()
            e = (lambda a, b: lambda m: a(m) and b(m))(a, b)
        return e

    def disj():
        e = conj()
        while peek() == ("id", "or"):
            take()
            a, b = e, conj()
            e = (lambda a, b: lambda m: a(m) or b(m))(a, b)
        return e

    e = disj()
    if pos[0] != len(toks):
        raise ValueError(f"trailing tokens in {src!r}")
    return e


# ---------------------------------------------------------------- flattening
def flatten(model, scope=None):
    elems = [e for e in model["element"] if scope is None or e["id"] in scope]
    eids = {e["id"] for e in elems}
    places, caps, init, owner = [], [], [], []
    trans = {}  # qualified id -> dict(inp, out, origin, port)
    for e in elems:
        extra = set(e) - {"id", "places", "transition"}
        if extra:
            raise ValueError(f"element {e['id']}: unexpected keys {extra} (TOML table scoping?)")
        for p, d in e.get("places", {}).items():
            places.append(f"{e['id']}/{p}")
            caps.append(d["cap"])
            init.append(d.get("init", 0))
            owner.append(e["id"])
        for t in e.get("transition", []):
            q = lambda d: {f"{e['id']}/{k}": v for k, v in d.items()}
            trans[f"{e['id']}/{t['id']}"] = dict(inp=q(t.get("in", {})), out=q(t.get("out", {})),
                                                 origin=[f"{e['id']}/{t['id']}"],
                                                 port=t.get("port", False), elem=[e["id"]])
    inside = lambda ref: ref.split("/")[0] in eids
    binds = [b for b in model.get("binding", []) if all(inside(b[k]) for k in b if k in
             ("caller", "callee", "sender", "receiver"))]
    for b in binds:
        for k in ("caller", "callee", "sender", "receiver"):
            if k in b and not trans[b[k]]["port"]:
                raise ValueError(f"{b[k]} is bound but not a port")
    # async: generated bag place
    for b in binds:
        if b["type"] != "async":
            continue
        name = f"bag:{b['name']}"
        places.append(name)
        caps.append(b["capacity"])
        init.append(0)
        owner.append(f"relationship {b['sender']} -> {b['receiver']}")
        trans[b["sender"]]["out"][name] = trans[b["sender"]]["out"].get(name, 0) + 1
        trans[b["receiver"]]["inp"][name] = trans[b["receiver"]]["inp"].get(name, 0) + 1
    # sync: fuse caller with callee, one copy per caller
    sync = [b for b in binds if b["type"] == "sync"]
    callers = {b["caller"] for b in sync}
    callees = {b["callee"] for b in sync}
    if callers & callees:
        raise ValueError(f"chained sync bindings not supported: {callers & callees}")
    for b in sync:
        a, c = trans[b["caller"]], trans[b["callee"]]
        add = lambda x, y: {k: x.get(k, 0) + y.get(k, 0) for k in set(x) | set(y)}
        trans[f"{b['caller']} + {b['callee']}"] = dict(
            inp=add(a["inp"], c["inp"]), out=add(a["out"], c["out"]),
            origin=a["origin"] + c["origin"], port=False, elem=a["elem"] + c["elem"])
    for t in callers | callees:
        del trans[t]
    idx = {p: i for i, p in enumerate(places)}
    tl = []
    for tid, t in sorted(trans.items()):
        tl.append((tid, tuple((idx[p], n) for p, n in t["inp"].items() if n),
                   tuple((idx[p], n) for p, n in t["out"].items() if n), t["elem"]))
    return places, caps, tuple(init), owner, tl, idx


# ---------------------------------------------------------------- exploration
def explore(places, caps, init, tl, budget):
    parent = {init: None}
    succ = {}
    over = []  # (marking, parent, tidx) capacity violations
    fired = set()
    q = deque([init])
    complete = True
    while q:
        m = q.popleft()
        out = []
        for ti, (_, inp, outp, _) in enumerate(tl):
            if all(m[i] >= n for i, n in inp):
                nm = list(m)
                for i, n in inp:
                    nm[i] -= n
                for i, n in outp:
                    nm[i] += n
                nm = tuple(nm)
                fired.add(ti)
                if any(nm[i] > caps[i] for i in range(len(nm))):
                    over.append((nm, m, ti))
                    continue
                out.append((ti, nm))
                if nm not in parent:
                    if len(parent) >= budget:
                        complete = False
                        continue
                    parent[nm] = (m, ti)
                    q.append(nm)
        succ[m] = out
    return parent, succ, over, fired, complete


def path_to(parent, m):
    steps = []
    while parent[m] is not None:
        m, ti = parent[m][0], parent[m][1]
        steps.append(ti)
    return steps[::-1]


def show(steps, tl, places, init, final_marking, note):
    lines = []
    m = list(init)
    for k, ti in enumerate(steps, 1):
        tid, inp, outp, elems = tl[ti]
        for i, n in inp:
            m[i] -= n
        for i, n in outp:
            m[i] += n
        lines.append(f"    {k:2d}. [{', '.join(dict.fromkeys(elems))}] fire {tid}")
    nz = ", ".join(f"{places[i]}={v}" for i, v in enumerate(final_marking) if v)
    lines.append(f"    => {note}: {nz}")
    return "\n".join(lines)


def check(path):
    with open(path, "rb") as f:
        model = tomllib.load(f)
    print(f"=== {model['model']['name']}  ({path.split('/')[-1]})")
    budget = model["model"].get("budget_states", 2_000_000)
    for prop in model["property"]:
        t0 = time.perf_counter()
        scope = prop.get("scope")
        places, caps, init, owner, tl, idx = flatten(model, scope)
        parent, succ, over, fired, complete = explore(places, caps, init, tl, budget)
        kind = prop["kind"]
        verdict, detail = None, ""
        undecided = (not complete and "budget exhausted") or \
                    (over and kind != "no_capacity_exceeded" and "capacity exceeded first; behavior beyond it undefined")
        if kind == "no_capacity_exceeded":
            if over:
                nm, pm, ti = over[0]
                steps = path_to(parent, pm) + [ti]
                bad = [places[i] for i in range(len(nm)) if nm[i] > caps[i]]
                verdict = "Disproved"
                detail = show(steps, tl, places, init, nm,
                              "capacity exceeded at " + ", ".join(f"{b} (cap {caps[idx[b]]})" for b in bad))
        elif kind == "deadlock_free":
            fin = parse_pred(prop["final"], idx) if "final" in prop else (lambda m: False)
            for m in parent:  # BFS insertion order = shortest first
                if not succ.get(m) and not fin(m) and not any(o[1] == m for o in over):
                    verdict = "Disproved"
                    detail = show(path_to(parent, m), tl, places, init, m, "dead marking, not final")
                    for tid, inp, _, _ in tl:  # explain: what each blocked step waits for
                        miss = [places[i] for i, n in inp if m[i] < n]
                        detail += f"\n       blocked: {tid}  waits for {', '.join(miss)}"
                    break
        elif kind == "never":
            p = parse_pred(prop["predicate"], idx)
            for m in parent:
                if p(m):
                    verdict = "Disproved"
                    detail = show(path_to(parent, m), tl, places, init, m, "predicate holds")
                    break
        elif kind == "always_reachable":
            p = parse_pred(prop["predicate"], idx)
            if not undecided:
                pred = {}
                for m, out in succ.items():
                    for _, nm in out:
                        pred.setdefault(nm, []).append(m)
                good = {m for m in parent if p(m)}
                q = deque(good)
                while q:
                    m = q.popleft()
                    for pm in pred.get(m, []):
                        if pm not in good:
                            good.add(pm)
                            q.append(pm)
                for m in parent:
                    if m not in good:
                        verdict = "Disproved"
                        detail = show(path_to(parent, m), tl, places, init, m,
                                      "from here the predicate is unreachable")
                        break
        elif kind == "no_dead_transitions":
            dead = [tl[i][0] for i in range(len(tl)) if i not in fired]
            if dead and not undecided:
                verdict = "Disproved"
                detail = "    never enabled: " + "; ".join(dead)
        if verdict is None:
            verdict = f"Unknown ({undecided})" if undecided else "Proved"
        ms = (time.perf_counter() - t0) * 1000
        label = prop.get("predicate") or prop.get("final") or ""
        print(f"  [{prop['id']}] {kind} {label!s:<.60}")
        print(f"      verdict: {verdict}   states: {len(parent)}   transitions: {len(tl)}"
              f"   time: {ms:.1f} ms")
        if detail:
            print(detail)
    print()


if __name__ == "__main__":
    for a in sys.argv[1:]:
        check(a)
