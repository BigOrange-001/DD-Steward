"""Freeze development-only historic combat openings before any026 arena outcomes."""
from pathlib import Path
import json,copy,re,collections,argparse
R=Path(__file__).resolve().parent.parent
BOSS=re.compile(r'^(necromancer|prophet|swine_(prince|king|god)|hag|witch|brigand_cannon|siren|crew|drowned_crew|flesh|formless_flesh)_')
RARE=re.compile(r'^(collector|shambler|thing|crow|fanatic)_')

def apply(state,p):
 if 'full' in p:return copy.deepcopy(p['full'])
 state.update(p.get('set',{}))
 if 'map_delta' in p:
  d=p['map_delta'];m=state.get('map')
  if m:
   ns={x['id']:x for x in m['nodes']}
   for k in d.get('remove',[]):ns.pop(k,None)
   ns.update({x['id']:x for x in d.get('upsert',[])})
   state['map']=dict(d['meta'],nodes=[ns[k] for k in d['order']])
 return state

def exp(v):return v.get('expedition') or {}
def combat(v):return exp(v).get('battle')
def alive(b):return {u['id'] for u in b.get('units',[]) if u.get('build') is not None and not u['corpse']}
def main():
 p=argparse.ArgumentParser();p.add_argument('--out',type=Path,default=R/'target/026-corpus/raw');a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
 if (a.out/'index.json').exists():raise RuntimeError('Frozen corpus already exists; use another output directory')
 paths=[R/f'target/023-development-{group}-seed-{seed}/steps.jsonl' for group in ['baseline','rules021'] for seed in range(1,9)]
 paths += sorted(p for p in (R/'target').glob('024*-seed-[1-8]/steps.jsonl') if 'holdout' not in str(p).lower())
 paths=list(dict.fromkeys(paths))
 episodes=[];scanned=[];errors=[]
 for path in paths:
  if not path.exists():continue
  state={};current=None;linecount=0
  def finish():
   nonlocal current
   if current is not None:episodes.append(current);current=None
  try:
   for line in path.open(encoding='utf-8'):
    linecount+=1;v=json.loads(line);state=apply(state,v.get('before',{}).get('replay',{}));b=combat(v['before']);ctx=state.get('context',{});key=(json.dumps(ctx,sort_keys=True),exp(v['before']).get('node'))
    if current is not None and (not b or key!=current['_key'] or b['round']<current['_last_round']):finish()
    if b and ctx.get('phase')=='expedition' and b.get('units') and any(u['id']==b['actor'] and u['hero'] and not u['corpse'] for u in b['units']):
     if current is None:
      saved=copy.deepcopy({k:state.get(k) for k in ['context','inventory','party_build','roster','battle_heroes','quest','item_states']})
      m=state.get('map') or {};node=next((n for n in m.get('nodes',[]) if n['id']==m.get('position')),None)
      saved['corridor']=bool(node and not node['room'])
      current=dict(source=str(path.relative_to(R)),step=v['step'],week=v['before'].get('week'),frame=saved,units=copy.deepcopy(b['units']),round=b['round'],actor=b['actor'],torch=exp(v['before']).get('torch'),retreat_legal=exp(v['before']).get('can_retreat'),old_max_deaths=0,initial_alive=len(alive(b)),_ids=alive(b),_key=key,_last_round=b['round'])
     current['_last_round']=b['round'];current['old_max_deaths']=max(current['old_max_deaths'],len(current['_ids']-alive(b)))
    state=apply(state,v.get('after',{}).get('replay',{}));after=combat(v['after'])
    if current is not None:
     if after:current['old_max_deaths']=max(current['old_max_deaths'],len(current['_ids']-alive(after)))
     elif v['after'].get('deaths',0)>v['before'].get('deaths',0):current['old_max_deaths']=max(current['old_max_deaths'],min(current['initial_alive'],v['after']['deaths']-v['before']['deaths']))
     if not after:finish()
   finish();scanned.append(dict(path=str(path.relative_to(R)),lines=linecount));print('scanned',path.parent.name,len(episodes),flush=True)
  except (ValueError,KeyError) as e:errors.append(dict(path=str(path),line=linecount,error=str(e)));finish()
 rows=[];counts=collections.Counter();ordinary_lengths=collections.Counter()
 for i,e in enumerate(episodes):
  q=e['frame'].get('quest') or {};names=[u['class'] for u in e['units'] if not u['hero'] and not u['corpse']]
  if q.get('region')=='farm' or q.get('difficulty')!=1:continue
  if any(RARE.match(n) for n in names):category='roaming'
  elif any(BOSS.match(n) for n in names):category='boss'
  elif e['old_max_deaths']>=2:category='death_heavy'
  elif q.get('length') in [1,2]:category='ordinary'
  else:continue
  # Selection uses source class and historical deaths only, never026 outcomes.
  if category=='ordinary' and ordinary_lengths[q['length']]>=100:continue
  if counts[category]>={'ordinary':200,'boss':150,'roaming':80,'death_heavy':100}[category]:continue
  if category=='ordinary':ordinary_lengths[q['length']]+=1
  counts[category]+=1;e['category']=category;e['id']=f'{category}-{counts[category]:03d}'
  for k in list(e):
   if k.startswith('_'):del e[k]
  file=a.out/(e['id']+'.json');file.write_text(json.dumps(e,ensure_ascii=False)+'\n',encoding='utf-8');rows.append(dict(id=e['id'],category=category,source=e['source'],step=e['step'],week=e['week'],old_max_deaths=e['old_max_deaths'],file=str(file)))
 (a.out/'index.json').write_text(json.dumps(dict(kind='partial historical visible snapshots; complete units/round/actor/torch; legal previews omitted in original logs',counts=dict(counts),rows=rows,scanned=scanned,errors=errors),ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(dict(counts),errors,flush=True)
if __name__=='__main__':main()
