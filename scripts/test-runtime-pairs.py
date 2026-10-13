# SPDX-License-Identifier: AGPL-3.0-only
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
spec = importlib.util.spec_from_file_location('pairs', Path(__file__).with_name('prepare-runtime-pairs.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PairTests(unittest.TestCase):
    def test_same_reading_in_opposite_contexts_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'data.jsonl'
            rows = [dict(input='ambiguous', context='river', good='橋', bad='はし', split='train'),
                    dict(input='ambiguous', context='meal', good='箸', bad='はし', split='train')]
            path.write_text('\n'.join(json.dumps(r) for r in rows), encoding='utf-8')
            report = dict(partition='train', training_during_evaluation=False,
                dataset_sha256=hashlib.sha256(path.read_bytes()).hexdigest(), rows=[
                    dict(input='ambiguous', context='river', reference='橋', modes=[{}, {}, dict(correct=False, reference_in_pool=True, first='箸')]),
                    dict(input='ambiguous', context='meal', reference='箸', modes=[{}, {}, dict(correct=False, reference_in_pool=True, first='橋')])])
            result, count = module.prepare(path, report)
            self.assertEqual(count, 2)
            self.assertEqual([(r['context'], r['good'], r['bad']) for r in result[2:]], [('river', '橋', '箸'), ('meal', '箸', '橋')])
    def test_training_only_hash_protected_and_evaluation_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'data.jsonl'
            rows = [dict(input='trial', context='scene', good='正解', bad='旧誤答', split='train'), dict(input='heldout', context='other', good='保持', bad='誤り', split='test')]
            path.write_text('\n'.join(json.dumps(r) for r in rows), encoding='utf-8')
            case = dict(input='trial', context='scene', reference='正解', modes=[{}, {}, dict(correct=False, reference_in_pool=True, first='新誤答')])
            report = dict(partition='train', training_during_evaluation=False, dataset_sha256=hashlib.sha256(path.read_bytes()).hexdigest(), rows=[case, case])
            result, count = module.prepare(path, report)
            self.assertEqual(count, 1)
            self.assertEqual(result[1], rows[1])
            for key, value in [('partition', 'test'), ('dataset_sha256', 'invalid')]:
                with self.assertRaises(ValueError):
                    module.prepare(path, dict(report, **{key: value}))
            with self.assertRaises(ValueError):
                module.prepare(path, dict(report, rows=[dict(case, input='heldout')]))


if __name__ == '__main__':
    unittest.main()
