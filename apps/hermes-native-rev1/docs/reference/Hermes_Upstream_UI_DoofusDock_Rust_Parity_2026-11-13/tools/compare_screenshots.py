#!/usr/bin/env python3
"""Offline objective image-diff aid; screenshots must first be captured by a real UI test."""
import argparse, math
from PIL import Image,ImageChops,ImageStat
p=argparse.ArgumentParser()
p.add_argument('source');p.add_argument('target');p.add_argument('--output',default='visual-diff.png')
a=p.parse_args()
src=Image.open(a.source).convert('RGB');out=Image.open(a.target).convert('RGB')
if src.size!=out.size: raise SystemExit(f'FAIL: screenshot sizes differ: {src.size} vs {out.size}')
diff=ImageChops.difference(src,out)
stat=ImageStat.Stat(diff)
rmse=math.sqrt(sum(x*x for x in stat.rms)/3)
diff.save(a.output)
print(f'source={a.source} target={a.target} size={src.size} channel-RMSE={rmse:.4f} diff={a.output}')
print('This is not a standalone pass/fail verdict. Check dynamic masks, fonts, DPI, hover/focus and behavior.')
