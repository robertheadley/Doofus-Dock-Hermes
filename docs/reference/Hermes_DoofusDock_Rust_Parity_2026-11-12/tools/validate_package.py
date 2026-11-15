#!/usr/bin/env python3
"""Offline package integrity and crosswalk test. Requires stdlib; yaml optional."""
import csv, json, pathlib, re, sys
root=pathlib.Path(__file__).resolve().parents[1]
def load_json(p): return json.loads((root/p).read_text(encoding='utf-8'))
def load_csv(p):
  with (root/p).open(newline='',encoding='utf-8-sig') as f: return list(csv.DictReader(f))
cap=load_json('maps/full_program_capability_map.json')['capabilities']
ui=load_csv('maps/UI_ELEMENT_INVENTORY.csv')
cross=load_csv('maps/CAPABILITY_CROSSWALK.csv')
ledger=load_csv('planning/REQUIREMENTS_LEDGER.csv')
tests=load_csv('planning/ACCEPTANCE_MATRIX.csv')
summary=load_json('maps/CAPABILITY_SUMMARY.json')
ids=[f['id'] for f in cap]
assert len(ids)==len(set(ids)), 'duplicate capability ids'
assert len(cap)==len(ui)==len(cross)==len(ledger)==len(tests)==summary['total_target_capabilities']
for records,key in [(ui,'id'),(cross,'id'),(ledger,'capability_id'),(tests,'capability_id')]:
 assert {r[key] for r in records}==set(ids),f'bad crosswalk {key}'
assert len({t['test_id'] for t in tests})==len(tests)
assert {t['result'] for t in tests}=={'pending'}
assert all(f['target_status']=='not_implemented' for f in cap)
assert len(load_json('maps/NAVIGATION_SCREENS.json')['screens'])>40
assert len(load_csv('maps/UI_COMPONENT_SPECS.csv'))>30
assert len(load_csv('maps/STATE_OWNERSHIP.csv'))>10
sources=load_json('maps/SOURCE_SNAPSHOTS.json')
assert all(re.fullmatch(r'[a-f0-9]{40}',v['sha']) for v in sources.values())
files=list((root/'maps/capabilities/evidence').glob('EV-*.json'))
assert len(files)>10
for f in files:
 d=json.loads(f.read_text(encoding='utf-8'));assert d['outcome']=='pass' and d['kind']=='inspection'
try:
 import yaml
 d=yaml.safe_load((root/'maps/capabilities.yaml').read_text(encoding='utf-8'))
 assert len(d['capabilities'])==len(cap)+summary['source_baseline_capabilities']
 assert {r['id'] for r in d['capabilities'] if r['id'].startswith('ui-')}==set(ids)
except ImportError:
 print('PyYAML not present: skipped UCM YAML parse (install PyYAML to check)')
print('PASS:',len(cap),'target behaviors,',len(ui),'UI inventory items,',len(tests),'pending tests,',len(files),'source inspection receipts')
