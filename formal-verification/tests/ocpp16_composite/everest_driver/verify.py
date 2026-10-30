#!/usr/bin/env python3
"""Check the CLI boundary and the copied profile-selection glue against pinned behaviour."""
import copy
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parent


def example(name):
    return json.loads((ROOT / 'examples' / (name + '.json')).read_text())


def invoke(value):
    text = value if isinstance(value, str) else json.dumps(value)
    result = subprocess.run([str(ROOT / 'build/everest_composite')], input=text,
                            text=True, capture_output=True, timeout=10)
    assert result.stderr == '', result.stderr
    return result.returncode, json.loads(result.stdout)


class DriverChecks(unittest.TestCase):
    def run_case(self, value):
        status, output = invoke(value)
        self.assertEqual(status, 0, output)
        return output

    def limits(self, value):
        return [p['limit'] for p in self.run_case(value)['schedule']['chargingSchedulePeriod']]

    def test_daily_pinned_observation(self):
        # This records upstream's two-second discrepancy; it is NOT a spec pass.
        output = self.run_case(example('spec_3_13_7'))
        self.assertEqual([(p['startPeriod'], p['limit']) for p in
                          output['schedule']['chargingSchedulePeriod']],
                         [(0, 11000), (28798, 6000), (71998, 11000)])

    def test_daily_without_intervening_leap_seconds(self):
        case = example('spec_3_13_7')
        case['profiles'][0]['profile']['chargingSchedule']['startSchedule'] = case['start']
        output = self.run_case(case)
        self.assertEqual([p['startPeriod'] for p in output['schedule']['chargingSchedulePeriod']],
                         [0, 28800, 72000])

    def test_station_max_caps_tx(self):
        self.assertEqual(self.limits(example('cpmax_vs_tx')), [16])

    def test_defaults_and_raw_no_limit(self):
        case = example('cpmax_vs_tx')
        case['profiles'] = []
        for unit, limit in [('A', 48), ('W', 33120)]:
            case['unit'] = unit
            output = self.run_case(case)
            self.assertEqual(output['intermediate'],
                             [{'startPeriod': 0, 'current_limit': -1, 'power_limit': -1}])
            self.assertEqual(output['schedule']['chargingSchedulePeriod'],
                             [{'startPeriod': 0, 'limit': limit, 'numberPhases': None}])

    def test_inactive_session_ignores_tx_and_default(self):
        case = example('cpmax_vs_tx')
        case['session_start'] = None
        self.assertEqual(self.limits(case), [16])
        case['profiles'] = case['profiles'][1:]
        self.assertEqual(self.limits(case), [48])
        case['profiles'][0]['profile']['chargingProfilePurpose'] = 'TxDefaultProfile'
        self.assertEqual(self.limits(case), [48])

    def test_tx_overrides_default_and_other_connectors_ignored(self):
        case = example('cpmax_vs_tx')
        case['profiles'] = case['profiles'][1:]
        default = copy.deepcopy(case['profiles'][0])
        default['connectorId'] = 0
        default['profile']['chargingProfilePurpose'] = 'TxDefaultProfile'
        default['profile']['chargingSchedule']['chargingSchedulePeriod'][0]['limit'] = 8
        case['profiles'].append(default)
        self.assertEqual(self.limits(case), [32])
        case['profiles'][0]['connectorId'] = 2
        self.assertEqual(self.limits(case), [8])

    def test_stack_priority_and_replacement_in_input_order(self):
        case = example('cpmax_vs_tx')
        case['profiles'] = case['profiles'][1:]
        higher = copy.deepcopy(case['profiles'][0])
        higher['profile']['stackLevel'] = 1
        higher['profile']['chargingSchedule']['chargingSchedulePeriod'][0]['limit'] = 12
        case['profiles'].append(higher)
        self.assertEqual(self.limits(case), [12])
        case['profiles'].append(copy.deepcopy(higher))
        case['profiles'][-1]['profile']['chargingSchedule']['chargingSchedulePeriod'][0]['limit'] = 7
        self.assertEqual(self.limits(case), [7])

    def test_transaction_id_has_no_matching_session_id(self):
        case = example('cpmax_vs_tx')
        case['profiles'] = case['profiles'][1:]
        case['profiles'][0]['profile']['transactionId'] = 123
        self.assertEqual(self.limits(case), [48])

    def test_invalid_inputs(self):
        valid = example('cpmax_vs_tx')
        cases = ['{', '{} {}', 'null', {}]
        for field, value in [('unit', 'kW'), ('connector', 0), ('connector', 1.5),
                             ('connector', 2**64), ('start', '2026-01-01'),
                             ('start', '2026-02-30T00:00:00Z'),
                             ('end', '2025-01-01T00:00:00Z'), ('profiles', None)]:
            case = copy.deepcopy(valid)
            case[field] = value
            cases.append(case)
        for field, value in [('chargingSchedulePeriod', []), ('duration', -1),
                             ('chargingRateUnit', 'kW')]:
            case = copy.deepcopy(valid)
            case['profiles'][0]['profile']['chargingSchedule'][field] = value
            cases.append(case)
        for value in cases:
            with self.subTest(value=value):
                status, output = invoke(value)
                self.assertEqual(status, 2, output)
                self.assertEqual(set(output), {'error'})
                self.assertIsInstance(output['error'], str)


if __name__ == '__main__':
    unittest.main()
