# T013: Temporal Campaign

## Goal

Bring up the `Temporal` proposal end to end so the dominant remaining
conformance gap — `test/built-ins/Temporal` (~4,600 cases that QuickJS-NG
passes) — turns from structurally-failing into pass/fail signal. Land the
shared internals (options parsing, ISO date-time records, balancing/constrain,
rounding) once, then build each Temporal type on top of it.

## Scope decisions

- **iso8601 calendar only** initially. Non-ISO calendars (Gregorian, Hebrew,
  Islamic, ...) and the full calendar method surface are explicitly out of
  scope until every other slice is green.
- **UTC and fixed numeric offset time zones only** for `ZonedDateTime` /
  `TimeZone` first; the full IANA tz database (named zones, DST transitions) is
  deferred until everything else is green.
- Date and nanosecond math is **hand-rolled on i64 / i128** day and
  nanosecond arithmetic — no new dependencies, no `chrono`-style crate.
- All Temporal internals live in `crates/qjs-runtime/src/temporal/`, one
  submodule per Temporal type plus a shared internals module, so the
  file-size guard stays satisfied as the surface grows.

## Evidence

`find third_party/test262/test/built-ins/Temporal -name '*.js'` counts at
campaign start (4,603 total `*.js`):

| Type | Cases |
| --- | ---: |
| ZonedDateTime | 901 |
| PlainDateTime | 773 |
| PlainDate | 652 |
| Duration | 540 |
| PlainYearMonth | 509 |
| PlainTime | 493 |
| Instant | 465 |
| PlainMonthDay | 199 |
| Now | 66 |
| toStringTag / prop-desc / keys / getOwnPropertyNames | 5 |

QuickJS-NG (`third_party/quickjs-ng`, read-only) implements Temporal and is the
reference for semantics where the spec text is ambiguous.

## Slices

Sliced by Temporal type with the dependency order that lets each type reuse the
prior one's records and helpers.

- [ ] S1 Foundations: the `Temporal` namespace object
      (`Symbol.toStringTag` "Temporal", constructors installed as they land),
      and the shared internals module `crates/qjs-runtime/src/temporal/`:
      options parsing (`GetOption`, `GetRoundingModeOption`, `ToTemporalOverflow`,
      `GetTemporalUnit`, `GetRoundingIncrement`, ...), the ISO date-time record
      struct + validation (`IsValidISODate`, `RejectTime`, ...),
      balancing/constrain helpers, and a `RoundingMode` enum + rounding helpers.
      Prove the plumbing with one visible surface: `Temporal.PlainTime`
      (constructor + getters + `from` + `with` + `add`/`subtract` + `toString`
      + `equals`/`compare` if it fits the budget; otherwise constructor +
      getters + `from` + `toString`, ticked as partial). Focused unit tests per
      helper family.
- [ ] S2 Duration: `Temporal.Duration` constructor, getters, `from`, `with`,
      `negated`/`abs`, `add`/`subtract`, `round`, `total`, `toString`,
      `compare`. Balancing across units.
- [ ] S3 Instant: `Temporal.Instant`, epoch conversions
      (`fromEpochMilliseconds`/`Nanoseconds`, `epochMilliseconds`/`Nanoseconds`),
      `from`, `add`/`subtract`, `round`, `until`/`since`, `toString`,
      `equals`/`compare`. i128 nanosecond epoch.
- [ ] S4 PlainTime: full surface if S1 only landed the partial set
      (`add`/`subtract`/`round`/`until`/`since`/`with`/`equals`/`compare`).
- [ ] S5 PlainDate (iso8601 calendar only) + `PlainMonthDay` + `PlainYearMonth`:
      constructors, getters (year/month/day/dayOfWeek/dayOfYear/...), `from`,
      `with`, `add`/`subtract`, `until`/`since`, `toString`, `equals`/`compare`.
- [ ] S6 PlainDateTime: composition of PlainDate + PlainTime records, the full
      method surface, `round`, `until`/`since`.
- [ ] S7 ZonedDateTime + TimeZone (UTC and fixed-offset only): wall-clock <->
      epoch conversion against a fixed offset, the method surface, `toInstant`,
      `toPlainDate`/`toPlainTime`/`toPlainDateTime`.
- [ ] S8 `Temporal.Now` (`instant`, `timeZoneId`, `plainDateTimeISO`,
      `plainDateISO`, `plainTimeISO`, `zonedDateTimeISO`) + residual calendar
      surface needed to clear remaining ISO-calendar cases.

## Scope (paths / ownership)

- Allowed paths: `crates/qjs-runtime/**` (engine semantics), plus
  `tasks/T013-temporal-campaign.md` and `tasks/README.md` for bookkeeping.
- S-slices that need Test262 wiring may touch `tests/test262/**` allowlists.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; S1 precedes everything (shared
  internals); S3 reuses S2's rounding; S6 reuses S4+S5 records; S7 reuses S6;
  S8 reuses S3 + S6/S7.

## References

- `docs/architecture.md`; `crates/qjs-runtime/src/date/` and
  `crates/qjs-runtime/src/math.rs` as namespace/constructor install patterns.
- ECMAScript Temporal proposal spec (Abstract Operations: `GetOption`,
  `RoundNumberToIncrement`, `BalanceTime`, `RegulateISODate`, ...).
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` Temporal implementation.
- Test262: `test/built-ins/Temporal/**`, `harness/temporalHelpers.js`.

## Acceptance Criteria

- S1: `Temporal` exists as a namespace object with `Symbol.toStringTag`
  "Temporal"; `Temporal.PlainTime` constructs, exposes getters, round-trips
  through `from`/`toString`; options/rounding/ISO-record helpers have unit
  coverage; non-Temporal items are unchanged.
- Each later slice: the type constructs, its methods round-trip, and the
  matching `test/built-ins/Temporal/<Type>` probe rises measurably.
- Campaign exit: `test/built-ins/Temporal` shows real pass signal in the
  burndown series; non-ISO calendars and full IANA zones remain documented
  out-of-scope follow-ups.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/built-ins/Temporal --all
./scripts/check.sh
./scripts/compare-qjs.sh
```
