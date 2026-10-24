# S1 driver contract

Intent: `intent/2026-10-24-ocpp-smart-charging-model.md`, S1 of its plan.

Implement the JSON stdin/stdout contract in the task: one connector, online,
transaction simulated exactly when session_start is non-null. Compile the pinned
EVerest profile.cpp unchanged and use its expansion, merge and conversion functions.
Expose intermediate limits before conversion/default filling. Invalid JSON or
malformed input exits 2 with a JSON error on stdout; success exits 0.

Profiles are installed in input order into maps keyed by purpose and stack level,
as in SmartChargingHandler. Connector 0 TxDefaultProfiles are copied to the target
connector. Ignore profiles for other connectors. No transaction ID is supplied by
this contract, so transaction-ID-bound TxProfiles cannot match and are omitted.
This limitation must be explicit in the README. Reject malformed schedule periods
before calling upstream code; do not alter calculation behaviour for valid profiles.

Download with curl into .deps; build only in build; ignore both. Use the upstream
default limits, phase count and voltage. Record the two specified examples and
their exact outputs. Check invalid input, defaults, inactive sessions and profile
selection in addition to the two examples. Do not run git or modify upstream files.
