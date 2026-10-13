# SPDX-License-Identifier: AGPL-3.0-only
"""Compare identical-input offline reports; never edits a dataset or model."""
import argparse
import json
from pathlib import Path


def compare(before, after):
    for key in ('dataset_sha256', 'model_sha256', 'dictionary_sha256', 'overlay_sha256', 'partition'):
        if before.get(key) != after.get(key):
            raise ValueError(f'not comparable: {key}')
    if len(before['rows']) != len(after['rows']):
        raise ValueError('row count changed')
    improved, regressed, pool_gained, pool_lost = [], [], [], []
    for previous, current in zip(before['rows'], after['rows']):
        for key in ('input', 'context', 'reference'):
            if previous[key] != current[key]:
                raise ValueError(f'row changed: {key}')
        a, b = previous['modes'][2], current['modes'][2]
        details = dict(input=current['input'], context=current['context'], reference=current['reference'], before=a['first'], after=b['first'])
        if not a['correct'] and b['correct']: improved.append(details)
        if a['correct'] and not b['correct']: regressed.append(details)
        if not a['reference_in_pool'] and b['reference_in_pool']: pool_gained.append(details)
        if a['reference_in_pool'] and not b['reference_in_pool']: pool_lost.append(details)
    return dict(partition=after['partition'], examples=len(after['rows']), improved=improved, regressed=regressed,
        before_temperature=before.get('neural_ranking_temperature'), after_temperature=after.get('neural_ranking_temperature'),
        pool_gained=pool_gained, pool_lost=pool_lost, before_metrics=before['metrics'], after_metrics=after['metrics'],
        scope='Same model/data/dictionary batch conversions; separate host timing runs are not controlled device speed or generalization proof')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    args = parser.parse_args()
    load = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))
    # ASCII JSON escapes preserve Japanese under Windows stdout code pages.
    print(json.dumps(compare(load(args.before), load(args.after)), ensure_ascii=True, indent=2))


if __name__ == '__main__':
    main()
