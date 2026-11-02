# E5 paper test: do real incidents fit bounded P/T nets? (assumption A1)

As of 2026-11-02. Experiment E5 from `../opportunity-solution-tree.md`. Tests A1: real deadlocks and overflows between components can be written as bounded P/T nets under the v1 rules (capacity on every place, ports, sync = fused transitions, async = bag place with capacity; no order, no request/reply matching, no time, no data; built-in properties or a linear "never" / "always reachable" predicate).

**Method.** Three public post-mortems, all primary sources (the company's own report). For each one: 3–6 elements, one net per element with ports, bindings, and a property. I modeled the design before the fix and after it, then ran a throwaway explicit-state explorer: `e5/explore.py`, Python 3 stdlib only, BFS over markings, shortest counterexample with each step mapped back to its elements. Nets: `e5/case*.toml`. Full output: `e5/results.txt`. To rerun: `python3 -I explore.py case*.toml` in `e5/`.

**Explorer semantics.** Going over a capacity is a *violation*, not blocking. Markings over a capacity are not expanded, so every other property is at best Unknown in that net. Blocking backpressure is modeled with complement places. "Always reachable P" means that from every reachable marking, a P-marking can be reached (AG EF). `deadlock_free` takes a *final predicate* for proper termination. Parameters are scaled down (pools of 2–4, fleets of 3–6).

**Sample.** All three are AWS reports, because AWS publishes the most detailed public post-mortems. I also read Slack's 2-22-22 report (https://slack.engineering/slacks-incident-on-2-22-22/) but did not model it (see "Implications"). No synthetic cases were needed.

## Case 1: lock/resource starvation. AWS EBS, us-east-1, 2011-04-21

- **Source:** https://aws.amazon.com/message/65648/. "These slow API calls began to back up and resulted in thread starvation in the EBS control plane". The pool was regional, so API calls failed in other AZs too.
- **Architecture (Container):** `api_a` (callers in degraded AZ-A) and `api_b` (callers in healthy AZ-B) call (sync) → `control_plane` (regional, one pool of 3 threads) → calls (sync) `cluster_a` (degraded) / `cluster_b`; the clusters respond (sync) back to `control_plane`.
- **Net summary:** callers have `pending → waiting → done`. The control plane has `threads_free(3)`, plus per-AZ `in/wait/ret` places with transition ports `accept_x`, `fwd_x`, `done_x`, `ret_x`. Each cluster has `create` / `respond`, where `respond` reads `healthy`, which starts at 0 for `cluster_a`. "Slow" is abstracted as "never responds". 8 bindings, all sync.
- **Properties:** P1 `deadlock_free`, final `api_b/done = 2`. P4 `never control_plane/in_a + wait_a + ret_a >= 3` ("AZ-A holds the whole pool").
- **Before:** P1 **Disproved**, 6 steps, 84 states, 0.5 ms. Three AZ-A calls take all threads and are forwarded to `cluster_a`. Then `api_b/call + control_plane/accept_b` is blocked, waiting for `control_plane/threads_free`. That is the incident's root cause, almost word for word.
- **Fix (as in the post-mortem: "be more Availability Zone aware and shed load"):** a per-AZ limit (`a_slots = 2`) plus `shed_a`. "AZ-A at its limit" is a zero test, written as a weight-2 read arc on the complement place `a_used`. Result: P1, P3 (`always_reachable api_b/done = 2`), P4 and no-capacity-exceeded are all **Proved**, 99 states, under 1 ms each.
- **Lost:** time. In `case1_ebs_timeout.toml` the control plane *has* a timeout on AZ-A calls. The real timeout was long, but a long and a short timeout give the same untimed net. P1 and P3 then come out **Proved**: the timeout always fires eventually, so the defect is hidden. Only P4, the linear "pool share" predicate, still ends **Disproved** (3 steps). So the loss of time hides the defect from deadlock and liveness properties but not from a well-chosen linear safety predicate. Also lost: the 13 % stuck volumes and the race condition (data, rates).

## Case 2: resource limit overflow + blocking buffer. AWS Kinesis, us-east-1, 2020-11-25

- **Source:** https://aws.amazon.com/message/11201/. Each front-end server "creates operating system threads for each of the other servers". After a small capacity addition, "all of the servers in the fleet [exceeded] the maximum number of threads allowed by an operating system configuration". Cognito webservers had a latent bug and blocked "on backlogged Kinesis buffers".
- **Architecture (Container), two property scopes:** (a) `membership` publishes peers (async bag, cap 8) to `fe1`; `fe_peers` sends shard info (async bag, cap 8) to `fe1`. (b) `users` log in (sync) through `cognito_web`, which has a 2-record buffer and flushes (sync) to `kinesis_api`. `kinesis_api` can `fail` for good.
- **Net summary:** `fe1/peer_threads` has capacity 4, standing in for the OS thread limit. `open_peer_thread` fires once per published peer. Fleet size is the initial marking: 5 peers after the scale-up. In Cognito, `auth` consumes `buf_free`, which is a blocking put.
- **Properties:** P1 `no_capacity_exceeded` (scope a). P2 `always_reachable fe1/map_entries = 5` (shard map can complete). P3 `deadlock_free`, final `users/signed_in = 4` (scope b).
- **Before:** P1 **Disproved**, 10 steps, 330 states, 1 ms: five `publish` / `open_peer_thread` pairs, then `fe1/peer_threads = 5 > 4`. P2 is Unknown, because the capacity is exceeded first. P3 **Disproved**, 3 steps, 24 states: `kinesis_api/fail`, then two logins fill the buffer, and `login + auth` is blocked waiting for `cognito_web/buf_free`. Both match the report's root causes. Adding servers shows up as a change of the initial marking.
- **Fix (as in the post-mortem):** "larger CPU and memory servers, reducing the total number of servers", so 3 peers; and harden Cognito against Kinesis API errors (`discard` fused with `kinesis_api/error`). P1, P2 and P3 are all **Proved**: 76 / 76 / 24 states, under 1 ms.
- **Lost:** almost nothing that matters. Peer identity is folded into a count: one thread could take every peer's shard info. That does not matter here. The real limit is in the thousands. The pure count stays tiny, but explicit interleaving of large folded populations will not.

## Case 3: retry amplification / metastable failure. Amazon DynamoDB, us-east-1, 2015-09-20

- **Source:** https://aws.amazon.com/message/5467D2/. Metadata responses "exceeded the retrieval and transmission time allowed by storage servers"; "Unavailable servers continued to retry requests for membership data, maintaining high load on the metadata service."
- **Architecture (Container):** `network` (repeatable blip or check-in) → sync `storage/lose`. `storage` (3 servers, folded) sends `request` over the async bag `membership_requests` (cap 3) → `metadata` (2 workers) responds over the async bag `membership_replies` (cap 3) → `storage/inbox`.
- **Net summary:** storage moves `serving → need → waiting`. `timeout` is a nondeterministic `waiting → need`, followed by a fresh `request`. The abandoned request stays in the metadata service (no cancellation). `apply` matches *any* reply to *any* waiting server. `drop_stale` absorbs late replies.
- **Properties:** P1 `no_capacity_exceeded`. P2 `always_reachable storage/serving = 3` (the fleet can recover).
- **Before:** P1 **Disproved**, 8 steps, 1,919 states, 13 ms: three servers lose membership and request, one times out and requests again, and `bag:membership_requests = 4 > 3`. A developer would recognize "retries pile on top of unfinished originals". But the trace does it with idle workers (`workers_free = 2`). Without rates, "the service is slower than the timeout" is just "has not fired yet".
- **Fix as in the post-mortem:** more capacity (4 workers, bags of 6), plus "reducing the rate … and lengthening the time allowed". The rate and time parts give the same untimed net. P1 stays **Disproved**: 14 steps, 17,149 states, 130 ms. **v1 cannot show that AWS's own fix works.** Any finite capacity is overrun by unbudgeted retries.
- **Fix that v1 can prove (ours, not the report's):** a retry budget of 2 outstanding attempts per server (`attempts_free = 6`). The token returns only when a reply is applied or dropped. P1 and P2 are **Proved**: 1,970 states, 12–19 ms. Scaling the fleet to 4 / 5 / 6 servers takes 6,755 / 18,145 / 41,385 states (0.05 / 0.14 / 0.36 s), roughly 2.5× per server. The P-invariant `attempts_free + bags + busy + replies_in = 2K` would prove the bound for any K.
- **Lost, and it matters:** rates and time. The *metastable* part ("does not recover after the trigger is gone") cannot be expressed. P2 is **Proved** on the fixed net, but it only means that recovery is *possible under some schedule*. A budgeted system whose service time exceeds the timeout can still stay stuck forever. Read as "the fleet will recover", that Proved would mislead. Also lost: membership *size* growing with GSIs, which is data that drives service time. Request/reply matching was lost too: a stale reply can satisfy another server. That over-approximates and did not hide the defect. Message order never mattered.

## Summary

| Case | Defect reproduced as Disproved? | Post-mortem fix Proved? | Lost information hides the defect? |
|---|---|---|---|
| 1 EBS thread starvation | Yes: deadlock_free and pool-share predicate | Yes (bulkhead + shedding) | With a timeout: yes for deadlock/liveness, **no** for the linear predicate |
| 2 Kinesis thread limit + Cognito buffer | Yes, both (overflow and blocked buffer) | Yes | No |
| 3 DynamoDB retry storm | Yes, the amplification (bag overflow) | **No**: rate/time fix not expressible; only a retry budget is provable | Yes for metastability (non-recovery); no for amplification |

**Decision rule (OST, set before the run):** ≥ 2 of 3 reproduced ⇒ A1 holds. **Outcome: 3 of 3 reproduced, so A1 holds and the v1 bet stands.** Case 2 fits cleanly. Case 1 fits once the modeler abstracts "slow" to "stuck", or uses a share-of-pool predicate. Case 3 fits only for amplification, not for metastability.

**Threats to validity:** the sample is small, all AWS, and picked by me. The models were written with the root cause already known, so the abstractions are hindsight-friendly (for example, "never responds"). Parameters are scaled down. The explorer is untested throwaway code, not cross-checked against TAPAAL (that is E1). The user's own incidents may lean more toward timing than these three.

## Implications

**Property language (ADR 0003)**
1. **Share-of-resource predicates belong in v1.** "Never does one caller hold all of pool P" (`Σ held_by_X ≥ cap(P)`) caught case 1 even with a timeout, where deadlock-freedom and "always reachable" were fooled. Offer this as a property template next to the built-ins.
2. **Final markings must be a predicate, not a list of markings.** Partial degradation (AZ-A stuck by design, B must finish) needed `final = "api_b/done = 2"`.
3. **Decide the capacity semantics explicitly.** Here, capacity is an assertion and exploration stops at the first overflow, so other properties end Unknown ("capacity exceeded first"). The verdict panel should say so. Blocking backpressure needs complement places, and the pattern library (S5) should offer "bounded buffer, blocking", "bounded buffer, shed when full" and "retry with budget".
4. **Zero tests are free in bounded nets.** "When full or at its limit" becomes a read arc of weight `cap` on the complement place. The editor should generate this ("shed when full") rather than tempting users into nondeterministic over-approximations.
5. **Word "always reachable" carefully.** Use "can always get back to …", never "will recover". With no rates, a Proved here says nothing about overload. That is a wrong-Proved risk in the user's mind, not in the model.
6. Unbudgeted retry loops are Disproved at *any* load. That is correct (amplification is unbounded) but may feel like noise. Explain the verdict as "retries can outnumber capacity" and suggest a retry budget.

**Colored nets and time**
- **Colored nets: no evidence to move them earlier.** Order never mattered. Request/reply matching only added harmless over-approximation (case 3). Identity folding (case 2) did not matter. Keep ADR 0002's bag semantics for v1.
- **Time and rates are the real gap.** They hid case 1 under a long timeout and made case 3's metastability and its actual fix inexpressible. Slack 2-22-22 (cache hit rate feeding database load) would fall in the same bucket. Recommendation: no timed nets in v1. Instead, (a) document the "slow = never responds" abstraction and the share-of-pool template; (b) state in the product that overload and metastability questions are out of scope, so they do not end Proved by accident; (c) treat timed or stochastic analysis (or a simulation/queueing export) as the follow-up if E9 or the journal shows that timing questions dominate.
- **Scale:** folded populations grow about 2.5× per instance, so P-invariant certificates (S11) are what make real fleet sizes provable.
