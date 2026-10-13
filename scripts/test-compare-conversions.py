# SPDX-License-Identifier: AGPL-3.0-only
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('compare', Path(__file__).with_name('compare-conversion-reports.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CompareTests(unittest.TestCase):
    def test_exact_comparison_detects_both_gain_and_regression(self):
        baseline = dict(dataset_sha256='data', model_sha256='model', dictionary_sha256='base', partition='train', metrics=[], rows=[
            dict(input='one', context='a', reference='A', modes=[{}, {}, dict(correct=False, reference_in_pool=False, first='wrong')]),
            dict(input='two', context='b', reference='B', modes=[{}, {}, dict(correct=True, reference_in_pool=True, first='B')])])
        after = copy.deepcopy(baseline)
        after['rows'][0]['modes'][2] = dict(correct=True, reference_in_pool=True, first='A')
        after['rows'][1]['modes'][2] = dict(correct=False, reference_in_pool=False, first='other')
        result = module.compare(baseline, after)
        self.assertEqual([row['input'] for row in result['improved']], ['one'])
        self.assertEqual([row['input'] for row in result['regressed']], ['two'])
        self.assertEqual(len(result['pool_gained']), 1)
        self.assertEqual(len(result['pool_lost']), 1)
        for key in ('dataset_sha256', 'model_sha256', 'dictionary_sha256', 'partition'):
            changed = copy.deepcopy(after); changed[key] = 'changed'
            with self.assertRaises(ValueError): module.compare(baseline, changed)
        changed = copy.deepcopy(after); changed['rows'][0]['context'] = 'different'
        with self.assertRaises(ValueError): module.compare(baseline, changed)


if __name__ == '__main__':
    unittest.main()
