"""Read public human JSONL only. Never opens a private checkpoint or a live world."""
import argparse,collections,copy,hashlib,json,subprocess,sys
from pathlib import Path
from freeze026 import apply
from play030 import Client,ROOT
CATEGORIES=['战斗目标','技能','奶还是打','留不留尾巴','换位','用物品','路线','奇物','扎营吃东西','离开时机','选本','选人','花钱','其他']
def kind(a):
 if isinstance(a,str):return a,{}
 if isinstance(a,dict) and a:
  k=next(iter(a));return k,a[k]
 return '',{}
def inner(a):
 k,v=kind(a)
 if k in ['Opening','Raid','Farm']:return kind(v)
 if k=='Town':return kind(v)
 return k,v
def fight(a):
 k,v=inner(a)
 return v if k=='Fight' else a
def category(human,ai,job,frame):
 hk,hv=inner(human);ak,av=inner(ai)
 if hk=='Fight' or ak=='Fight' or (job or {}).get('wrapper')=='Arena':
  h,a=fight(human),fight(ai);ht,hp=kind(h);at,ap=kind(a)
  if 'Retreat' in [ht,at]:return '离开时机'
  if 'Item' in [ht,at]:return '用物品'
  if any(x in ['Move','Sacrifice'] for x in [ht,at]):return '换位'
  o=(job or {}).get('input',{}).get('observation',{});legal=o.get('actions',[])
  def recovery(action):
   l=next((x for x in legal if x['action']==action),{})
   return bool(l.get('heal',[0,0])[1]>0 or l.get('stress_heal',0)>0)
  if recovery(h)!=recovery(a):
   enemies=sum(not u['hero'] and not u['corpse'] and u['hp']>0 for u in o.get('units',[]))
   return '留不留尾巴' if enemies==1 else '奶还是打'
  if isinstance(hp,dict) and isinstance(ap,dict) and hp.get('skill')==ap.get('skill') and hp.get('target')!=ap.get('target'):return '战斗目标'
  return '技能'
 if any(k in ['Leave','Exit'] for k in [hk,ak]):return '离开时机'
 if any(k in ['Move','Backstep','Unlock'] for k in [hk,ak]):return '路线'
 if 'Interact' in [hk,ak] or 'Disarm' in [hk,ak] or 'Obstacle' in [hk,ak]:return '奇物'
 if any(k in ['Camp','Hunger'] for k in [hk,ak]):return '扎营吃东西'
 if 'Use' in [hk,ak]:
  v=hv if hk=='Use' else av;slot=v.get('slot',-1) if isinstance(v,dict) else -1;slots=(frame.get('inventory') or {}).get('slots',[])
  return '扎营吃东西' if 0<=slot<len(slots) and slots[slot] and slots[slot]['item']['kind']=='provision' else '用物品'
 if 'Select' in [hk,ak] or 'Prepare' in [hk,ak]:return '选本'
 if any(k in ['Party','AssignParty','Hire','Dismiss'] for k in [hk,ak]):return '选人'
 if any(k in ['ToggleSkill','SelectSkills','SelectCamping','ToggleCamping'] for k in [hk,ak]):return '技能'
 if any(k in ['Upgrade','LearnCamp','Equip','Unequip','Sell','Buy','Exchange','Queue','Cancel','BuildDistrict'] for k in [hk,ak]):return '花钱'
 if 'Reorder' in [hk,ak]:return '换位'
 return '其他'
def j(v):return json.dumps(v,ensure_ascii=False,separators=(',',':'))
def analyse(source,out,exe=None):
 source=Path(source).resolve();source=source/'steps.jsonl' if source.is_dir() else source
 original=hashlib.sha256(source.read_bytes()).hexdigest();worker=None;frame={};rows=[]
 groups={k:dict(steps=0,rules_available=0,rollout_available=0,rules_agree=0,rollout_agree=0,displayed_rules=0,displayed_rollout=0,followed_rules=0,followed_rollout=0) for k in ['shown','not_shown']}
 categories={k:dict(rules=0,rollout=0,examples=[]) for k in CATEGORIES}
 try:
  for line in source.open(encoding='utf-8-sig'):
   log=json.loads(line)
   if log.get('schema_version')!='dd-replay-022-v2' or log.get('decision_maker')!='human' or log['step']!=len(rows)+1:raise ValueError('需要完整、连续的human/022-v2记录')
   frame=apply(frame,log['before']['replay']);before=copy.deepcopy(frame);frame=apply(frame,log['after']['replay'])
   h=log['human'];exposure=h.get('hints',{});shadow=h.get('shadow') or {};human=h['action'];job=shadow.get('job')
   rules=exposure.get('rules') if exposure.get('rules_shown') else shadow.get('rules')
   rules=rules or {};group='shown' if exposure.get('shown') else 'not_shown';g=groups[group];g['steps']+=1
   cached=exposure.get('rollout') if exposure.get('rollout_shown') else None
   if job:
    if worker is None:worker=Client(['--shadow-worker'],exe)
    try:comparison=worker.call('compare',job=job,human=human,cached=cached)
    except RuntimeError as e:comparison=dict(rollout=cached,human_score=None,ai_score=None,human_in_candidates=None,score_error=str(e))
   else:
    mc=cached or shadow.get('unscored_rollout') or (dict(action=rules.get('action'),delegated='existing rules outside scored combat') if not shadow.get('model_error') else dict(error=shadow['model_error']))
    comparison=dict(rollout=mc,human_score=None,ai_score=None,human_in_candidates=None,score_reason='no scored public battle at this boundary')
   mc=comparison.get('rollout') or {};ra=rules.get('action');ma=mc.get('action')
   for name,a in [('rules',ra),('rollout',ma)]:
    if a is not None:g[name+'_available']+=1;g[name+'_agree']+=a==human
   for name in ['rules','rollout']:
    if exposure.get(name+'_shown'):g['displayed_'+name]+=1;g['followed_'+name]+=bool(exposure.get('followed_'+name))
   row=dict(step=log['step'],group=group,hints_enabled_at_selection=bool(exposure.get('enabled')),human=human,rules=rules,**comparison,notes=h.get('notes',[]),thinking_seconds=h.get('thinking_seconds'),scripted=bool(h.get('scripted')),context=before.get('context'),actor=((job or {}).get('input') or {}).get('observation',{}).get('actor'))
   rows.append(row)
   for name,a in [('rules',ra),('rollout',ma)]:
    if a is None or a==human:continue
    c=category(human,a,job,before);categories[c][name]+=1;categories[c]['examples'].append(dict(step=log['step'],advisor=name,human=human,ai=a,notes=row['notes'],context=row['context'],human_in_candidates=comparison.get('human_in_candidates'),human_score=comparison.get('human_score'),ai_score=comparison.get('ai_score')))
 finally:
  if worker:worker.close()
 assert hashlib.sha256(source.read_bytes()).hexdigest()==original,'source changed during analysis; retry after saving/exiting'
 for c in categories.values():c['examples']=sorted(c['examples'],key=lambda e:(not bool(e['notes']),e['human_in_candidates'] is not False,e['step']))[:3]
 result=dict(version='human-shadow030-v1',source=str(source),source_sha256=original,groups=groups,categories=categories,steps=rows,limits=['Rule intentions are approximate and committed along the real human trajectory. Cheap rule choices are captured offscreen even when hints are off; offline scoring fills missing rollout results.','Shown rollout/rule recommendations are reused exactly. Offscreen completion is not classified as shown. Previously shown hints remain exposed after toggling off or resuming.','Outside-candidate forced scoring uses unchanged public models and common seeds, across the same plans. Pilot sample counts and unavailable model scores remain explicit.','This is tool verification, not original-game calibration or evidence of human preference unless the records are actually from the user.'])
 out=Path(out);out.mkdir(parents=True,exist_ok=False)
 (out/'shadow.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8',newline='\n')
 text=['做到：逐步影子对比完成，原记录未修改。','提示分组：按实际显示过的提示分组；看过后再关闭仍算已显示。',f'记录：{len(rows)}步；脚本动作 {sum(r["scripted"] for r in rows)}步。','','|提示暴露|步骤|规则一致|推演一致|跟随已显示规则|跟随已显示推演|','|---|---:|---:|---:|---:|---:|']
 for label,g in groups.items():
  ratio=lambda a,b:f'{a}/{b} ({a/b:.1%})' if b else '0/0（无机会）'
  text.append('|'+label+'|'+str(g['steps'])+'|'+ratio(g['rules_agree'],g['rules_available'])+'|'+ratio(g['rollout_agree'],g['rollout_available'])+'|'+ratio(g['followed_rules'],g['displayed_rules'])+'|'+ratio(g['followed_rollout'],g['displayed_rollout'])+'|')
 text+=['','|不一致类别|规则|推演|','|---|---:|---:|']
 for name,c in categories.items():text.append(f"|{name}|{c['rules']}|{c['rollout']}|")
 for name,c in categories.items():
  if not c['examples']:continue
  text+=['',f'### {name}','']
  for e in c['examples']:text.append('- `'+j(e).replace('`','\\`')+'`')
 text+=['','跨步记忆是沿人类真实动作提交的近似意图；便宜规则在关提示时也保存在公开影子上下文，离线补推演。已经显示的结果原样复用。候选外动作强制评分只用于诊断；分数为空时保留模型原因，不虚构0分。完整均值、最差10%、样本数、候选归属与每步备注见shadow.json。','',f'原记录SHA-256：`{original}`。分析只读steps.jsonl，不打开私有检查点。']
 (out/'shadow.md').write_text('\n'.join(text)+'\n',encoding='utf8',newline='\n');return out
def main():
 p=argparse.ArgumentParser();p.add_argument('record');p.add_argument('--out');p.add_argument('--exe');a=p.parse_args();source=Path(a.record);base=ROOT/'target/030-shadow'/ (source.stem if source.is_file() else source.name);out=Path(a.out) if a.out else base;n=1
 while out.exists() and not a.out:out=base.with_name(base.name+f'-{n}');n+=1
 print(analyse(source,out,Path(a.exe).resolve() if a.exe else None)/'shadow.md')
if __name__=='__main__':
 if hasattr(sys.stdout,'reconfigure'):sys.stdout.reconfigure(encoding='utf8')
 main()
