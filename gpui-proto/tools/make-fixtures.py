#!/usr/bin/env python3
"""Throwaway Copilot CLI and Claude Code histories for trying the GPUI app without your own.

  make-fixtures.py <dir> small       ~80 varied sessions: repos, days, a 3000-message one, Claude edits
  make-fixtures.py <dir> big [N]     N (default 10000) plain sessions, for timing

Then point the app at it:
  COPILOT_HOME=<dir>/copilot CLAUDE_CONFIG_DIR=<dir>/claude LOGBOOK_CACHE_DIR=<dir>/cache cargo run --release
"""
import sys

SMALL = r"""
import sys,os,json,datetime,uuid,random
root=sys.argv[1]; random.seed(7)
now=datetime.datetime.now(datetime.timezone.utc)
repos=["bookshelf","reading-list","ado-hooks","site","logbook","scratch"]
def iso(h): return (now-datetime.timedelta(hours=h)).isoformat()
def work(n):
    d=f"{root}/work/{n}"; os.makedirs(d,exist_ok=True); return d
os.makedirs(root+"/cache",exist_ok=True)
base=root+"/copilot/session-state"
code="```rust\nfn rebuild(&self) -> Result<IndexStats> {\n    let mut next = self.index.read().clone();\n    indexer::run(&mut next, &self.sources)\n}\n```"
topics=["Fix the flaky overdue-loans test","Dashboard env config for staging","Why is CI red on main?","Refactor the indexer","Add a dark theme","Reminder queue design","Update dependencies","Write release notes","Investigate slow search","Clean up the sidebar"]
n=0
for i in range(60):
    repo=repos[i%len(repos)]; cwd=work(repo)
    sid=str(uuid.UUID(int=1000+i)); d=f"{base}/{sid}"; os.makedirs(d,exist_ok=True)
    h=i*9+random.randint(0,6)
    name=topics[i%len(topics)]+(f" #{i}" if i>=10 else "")
    open(d+"/workspace.yaml","w").write(f"id: {sid}\nname: {name}\ncwd: {cwd}\nbranch: {'main' if i%3 else 'feature/x'+str(i)}\ncreated_at: {iso(h+2)}\nupdated_at: {iso(h)}\n")
    ev=[{"type":"session.start","timestamp":iso(h+2)}]
    cnt=random.randint(2,14)
    for k in range(cnt):
        ts=iso(h+2-k*0.1)
        if k%2==0: ev.append({"type":"user.message","timestamp":ts,"data":{"content":f"please look at the reminder queue and {name.lower()} step {k}"}})
        else: ev.append({"type":"assistant.message","timestamp":ts,"data":{"content":f"Sure. Here is what I found about the **reminder** queue.\n\n- polls every five minutes\n- stores `loan id` and due time\n\n{code}"}})
    open(d+"/events.jsonl","w").write("\n".join(json.dumps(e) for e in ev)+"\n")
# one long session, newest
sid=str(uuid.UUID(int=1)); d=f"{base}/{sid}"; os.makedirs(d,exist_ok=True); cwd=work("logbook")
open(d+"/workspace.yaml","w").write(f"id: {sid}\nname: Long session with 3000 messages\ncwd: {cwd}\nbranch: main\ncreated_at: {iso(30)}\nupdated_at: {iso(0.2)}\n")
ev=[{"type":"session.start","timestamp":iso(30)}]
table="| step | result |\n|------|--------|\n| scan | ok |\n| index | slow |"
for k in range(1500):
    ts=iso(30-k*0.02)
    ev.append({"type":"user.message","timestamp":ts,"data":{"content":f"Step {k}: can you check the indexer and tell me why `rebuild()` is slow on large histories?"}})
    body=f"Looking at step {k}. The scan walks every session folder, then fingerprints files.\n\n{code}\n\n{table}\n\n"+("That is cheap; the cost is parsing. "*12)
    if k==7: body=("A very long reply. "*200)
    ev.append({"type":"assistant.message","timestamp":ts,"data":{"content":body}})
open(d+"/events.jsonl","w").write("\n".join(json.dumps(e) for e in ev)+"\n")
# Claude sessions with file edits
proj_root=root+"/claude/projects"
for i in range(18):
    repo=repos[(i*2)%len(repos)]; cwd=work(repo)
    cid=str(uuid.UUID(int=5000+i)); proj=f"{proj_root}/"+"".join(c if c.isalnum() else "-" for c in cwd); os.makedirs(proj,exist_ok=True)
    h=i*13+3
    def line(t,hh,msg): return json.dumps(separators=(',',':'),obj={"type":t,"isSidechain":False,"message":msg,"timestamp":iso(hh),"cwd":cwd,"gitBranch":"feat/e2e","sessionId":cid})
    fp=f"{cwd}/src/file{i}.rs"
    L=[line("user",h,{"role":"user","content":f"Why does the tokenizer drop zebra-striped rows? ({i})"}),
       line("assistant",h-0.01,{"id":"m1","role":"assistant","content":[{"type":"text","text":"The row filter treats alternate rows as headers. Fixed in `split_rows`."}]}),
       line("assistant",h-0.02,{"id":"m2","role":"assistant","content":[{"type":"tool_use","id":"e1","name":"Edit","input":{"file_path":fp}}]}),
       json.dumps(separators=(",",":"),obj={"type":"user","isSidechain":False,"message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"e1","content":"ok"}]},"toolUseResult":{"filePath":fp,"structuredPatch":[{"oldStart":7,"oldLines":3,"newStart":7,"newLines":3,"lines":[" fn split_rows(rows: &[Row]) {","-    i % 2 == 0","+    i == 0"," }"]}]},"timestamp":iso(h-0.03),"cwd":cwd,"gitBranch":"feat/e2e","sessionId":cid})]
    open(f"{proj}/{cid}.jsonl","w").write("\n".join(L)+"\n")
print("ok")

"""

BIG = r"""
import sys,os,json,datetime,uuid,random
root=sys.argv[1]; N=int(sys.argv[2]); random.seed(3)
now=datetime.datetime.now(datetime.timezone.utc)
def iso(h): return (now-datetime.timedelta(hours=h)).isoformat()
repos=[f"repo-{i}" for i in range(40)]
words="queue index parser sidebar release search theme updater fixture loan reminder dashboard staging config transcript session agent resume filter range".split()
os.makedirs(root+"/cache",exist_ok=True)
base=root+"/copilot/session-state"
for i in range(N):
    repo=repos[i%len(repos)]; cwd=f"{root}/work/{repo}"
    sid=str(uuid.UUID(int=10_000+i)); d=f"{base}/{sid}"; os.makedirs(d,exist_ok=True)
    h=i*0.7
    w=random.sample(words,3)
    open(d+"/workspace.yaml","w").write(f"id: {sid}\nname: {' '.join(w)} #{i}\ncwd: {cwd}\nbranch: main\ncreated_at: {iso(h+2)}\nupdated_at: {iso(h)}\n")
    ev=[{"type":"session.start","timestamp":iso(h+2)}]
    for k in range(random.randint(2,30)):
        txt=" ".join(random.choices(words,k=random.randint(20,120)))
        ev.append({"type":"user.message" if k%2==0 else "assistant.message","timestamp":iso(h+2-k*0.01),"data":{"content":txt}})
    open(d+"/events.jsonl","w").write("\n".join(json.dumps(e) for e in ev)+"\n")
os.makedirs(f"{root}/claude/projects",exist_ok=True)
print("ok",N)

"""

if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[2] not in ("small", "big"):
        sys.exit(__doc__)
    root, mode = sys.argv[1], sys.argv[2]
    sys.argv = [sys.argv[0], root] + ([sys.argv[3] if len(sys.argv) > 3 else "10000"] if mode == "big" else [])
    if mode == "big":
        sys.argv = [sys.argv[0], root, sys.argv[2]]
    exec(compile(SMALL if mode == "small" else BIG, mode, "exec"), {"__name__": "__main__"})
