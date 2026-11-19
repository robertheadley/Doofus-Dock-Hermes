#!/usr/bin/env python3
"""Offline package integrity and crosswalk test. Requires stdlib; yaml optional."""
import csv, json, pathlib, re, sys
root=pathlib.Path(__file__).resolve().parents[1]
def load_json(p): return json.loads((root/p).read_text(encoding='utf-8'))
def load_csv(p):
  with (root/p).open(newline='',encoding='utf-8-sig') as f: return list(csv.DictReader(f))
cap=load_json('maps/full_program_capability_map.json')['capabilities']
capability_map_text=(root/'maps/capabilities.yaml').read_text(encoding='utf-8')
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
allowed_status={'not_implemented','partial','implemented','verified'}
assert all(f['target_status'] in allowed_status for f in cap)
status_by_id={f['id']:f['target_status'] for f in cap}
assert all(r['target_status'] in allowed_status for r in ui)
assert all(r['target_status'] in allowed_status for r in cross)
timestamp_id='ui-display-local-timestamps-and-message-role'
assert status_by_id[timestamp_id]=='partial'
assert next(r['target_status'] for r in ui if r['id']==timestamp_id)=='partial'
assert next(r['target_status'] for r in cross if r['id']==timestamp_id)=='partial'
assert len(load_json('maps/NAVIGATION_SCREENS.json')['screens'])>40
assert len(load_csv('maps/UI_COMPONENT_SPECS.csv'))>30
assert len(load_csv('maps/STATE_OWNERSHIP.csv'))>10
sources=load_json('maps/SOURCE_SNAPSHOTS.json')
assert all(re.fullmatch(r'[a-f0-9]{40}',v['sha']) for v in sources.values())
files=list((root/'maps/capabilities/evidence').glob('EV-*.json'))
assert len(files)>10
inspection_count=implementation_count=0
for f in files:
 d=json.loads(f.read_text(encoding='utf-8'))
 if d['kind']=='inspection':
  assert d['outcome']=='pass';inspection_count+=1
 elif d['kind']=='implementation':
  assert d['outcome'] in {'partial','pass'} and re.fullmatch(r'[a-f0-9]{40}',d['commit'])
  assert d['capability_id'] in status_by_id and status_by_id[d['capability_id']] in {'partial','implemented','verified'}
  implementation_count+=1
 elif d['kind']=='test_result':
  assert d['outcome'] in {'partial','pass'} and re.fullmatch(r'[a-f0-9]{40}',d['commit'])
  assert re.fullmatch(r'[a-z0-9-]+',d['capability_id'])
  assert re.search(rf'(?m)^- id: {re.escape(d["capability_id"])}$',capability_map_text)
  assert isinstance(d.get('scenario'),str) and d['scenario']
  assert isinstance(d.get('locator'),str) and d['locator']
  implementation_count+=1
 else: raise AssertionError(f"unsupported evidence kind: {d['kind']}")
try:
 import yaml
 d=yaml.safe_load((root/'maps/capabilities.yaml').read_text(encoding='utf-8'))
 assert len(d['capabilities'])==len(cap)+summary['source_baseline_capabilities']
 assert {r['id'] for r in d['capabilities'] if r['id'].startswith('ui-')}==set(ids)
except ImportError:
 print('PyYAML not present: skipped UCM YAML parse (install PyYAML to check)')
print('PASS:',len(cap),'target behaviors,',len(ui),'UI inventory items,',len(tests),'pending tests,',inspection_count,'source inspection receipts,',implementation_count,'implementation receipts')
