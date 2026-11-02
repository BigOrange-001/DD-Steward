"""Minimal Chinese terminal for the public JSONL protocol. No private save is read."""
import argparse,json,os,queue,subprocess,sys,time,xml.etree.ElementTree as ET
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
class LineInput:
 def __init__(self):self.buffer=''
 def get(self,timeout=None):
  if not sys.stdin.isatty():return sys.stdin.readline() or 'q'
  if os.name!='nt':
   import select
   if not select.select([sys.stdin],[],[],timeout)[0]:raise queue.Empty
   return sys.stdin.readline() or 'q'
  import msvcrt
  end=None if timeout is None else time.monotonic()+timeout
  while end is None or time.monotonic()<end:
   if not msvcrt.kbhit():time.sleep(.02);continue
   char=msvcrt.getwch()
   if char=='\x03':raise KeyboardInterrupt
   if char in ('\x00','\xe0'):msvcrt.getwch();continue
   if char in ('\r','\n'):
    print();line=self.buffer;self.buffer='';return line
   if char=='\x1a':return 'q'
   if char=='\b':
    if self.buffer:self.buffer=self.buffer[:-1];print('\b \b',end='',flush=True)
   else:self.buffer+=char;print(char,end='',flush=True)
  raise queue.Empty
class Client:
 def __init__(self,args,exe=None):
  flags=getattr(subprocess,'CREATE_NEW_PROCESS_GROUP',0) if os.name=='nt' else 0
  self.process=subprocess.Popen([str(exe or ROOT/'target/release/play030.exe'),*args],cwd=ROOT,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',bufsize=1,creationflags=flags)
 def call(self,cmd,**kwargs):
  self.process.stdin.write(json.dumps(dict(cmd=cmd,**kwargs),ensure_ascii=False)+'\n');self.process.stdin.flush()
  line=self.process.stdout.readline()
  if not line:raise RuntimeError(self.process.stderr.read() or '程序已退出')
  result=json.loads(line)
  if not result.get('ok'):raise RuntimeError(result.get('error','操作失败'))
  return result['data']
 def close(self):
  if self.process.poll() is None:self.process.stdin.close();self.process.wait(timeout=30)
class Names:
 def __init__(self,paths):
  self.names={}
  for path in paths:
   try:
    for language in ET.parse(path).getroot().findall('language'):
     if language.get('id')=='schinese':
      for entry in language.findall('entry'):self.names[entry.get('id')]=''.join(entry.itertext())
   except (OSError,ET.ParseError):pass
 def name(self,key,hero=None):
  if not isinstance(key,str):return str(key)
  candidates=[key]
  if hero:candidates+=[f'combat_skill_name_{hero}_{key}',f'camping_skill_name_{hero}_{key}']
  candidates += ['hero_class_name_'+key,'str_monster_name_'+key,'monster_name_'+key,'str_monster_skill_'+key,'str_inventory_title_'+key,'str_curio_name_'+key]
  candidates += ['str_inventory_title_supply_'+key,'str_inventory_title_estate_'+key,'str_dungeon_name_'+key,'dungeon_name_'+key]
  base,_,tier=key.rpartition('_')
  if tier in ['A','B','C','D','E']:candidates+=['str_monster_name_'+base,'monster_name_'+base]
  return next((self.names[k] for k in candidates if k in self.names),key)
def battle(state):
 o=state['observation']
 if 'Arena' in o:return o['Arena']
 if 'Opening' in o:return o['Opening'].get('battle')
 c=o.get('Campaign',{})
 return (c.get('expedition') or {}).get('battle') or ((c.get('content019') or {}).get('farm') or {}).get('battle')
def unwrap(action):
 if isinstance(action,dict):
  for key in ['Opening','Raid','Farm']:
   if key in action:return action[key]
 return action
def action_label(action,state,names):
 b=battle(state);a=unwrap(action)
 if isinstance(a,dict) and 'Fight' in a:a=a['Fight']
 if b:
  legal=next((x for x in b.get('actions',[]) if x['action']==a),None)
  if legal:
   actor=next((u for u in b['units'] if u['id']==b.get('actor')), {})
   title=names.name(legal['label'],actor.get('class'))
   targets='、'.join('#'+str(i) for i in legal.get('targets',[])) or '无指定目标'
   return f"{title} → {targets}  命中{legal['hit_chance']:.0%}  伤害{legal['damage'][0]}～{legal['damage'][1]}  治疗{legal['heal'][0]}～{legal['heal'][1]} 减压{legal['stress_heal']}"
 return json.dumps(action,ensure_ascii=False,separators=(',',':'))
def matching_id(state,action):
 for row in state['actions']:
  if row.get('action')==action:return str(row['id'])
  if row.get('template') and isinstance(action,dict) and row['template'] in action.get('Town',{}):return f"{row['id']}（需参数 {json.dumps(action['Town'][row['template']],ensure_ascii=False)}）"
 return '不可用'
def render_hints(state,names):
 h=state.get('hints',{})
 if not h.get('enabled'):print('提示：关');return False,False
 rules=h.get('rules') or {};mc=h.get('rollout') or {}
 same=rules.get('action') is not None and rules.get('action')==mc.get('action')
 if rules.get('action') is not None:print(f"{'[规+推]' if same else '[规]'} #{matching_id(state,rules['action'])}  {rules.get('rule_id','')}：{rules.get('reason','')}")
 elif rules.get('error'):print('规则提示不可用：'+rules['error'])
 if mc.get('action') is not None:
  if not same:print(f"[推] #{matching_id(state,mc['action'])}"+('（原有规则回落：'+mc['fallback']+'）' if mc.get('fallback') else '（本阶段沿用规则）' if mc.get('delegated') else ''))
  elif mc.get('fallback'):print('  推演回落：'+mc['fallback'])
  for i,row in enumerate(mc.get('top3',[]),1):print(f"  {i}. #{matching_id(state,row['action'])} 均值 {row['mean']:.2f} / 最差10% {row['worst_tail']:.2f} / 合成 {row['value']:.2f}；{row['samples']}样本"+('；初筛已淘汰' if 'eliminated_pilot' in row.get('categories',[]) else ''))
 elif h.get('pending'):print('[推] 正在独立计算；现在就可以选动作。')
 elif h.get('model_error'):print('[推] 模型不可用：'+h['model_error'])
 return rules.get('action') is not None,mc.get('action') is not None
def render(state,names,page=1):
 b=battle(state);o=state['observation'];c=o.get('Campaign')
 phase='战斗' if b else '地牢' if c and c.get('expedition') else '补给' if c and c.get('provisions') else '城镇' if c else {'OldRoad':'老路','Preparation':'序章补给','FirstRuins':'初次废墟','Expedition':'地牢','Finished':'序章结束'}.get(o.get('Opening',{}).get('phase'),'开局')
 print(f"\n—— 第{state['step']}步 · {phase} · {state['status']} ——")
 if b:
  print(f"第{b['round']}回合 · 当前 #{b.get('actor')} · 光照 {b.get('torch')} · 补给 {b.get('supplies',[])}")
  for u in b['units']:
   states=[]
   for k,label in [('stunned','眩晕'),('corpse','尸体'),('stealthed','潜行'),('marked','标记'),('immobilized','定身')]:
    if u.get(k):states.append(label)
   for k,label in [('bleed','流血'),('blight','腐蚀'),('horror','恐惧'),('regeneration','持续治疗'),('buffs','增减益')]:
    if u.get(k):states.append(label+str(u[k]))
   print(f" {'我' if u['hero'] else '敌'} #{u['id']} {names.name(u['class'])} 阵位{u.get('occupied_ranks',[u['rank']])} HP {u['hp']}/{u['max_hp']} 压力{u['stress']} {' '.join(states)}")
 if not c and not b and 'Opening' in o:
  opening=o['Opening'];print(f"资产 {opening.get('wallet',{})} · 队伍 {opening.get('party',[])}")
  for h in opening.get('heroes',[]):print(f"  GUID {h['guid']} {names.name(h['class'])} HP {h['hp']}/{h['maximum_hp']} 压力{h['stress']}")
 if c and not b and not c.get('expedition'):
  town=c['town'];print(f"第{c['week']}周 · 资产 {town.get('wallet',{})} · 队伍 {town.get('party',[])}")
  for h in town.get('heroes',[]):print(f"  GUID {h['guid']} {names.name(h['class'])} 等级{h['level']} 压力{h['stress']} 武/甲{h['weapon']+1}/{h['armour']+1}"+(' 忙碌' if h['busy'] else '')+(' 失踪' if h['missing'] else '')+(' 阵亡' if not h['alive'] else ''))
  for q in (c.get('board') or {}).get('quests',[]):print(f"  任务 {q['id']} {names.name(q['dungeon'])} 难度{q['difficulty']} 长度{q['length']} {q['kind']}"+(' 可准备' if q['preparation_eligible'] else ' '+str(q.get('preparation_error',''))))
 frame=state.get('frame',{});inv=frame.get('inventory')
 if inv:print('背包：'+json.dumps(inv,ensure_ascii=False))
 m=frame.get('map')
 if m:
  print('当前位置：'+str(m.get('position'))+'；已知地图（未侦察内容保持隐藏）：')
  for n in m.get('nodes',[]):print('  '+json.dumps(n,ensure_ascii=False,separators=(',',':')))
 rules,mc=render_hints(state,names)
 rows=state['actions'];per=24;pages=max(1,(len(rows)+per-1)//per);page=max(1,min(page,pages));hint=state.get('hints',{})
 print(f"动作 第{page}/{pages}页，共{len(rows)}项；l 页码换页，编号可直接输入。")
 for row in rows[(page-1)*per:page*per]:
  a=row.get('action');markers=''
  if hint.get('enabled'):
   def hit(action):return action is not None and (action==a or bool(row.get('template') and isinstance(action,dict) and row['template'] in action.get('Town',{})))
   r=hit((hint.get('rules') or {}).get('action'));m=hit((hint.get('rollout') or {}).get('action'));markers='[规+推]' if r and m else '[规]' if r else '[推]' if m else ''
  text=action_label(a,state,names) if a is not None else f"Town.{row['template']} 参数菜单（编号后加JSON；s查看可用英雄与技能）"
  print(f" {row['id']:>4} {markers} {text}"+(' 【需确认】' if row.get('confirmation') else ''))
 if state.get('notes'):print('本步备注：'+'；'.join(state['notes']))
 return rules,mc
HELP='输入动作编号；参数菜单：编号 空格 JSON。n 文字：加备注；s：完整公开局面；h：提示开关；l 页码：动作页；?：帮助；q：保存退出。没有悔棋。'
def main():
 p=argparse.ArgumentParser(description=HELP);p.add_argument('--arena');p.add_argument('--start',choices=['old-road','veteran','champion'],default='old-road');p.add_argument('--seed',type=int,default=1);p.add_argument('--weeks',type=int,default=208);p.add_argument('--out');p.add_argument('--resume');p.add_argument('--no-hints',action='store_true');p.add_argument('--list-scenes',action='store_true');args=p.parse_args()
 os.chdir(ROOT)
 if args.list_scenes:
  v=json.loads((ROOT/'data/human-scenes-030.json').read_text(encoding='utf8'))
  for r in v['rows']:print(r['id'],r['category'])
  return
 if not args.out and not args.resume:p.error('需要 --out 新目录，或 --resume 原目录')
 cmd=['--json','--resume',args.resume] if args.resume else ['--json','--seed',str(args.seed),'--weeks',str(args.weeks),'--out',args.out,*(['--arena',args.arena] if args.arena else ['--start',args.start])]
 if args.no_hints:cmd+=['--no-hints']
 print('正在加载模拟器与已启用内容的中文名……');client=Client(cmd)
 try:
  names=Names(client.call('info')['localization_files']);state=client.call('view');inputs=LineInput()
  print(HELP);page=1;started=time.monotonic()
  def display():
   seen=render(state,names,page)
   client.call('seen',step=state['step'],rules=seen[0],rollout=seen[1]);print('> ',end='',flush=True)
  display()
  while True:
   if state['status']!='Running':print('当前场景/预算已结束。记录已保存。');client.call('q');break
   try:line=inputs.get(timeout=.25).strip()
   except queue.Empty:
    if state.get('hints',{}).get('enabled') and state['hints'].get('pending'):
     latest=client.call('poll')
     if not latest.get('hints',{}).get('pending'):
      state=latest;print('\n推演已完成：');seen=render_hints(state,names);client.call('seen',step=state['step'],rules=seen[0],rollout=seen[1]);print('> ',end='',flush=True)
    continue
   try:
    if line=='q':client.call('q',thinking_seconds=time.monotonic()-started);print('已保存。');break
    if line.startswith('n '):
     client.call('note',text=line[2:]);prior_mc=state.get('hints',{}).get('rollout');state=client.call('view');print('已记下：'+line[2:])
     if not prior_mc and state.get('hints',{}).get('rollout'):
      seen=render_hints(state,names);client.call('seen',step=state['step'],rules=seen[0],rollout=seen[1])
     print('> ',end='',flush=True);continue
    if line=='?':print(HELP);print('> ',end='',flush=True);continue
    if line=='s':
     state=client.call('view');print(json.dumps(state,ensure_ascii=False,indent=2));hint=state.get('hints',{});client.call('seen',step=state['step'],rules=bool(hint.get('enabled') and (hint.get('rules') or {}).get('action') is not None),rollout=bool(hint.get('enabled') and (hint.get('rollout') or {}).get('action') is not None));print('> ',end='',flush=True);continue
    if line=='h':state=client.call('hints');display();continue
    if line.startswith('l'):
     page=int(line[1:].strip() or '1');display();continue
    pieces=line.split(maxsplit=1);index=int(pieces[0]);row=next((r for r in state['actions'] if r['id']==index),None)
    if row is None:raise ValueError('编号不在当前公开菜单')
    confirm=False
    if row.get('confirmation'):
     print('这个动作不可撤销。输入 yes 确认，其他输入取消：',flush=True)
     confirmation=inputs.get().strip()
     if confirmation=='q':client.call('q',thinking_seconds=time.monotonic()-started);print('已保存。');return
     if confirmation.lower()!='yes':print('已取消。');print('> ',end='',flush=True);continue
     confirm=True
    kw=dict(step=state['step'],index=index,confirm=confirm,thinking_seconds=time.monotonic()-started)
    if row.get('template'):
     if len(pieces)<2:raise ValueError('这个菜单需要JSON参数；用s查看domain')
     kw['params']=json.loads(pieces[1])
    reply=client.call('act',**kw);state=reply['next'];started=time.monotonic();page=1;display()
   except (ValueError,RuntimeError,StopIteration) as e:print('操作未完成：'+str(e));print('> ',end='',flush=True)
 except KeyboardInterrupt:
  print('\n正在保存……');client.call('q',thinking_seconds=max(0,time.monotonic()-locals().get('started',time.monotonic())))
 finally:client.close()
if __name__=='__main__':
 if hasattr(sys.stdout,'reconfigure'):sys.stdout.reconfigure(encoding='utf-8')
 if hasattr(sys.stdin,'reconfigure'):sys.stdin.reconfigure(encoding='utf-8')
 if hasattr(sys.stderr,'reconfigure'):sys.stderr.reconfigure(encoding='utf-8')
 main()
