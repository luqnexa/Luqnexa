# SPDX-License-Identifier: AGPL-3.0-only
"""Build train-only contrast pairs from reviewed offline conversion output.

Requires explicit --reviewed: generated references and negatives need review.
Does not train, install, alter evaluation splits, or contact a network.
"""
import argparse
import hashlib
import json
from pathlib import Path


def prepare(dataset, report):
    raw = dataset.read_bytes()
    records = [json.loads(line) for line in raw.decode('utf-8-sig').splitlines() if line.strip()]
    if report.get('partition') != 'train' or report.get('training_during_evaluation') is not False:
        raise ValueError('only offline training-partition conversions are accepted')
    if report.get('dataset_sha256') != hashlib.sha256(raw).hexdigest():
        raise ValueError('dataset hash mismatch')
    by_input = {r['input']: r for r in records if r['split'] == 'train'}
    excluded = {r['input'] for r in records if r['split'] != 'train'}
    pairs = {(r['input'], r['context'], r['good'], r['bad']) for r in records}
    extras = []
    for row in report['rows']:
        source = by_input.get(row['input'])
        if source is None or row['input'] in excluded:
            raise ValueError('report contains non-training input')
        if row['reference'] != source['good'] or row['context'] != source['context']:
            raise ValueError('reference or context changed')
        outcome = row['modes'][2]
        if outcome['correct'] or not outcome['reference_in_pool']:
            continue
        negative = outcome['first']
        if not isinstance(negative, str) or not negative or negative == source['good']:
            raise ValueError('invalid negative')
        pair = (source['input'], source['context'], source['good'], negative)
        if pair in pairs:
            continue
        pairs.add(pair)
        extras.append(dict(source, bad=negative,
            provenance='Offline training-partition first candidate; assistant-reviewed, not independent human validation',
            review='Exact-surface contrast; kana or alternative spelling may also be acceptable. Not a universal semantic error.'))
    return records + extras, len(extras)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dataset', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--reviewed', action='store_true', required=True)
    args = parser.parse_args()
    rows, added = prepare(args.dataset, json.loads(args.report.read_text(encoding='utf-8-sig')))
    with args.output.open('x', encoding='utf-8', newline='\n') as output:
        for row in rows:
            output.write(json.dumps(row, ensure_ascii=False) + '\n')
    print(json.dumps(dict(added_contrasts=added, unique_train_sentences=len({r['input'] for r in rows if r['split'] == 'train'}), evaluation_unchanged=True)))


if __name__ == '__main__':
    main()
