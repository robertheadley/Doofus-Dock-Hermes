#!/usr/bin/env python3
"""Check internal consistency of the proposed Rust GUI parity research package."""
from pathlib import Path
import csv,json
ROOT=Path(__file__).resolve().parent.parent
cap=json.loads((ROOT/'maps/full_program_capability_map.json').read_text())['capabilities']
ids=[x['id'] for x in cap]
assert len(ids)==len(set(ids)), 'Duplicate capability ID'
tests=[x['test_id'] for x in cap]
assert len(tests)==len(set(tests)), 'Duplicate test ID'
req=list(csv.DictReader((ROOT/'planning/REQUIREMENTS_LEDGER.csv').open()))
acc=list(csv.DictReader((ROOT/'planning/ACCEPTANCE_MATRIX.csv').open()))
assert len(req)==len(acc)==len(cap)
assert {x['capability_id'] for x in req}==set(ids)
assert {x['capability_id'] for x in acc}==set(ids)
assert {x['test_id'] for x in req}==set(tests)
assert {x['test_id'] for x in acc}==set(tests)
assert all(x['target_status']=='not_implemented' for x in cap)
assert not any(x['area'] in {'doofus-control','doofus-vault','doofus-rules-core','dashboard-administration'} for x in cap)
lay=json.loads((ROOT/'maps/HERMES_SOURCE_LAYOUT_TREES.json').read_text())
assert lay['source_default']['weights']==[1,3.4,1.25]
assert lay['mode_policy']['default_mode']=='advanced'
assert json.loads((ROOT/'maps/CAPABILITY_SUMMARY.json').read_text())['target_total']==len(cap)
assert (ROOT/'PROMPT_FOR_HERMES.md').is_file()
print(f'OK: {len(cap)} pending capabilities and matching requirement/test IDs; source default tree/mode checked.')
