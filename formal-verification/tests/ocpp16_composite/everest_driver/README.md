# EVerest OCPP 1.6 composite-schedule driver (S1)

Builds successfully as a standalone CLI. The CPMax example passes. The section
3.13.7 example has the requested limits but **does not meet the expected transition
times**: unchanged EVerest returns 28798 and 71998, two seconds early. This is an
observed discrepancy, preserved for the later differential harness.

Implements [S1](../../../../intent/2026-10-24-ocpp-smart-charging-model-plan.md).
The local [spec](spec.md) records the CLI boundary and assumptions.

## Source and build

Pinned source: [EVerest/EVerest dd696165d407d461a724232a4d77fe476a964ef1](https://github.com/EVerest/EVerest/tree/dd696165d407d461a724232a4d77fe476a964ef1).
`build.sh` downloads that commit's codeload tarball using curl. It does not require
or use the supplied read-only checkout. All four compiled EVerest translation
units and the inspected smart-charging implementation/header were compared
byte-for-byte with that checkout and matched.

From the repository root:

```sh
cd formal-verification/tests/ocpp16_composite/everest_driver
./build.sh
./build/everest_composite < examples/spec_3_13_7.json
./build/everest_composite < examples/cpmax_vs_tx.json
python3 verify.py
```

Downloads are cached in `.deps/`; objects, executable and compiler scratch files
are in `build/`. Both directories are ignored. Re-running `build.sh` reuses the
downloads and rebuilds the executable. It can be invoked from any working directory.
No CMake, git, database, charger service or network connection at runtime is needed.
Set `CXX` to choose another C++17 compiler.

Dependencies:

- nlohmann/json **3.11.3**, `single_include/nlohmann/json.hpp` and `json_fwd.hpp`,
  fetched individually with curl from the upstream tag.
- HowardHinnant/date **3.0.1**, fetched as its codeload release tarball. Uses real
  `date.h`, `tz.h`, `tz_private.h` and `src/tz.cpp`, compiled with
  `USE_OS_TZDB=1` and `HAS_REMOTE_API=0`. The OS timezone/leap-second database is a
  runtime dependency and can affect historical recurring profiles.
- Existing Homebrew headers under `/opt/homebrew/include`: Boost **1.92.0** and
  fmt **12.2.0** on this machine; fmt is configured header-only. No Boost/fmt
  libraries need linking.
- Tested with Apple clang **21.0.0**, C++17, macOS arm64; Python 3 for checks only.
  Date 3.0.1 emits deprecated literal-operator spelling warnings on this compiler.

The complete, unmodified upstream translation units compiled are:
`lib/ocpp/v16/profile.cpp`, `lib/ocpp/v16/ocpp_types.cpp`,
`lib/ocpp/v16/ocpp_enums.cpp`, and `lib/ocpp/common/types.cpp`, all beneath
`lib/everest/ocpp/`. Function/data sections and linker dead stripping remove unused
serialization/type code, avoiding unrelated runtime dependencies.

## CLI contract

One JSON object on stdin, one compact JSON object plus newline on stdout. Success
exits 0. Malformed JSON, fields, units, UTC timestamps, windows or periods exit 2
with `{"error":"..."}` on stdout. Runtime logging is suppressed. Examples contain
the exact input shape. `start <= end` and an int32 duration are required. Input
UTC timestamps can end in `Z` or `+00:00`; optional fractional seconds are accepted.
`startSchedule` is floored to seconds and serialized by upstream DateTime, including
`.000Z`. Nonempty schedule periods must start at 0 and strictly increase; limits
must be finite and nonnegative and explicit phase counts must be 1..3.

Output `intermediate` exposes the merged raw current and power constraints before
unit conversion/default filling; `-1` means no limit in that unit. `schedule`
contains only `startSchedule`, `duration`, `chargingRateUnit` and
`chargingSchedulePeriod`; absent `numberPhases` is explicitly `null`. The enhanced
upstream metadata is deliberately omitted by the JSON adapter.

Online operation only, target `evse_id = connector >= 1`, with
`simulate_transaction_active = (session_start != null)`. A supplied session start
stands in for the transaction's energy-meter start timestamp. There is **no
transaction ID in this contract**: ID-bound TxProfiles are omitted because they
cannot match a known transaction ID. Profiles without transactionId apply normally.
There is no profile-validation/acceptance service or installation database: this
adapter installs the supplied well-shaped profiles directly. Profiles for other
connectors are ignored. ChargePointMaxProfiles must have connectorId 0; TxProfiles
must have a positive connectorId. Profiles are installed in array order, so the
last profile at the same purpose/stack level replaces earlier profiles.

## Re-implemented glue and shims

All line citations below refer to `lib/everest/ocpp/` at the pinned commit; the
same citations appear in `main.cpp`. No upstream source is modified.

| Copied glue | Upstream location | Driver treatment |
| --- | --- | --- |
| Profile installation maps | `lib/ocpp/v16/smart_charging.cpp:417-452` | Same purpose/stack assignment and copying connector-0 TxDefaultProfiles to the target connector; no locks or DB writes in a one-shot process. |
| `get_valid_profiles` | `lib/ocpp/v16/smart_charging.cpp:598-661` | CPMax profiles included for station and connector, then matching unbound Tx and TxDefault profiles in stack order; no ignored purposes while online. Upstream does not use its start/end arguments. |
| `generate_evse_intermediates` | `lib/ocpp/v16/smart_charging.cpp:184-215` | Append station profiles, expand TxDefault and Tx only when a transaction is active/simulated, then prefer Tx over TxDefault. |
| `CompositeScheduleConfig` defaults | `lib/ocpp/v16/smart_charging.cpp:155-182` | 48 A, 33120 W, 3 phases, 230 V using upstream constants; no configuration overrides. |
| Single-connector `calculate_enhanced_composite_schedule` orchestration | `lib/ocpp/v16/smart_charging.cpp:245-318` | Generate connector intermediate, add station CPMax, merge lowest limits, floor start/duration, convert with defaults. No station-total branch. |

All actual expansion/merging/conversion calls use real `profile.cpp`:
`calculate_all_profiles`, `generate_profile_from_periods`,
`merge_tx_profile_with_tx_default_profile`, `merge_profiles_by_lowest_limit`,
`convert_intermediate_into_schedule`, `floor_seconds`, `elapsed_seconds`. JSON profile parsing uses
real `ocpp_types.cpp`; DateTime parsing/formatting uses real common `types.cpp`.

The include path puts `shim/` first:

- `shim/everest/logging.hpp`: `EVLOG_*` are no-op stream sinks.
- `shim/ocpp/v16/smart_charging.hpp`: imports the real OCPP types and enhanced
  period type, and repeats only the five constants from upstream
  `include/ocpp/v16/smart_charging.hpp:19-23`. Avoids database/connector/transaction
  headers; contains no scheduling algorithm. Default A/W limits and voltage come
  directly from the real `include/ocpp/common/constants.hpp:18-21`.

## Verification, observed 2026-10-24

Commands below run from this directory. Both exit 0 and write no stderr.

```sh
./build/everest_composite < examples/spec_3_13_7.json
```

```json
{"intermediate":[{"current_limit":-1.0,"power_limit":11000.0,"startPeriod":0},{"current_limit":-1.0,"power_limit":6000.0,"startPeriod":28798},{"current_limit":-1.0,"power_limit":11000.0,"startPeriod":71998}],"schedule":{"chargingRateUnit":"W","chargingSchedulePeriod":[{"limit":11000.0,"numberPhases":3,"startPeriod":0},{"limit":6000.0,"numberPhases":3,"startPeriod":28798},{"limit":11000.0,"numberPhases":3,"startPeriod":71998}],"duration":86400,"startSchedule":"2026-01-01T00:00:00.000Z"}}
```

Expected boundaries were 0, 28800, 72000; observed boundaries are 0, 28798, 71998.
The limit values match. In `lib/ocpp/v16/profile.cpp:175-235`, daily recurrence
subtracts `date::utc_clock` instants and takes elapsed seconds modulo 86400. There
are two leap seconds between the 2013 anchor and the 2026 window. This explains
the observed two-second shift; no compensation is applied in the driver. The
focused check changing only startSchedule to the 2026 window start yields exact
0, 28800, 72000 boundaries. This is evidence of pinned behaviour, not a passing
check against the section 3.13.7 timing expectation.

Two controls change only the startSchedule and show one second of drift per leap
second between the anchor and the window.

| Input | startSchedule | Leap seconds since | 08:00 boundary |
| --- | --- | --- | --- |
| `examples/spec_3_13_7.json` | 2013-01-01 | 2 | 28798 |
| `examples/leap_control_2016.json` | 2016-07-01 | 1 | 28799 |
| `examples/leap_control_2020.json` | 2020-01-01 | 0 | 28800 |

```sh
./build/everest_composite < examples/cpmax_vs_tx.json
```

```json
{"intermediate":[{"current_limit":16.0,"power_limit":-1.0,"startPeriod":0}],"schedule":{"chargingRateUnit":"A","chargingSchedulePeriod":[{"limit":16.0,"numberPhases":null,"startPeriod":0}],"duration":86400,"startSchedule":"2026-01-01T00:00:00.000Z"}}
```

Matches the expected 16 A throughout. Upstream leaves `numberPhases` unset even
though it uses 3 as the default for conversions, so the serialized field is null.

```sh
python3 verify.py
```

```text
.........
----------------------------------------------------------------------
Ran 9 tests in 0.168s

OK
```

Checks both examples (explicitly recording the daily discrepancy), the daily
anchor control, no-profile raw/default limits in A and W, inactive sessions,
station-default copying, Tx precedence, unrelated connectors, stack priority,
last-write replacement, ID-bound Tx filtering, and malformed input/error exits.
A second `./build.sh` succeeds using cached downloads. These are focused adapter
checks; no claim of full OCPP conformance or comparison with a Lean model is made.
