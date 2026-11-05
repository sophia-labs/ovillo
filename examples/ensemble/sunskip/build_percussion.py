#!/usr/bin/env python3
"""Reproduce the invented Sunskip pocket anatomy and its composed performance.
Stdlib only. All changed anatomy numbers are design hypotheses, not measurements.
"""
from pathlib import Path
import copy, hashlib, json, math
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
BASE=ROOT/'profiles/ariadne-steel-string-v0.plan.json'
def norm(v):
    s=math.sqrt(sum(x*x for x in v)); return [x/s for x in v]
def write(name,obj): (HERE/name).write_text(json.dumps(obj,indent=2)+'\n')
p=json.loads(BASE.read_text()); p['world_name']='sunskip-pocket-modal-percussion-v1'
# The first mode remains the declared tab course. Other modes are invented.
roles={6:('low', [1,1.61,2.14,2.83,3.74,4.51], .29,.30,.16,.08,0),
       5:('tom', [1,1.47,2.09,2.91,3.67,4.88], .21,.20,.32,.10,-3),
       4:('rim', [1,3.73,5.13,7.21,8.63,10.41], .075,.08,.91,.14,-7),
       3:('side',[1,2.19,3.47,4.93,6.17,7.89], .11,.12,.70,.12,-6),
       2:('bead',[1,4.11,6.29,8.77,10.83,13.11], .055,.055,.97,.07,-11),
       1:('seed',[1,2.73,4.81,6.37,8.93,11.41], .038,.04,.83,.06,-12)}
hand={}
for b in p['bodies']:
    body=b['body']; path=body['configuration_path']
    if b['id'].startswith('string/'):
        n=int(b['id'].split('/')[1]); role,ratios,t60,pos,pick,pm,gain=roles[n]
        f0=path['left']['metric']['frequency_hz'][0]
        # Extend each distinct cluster with irrationally spaced upper modes.
        ratios=ratios+[ratios[-1]+(i+1)*(.81+n*.047)+(i+1)**1.16*.16 for i in range(18)]
        freq=[f0*r for r in ratios]
        decay=[t60/(1+.19*i)**.85 for i in range(24)]
        shape=norm([math.sin(math.pi*(i+1)*pos)/(i+1) for i in range(24)])
        body['anatomy_name']='invented-pocket-'+role
        body['source_direction']=shape
        hand['pk.'+role]={'string':n,'pitch':{6:'E2',5:'A2',4:'D3',3:'G3',2:'B3',1:'E4'}[n],'pos':pos,'pick':pick,'pm':pm,'gain':gain}
    else:
        n=int(b['id'].split('/')[1]); f0=[108.,327.,690.][n]
        freq=[f0*(1+.39*i+.027*i*i) for i in range(24)]
        decay=[[.15,.09,.045][n]/(1+.08*i) for i in range(24)]
        shape=norm([math.cos((i+1)*(.23+n*.17))/(1+.11*i) for i in range(24)])
        body['anatomy_name']='invented-pocket-shell-'+str(n)
    for config in [path['left'],path['right']]:
        config['metric']['frequency_hz']=freq[:]
        config['metric']['t60_seconds']=decay[:]
        config['radiation_direction']=shape[:]
    # Fixed percussion: both ends equal; score uses fret zero exclusively.
    path['coordinate']='fixed_modal_geometry'; path['left_name']='rest';path['right_name']='rest';path['initial_amount']=0.
for e in p['direct_couplings']:
    n=int(e['left_body'].split('/')[1]); j=int(e['right_body'].split('/')[1])
    pos=roles[n][3]
    e['left_direction']=norm([math.sin(math.pi*(i+1)*pos)/(1+.35*i) for i in range(24)])
    e['right_direction']=norm([math.cos((i+1)*(.21+.09*n+.13*j))/(1+.1*i) for i in range(24)])
    e['rate_hz']=[72.,210.,490.][j]
    # Role-specific reciprocal sharing; low hits favor low shell.
    e['weight']={6:[.85,.16,.025],5:[.65,.28,.04],4:[.08,.45,.52],3:[.23,.60,.25],2:[.025,.21,.72],1:[.035,.31,.56]}[n][j]
# Canonical baseline space ports, poses, shared-space, passive reservoirs and laws retained.
write('pocket.plan.json',p)
# Event coordinates are 32nd-note ticks. No random timing/velocity offsets.
parts={k:{} for k in hand}
events={k:{b:{} for b in range(1,49)} for k in hand}
def hit(role,b,t,d='p',g=0):
    if 0<=t<32: events['pk.'+role][b][t]=(d,g)
for b in range(1,49):
    if b<=4:
        if b in (1,3,4): hit('low',b,0,'p')
        if b>=2: hit('side',b,20 if b%2==0 else 22,'pp')
        if b>=3: hit('bead',b,6,'ppp');hit('bead',b,26,'pp')
        if b==4: hit('tom',b,28,'pp');hit('seed',b,30,'ppp')
        continue
    if b==48:
        hit('low',b,0,'mp');hit('side',b,8,'pp');hit('bead',b,10,'ppp');continue
    if b>=45:
        hit('low',b,0,'p');hit('side',b,18 if b==45 else 20,'pp')
        hit('seed',b,6,'ppp');hit('bead',b,26,'ppp')
        if b==45: hit('tom',b,28,'pp')
        if b==47: hit('rim',b,24,'pp');hit('seed',b,29,'ppp')
        continue
    middle=21<=b<=28
    dense=13<=b<=20 or 37<=b<=44
    late=b>=29
    # Two-bar low motif: displaced second answer, never four-square backbeat.
    hit('low',b,0,'p' if middle else 'mp',-1 if b%4==0 else 0)
    if not middle or b%2==0: hit('low',b,[18,20,22,18][(b-5)%4],'pp' if middle else 'p')
    if b%4==3 and not middle: hit('low',b,30,'pp',-1)
    # Rim and side alternate structural responsibility across a four-bar phrase.
    for t in ([10,26] if b%4==1 else [12,28] if b%4==2 else [10,24] if b%4==3 else [14,26]):
        if middle and t>20: continue
        hit('side' if (b+t//2)%3 else 'rim',b,t,'pp' if middle else 'p',-1)
    # 3+3+2 bead cycle, rotated on alternate bars, with a quiet omitted middle.
    bead=([4,16,28] if b%2 else [6,18,26])
    if middle: bead=[6] if b%2 else [26]
    for i,t in enumerate(bead):hit('bead',b,t,'pp' if i==0 and dense else 'ppp',i*.4)
    # Five 16th-spacing seed phrase across the whole form; intentional phase.
    for t in range(2,32,2):
        if ((b-5)*16+t//2)%5==0 and (not middle or b%3==0):hit('seed',b,t,'ppp',-1.5)
    if b%4==2 and not middle:hit('side',b,23,'ppp');hit('side',b,24,'pp',-2)
    if dense and b%2:
        hit('rim',b,7,'ppp');hit('rim',b,8,'pp',-2)
    if late and not middle and b%4==1:
        hit('seed',b,21,'ppp');hit('seed',b,22,'pp',-2)
    if b in (12,20,28,36,44):
        # Different endings: a handoff, not a repeated stock fill.
        ending={12:[('tom',24,'p'),('side',27,'pp'),('seed',29,'ppp')],
                20:[('tom',22,'mp'),('rim',25,'pp'),('tom',28,'p'),('bead',31,'ppp')],
                28:[('side',24,'pp'),('tom',30,'pp')],
                36:[('tom',24,'p'),('rim',27,'pp'),('side',28,'p'),('seed',31,'ppp')],
                44:[('tom',22,'mp'),('side',25,'pp'),('rim',27,'p'),('tom',30,'pp')]}[b]
        for role,t,d in ending:hit(role,b,t,d)
    elif b%4==0 and not middle:hit('tom',b,28 if b%8 else 30,'pp')
    if b in (16,32,40):hit('tom',b,15,'ppp');hit('tom',b,16,'pp')
# Compact durations encode the silence explicitly. Notes last one tick: anatomy owns decay.
durs=[(32,'w'),(24,'h.'),(16,'h'),(12,'q.'),(8,'q'),(6,'e.'),(4,'e'),(3,'s.'),(2,'s'),(1,'t')]
def rests(ticks):
    out=[]
    for n,s in durs:
        while ticks>=n:out.append('R'+s);ticks-=n
    assert ticks==0
    return out
for voice,bars in events.items():
    for b,ev in bars.items():
        out=[];now=0
        for t,(d,g) in sorted(ev.items()):
            out+=rests(t-now)
            gain=f'[gain={g:g}]' if g else ''
            h=hand[voice];out += ['{'+d+'}',f"{h['pitch']}t@{h['string']}:0{gain}"]
            now=t+1
        out+=rests(32-now)
        parts[voice][str(b)]=' '.join(out)
write('percussion.parts.json',parts)
# Record all changed/inherited fields unambiguously through full parameter payload + baseline hash.
design={'status':'design-hypothesis','name':'Sunskip pocket','claim':'Invented passive modal percussion played by the canonical finite boundary pulse; no measured drum, membrane collision, noise source or real-instrument reconstruction is claimed.',
'baseline':{'path':str(BASE.relative_to(ROOT)),'sha256':hashlib.sha256(BASE.read_bytes()).hexdigest(),'schema':p['schema_version']},
'law_provenance':['ARIADNE-PURE-PHYSICS-CONSTITUTION.md §3.1, §3.2, §8','SPEC-ARIADNE-STEEL-STRING.md: Limits of v0','mus-rs/crates/mus-engine/src/ariadne_tab.rs: apply_hands, performance, PLUCK_SCALE'],
'parameter_status':'Every value changed from baseline is an unmeasured design hypothesis; the entire generated plan is included by digest. Baseline fields retained verbatim inherit baseline provenance, not empirical validation.',
'plan_sha256':hashlib.sha256((HERE/'pocket.plan.json').read_bytes()).hexdigest(),
'changed_fields':{'world_name':p['world_name'],'bodies':[{ 'id':b['id'],'body_anatomy_name':b['body']['anatomy_name'],'configuration_path':b['body']['configuration_path'],'source_direction':b['body']['source_direction']} for b in p['bodies']],'direct_couplings':p['direct_couplings']},
'hands':hand,'recommended_base_gain_db':0,'recommended_send':0.02,
'engine_overrides':'apply_hands replaces source_direction with normalized sin(pi*n*pos)/n, replaces aperture frequency/damping from pick, adds pm-dependent modal loss, and may retune using first mode. First modes match baseline tuning; both endpoints equal; fret remains zero.',
'form':'1–4 gradual entrance; 5–12 A; 13–20 B; 21–28 piano foreground; 29–36 A prime; 37–44 B prime; 45–48 coda. Five-16th seed phase and 3+3+2 bead motives; final events before beat 2.5.',
'event_counts':{k:sum(len(v) for v in bars.values()) for k,bars in events.items()},
'validation':{'generator_assertions':'24 positive frequencies and decays per endpoint; frequencies below 21600 Hz; normalized declared shapes; event coordinates fit exactly 32 ticks/bar. Engine check/render still required.'}}
for b in p['bodies']:
    for end in ('left','right'):
        c=b['body']['configuration_path'][end]
        assert len(c['metric']['frequency_hz'])==24 and all(0<f<21600 for f in c['metric']['frequency_hz'])
        assert all(t>0 for t in c['metric']['t60_seconds'])
        assert abs(sum(x*x for x in c['radiation_direction'])-1)<1e-12
write('percussion.design.json',design)
print('Wrote pocket.plan.json, percussion.parts.json, percussion.design.json')
print('Events:',design['event_counts'])
