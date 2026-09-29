import { FormEvent, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Page="dashboard"|"organization"|"departments"|"positions"|"employees"|"settings";
type Session={user_id:string;username:string;display_name:string;permissions:string[]};
type Summary={employees:number;target_hc:number;fte:number;vacancies:number};
const menu:{id:Page;label:string;permission:string}[]=[
 {id:"dashboard",label:"Dashboard",permission:"dashboard.view"},{id:"organization",label:"Organization Chart",permission:"department.view"},
 {id:"departments",label:"หน่วยงาน",permission:"department.view"},{id:"positions",label:"ตำแหน่ง",permission:"position.view"},
 {id:"employees",label:"ข้อมูลพนักงาน",permission:"employee.view"},{id:"settings",label:"ตั้งค่า",permission:"settings.manage"}
];

export function App(){
 const [setup,setSetup]=useState<boolean|null>(null),[session,setSession]=useState<Session|null>(null),[page,setPage]=useState<Page>("dashboard"),[error,setError]=useState("");
 useEffect(()=>{invoke<boolean>("needs_setup").then(setSetup).catch(e=>setError(String(e)))},[]);
 async function submit(e:FormEvent<HTMLFormElement>){
  e.preventDefault();setError("");const f=new FormData(e.currentTarget),username=String(f.get("username")||""),password=String(f.get("password")||"");
  try{
   if(setup){await invoke("create_first_admin",{username,password,displayName:String(f.get("displayName")||"")});setSetup(false)}
   else setSession(await invoke<Session>("login",{username,password}));
  }catch(e){setError(String(e))}
 }
 if(setup===null)return <main className="login-shell"><section className="login-card"><h1>Roster Master Desktop</h1><p>กำลังเปิดฐานข้อมูลในเครื่อง…</p>{error&&<div className="error">{error}</div>}</section></main>;
 if(!session)return <main className="login-shell"><form className="login-card" onSubmit={submit}><div className="brand-mark">RM</div><h1>{setup?"ตั้งค่าระบบครั้งแรก":"Roster Master Desktop"}</h1><p>{setup?"สร้างบัญชี System Admin สำหรับเครื่องนี้":"ระบบบริหารอัตรากำลัง • Offline-first"}</p>{setup&&<label>ชื่อผู้ดูแลระบบ<input name="displayName" required/></label>}<label>ชื่อผู้ใช้<input name="username" required autoFocus/></label><label>รหัสผ่าน<input name="password" type="password" minLength={setup?10:1} required/></label>{error&&<div className="error">{error}</div>}<button>{setup?"สร้างผู้ดูแลระบบ":"เข้าสู่ระบบ"}</button><small>{setup?"รหัสผ่านอย่างน้อย 10 ตัวอักษร":"ตรวจสอบบัญชีจาก SQLite ในเครื่อง"}</small></form></main>;
 const allowed=menu.filter(m=>session.permissions.includes(m.permission));
 return <div className="app-shell"><aside><div className="app-brand"><b>Roster Master</b><span>Desktop v0.1</span></div><nav>{allowed.map(m=><button key={m.id} className={page===m.id?"active":""} onClick={()=>setPage(m.id)}>{m.label}</button>)}</nav><div className="user-box"><b>{session.display_name}</b><span>{session.username}</span><button onClick={()=>setSession(null)}>ออกจากระบบ</button></div></aside><main className="content"><header><div><h1>{menu.find(x=>x.id===page)?.label}</h1><p>Desktop จาก Roster Master v5.55</p></div><div className="offline">● Local SQLite</div></header>{page==="dashboard"?<Dashboard/>:page==="departments"?<Departments session={session}/>:page==="positions"?<Positions session={session}/>:<Placeholder title={menu.find(x=>x.id===page)?.label||""}/>}</main></div>
}
function Dashboard(){
 const [s,setS]=useState<Summary|null>(null),[error,setError]=useState("");
 useEffect(()=>{invoke<Summary>("dashboard_summary").then(setS).catch(e=>setError(String(e)))},[]);
 return <>{error&&<div className="error">{error}</div>}<section className="cards"><Metric title="พนักงานทั้งหมด" value={s?String(s.employees):"…"} note="Active HC"/><Metric title="กรอบอัตรา" value={s?s.target_hc.toFixed(2):"…"} note="Target HC"/><Metric title="FTE ปัจจุบัน" value={s?s.fte.toFixed(2):"…"} note="Effective Assignment"/><Metric title="ตำแหน่งว่าง" value={s?s.vacancies.toFixed(2):"…"} note="Target - FTE"/></section><section className="panel"><h2>อัตรากำลังรายบริษัท</h2><p>โมดูลถัดไปจะอ่าน Company / Target HC / Actual HC / FTE จาก SQLite</p><div className="chart-placeholder">Company Workforce Chart</div></section><section className="panel"><h2>พนักงานใกล้ครบทดลองงาน 90 วัน</h2><p>คงกฎจาก v5.55 และจะคำนวณจากวันที่เริ่มงานใน SQLite</p></section></>
}
function Metric(p:{title:string;value:string;note:string}){return <article className="metric"><span>{p.title}</span><strong>{p.value}</strong><small>{p.note}</small></article>}
function Placeholder({title}:{title:string}){return <section className="panel"><h2>{title}</h2><p>โมดูลนี้กำลังย้ายจาก Roster Master v5.55</p></section>}

type Group={id:string;code:string;name:string}; type Company={id:string;code:string;name:string;company_group_id?:string}; type Unit={id:string;code:string;name:string;company_id:string;parent_id?:string;unit_type:string}; type Pos={id:string;code:string;name:string;org_unit_id:string;grade?:number;target_hc:number};

function Departments({session}:{session:Session}){
 const [groups,setGroups]=useState<Group[]>([]),[companies,setCompanies]=useState<Company[]>([]),[units,setUnits]=useState<Unit[]>([]),[error,setError]=useState("");
 const load=()=>Promise.all([invoke<Group[]>("list_company_groups",{userId:session.user_id}),invoke<Company[]>("list_companies",{userId:session.user_id}),invoke<Unit[]>("list_org_units",{userId:session.user_id})]).then(([g,c,u])=>{setGroups(g);setCompanies(c);setUnits(u)}).catch(e=>setError(String(e)));
 useEffect(()=>{load()},[]);
 async function addGroup(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);try{await invoke("add_company_group",{userId:session.user_id,code:String(f.get("code")),name:String(f.get("name"))});e.currentTarget.reset();load()}catch(e){setError(String(e))}}
 async function addCompany(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);try{await invoke("add_company",{userId:session.user_id,code:String(f.get("code")),name:String(f.get("name")),companyGroupId:String(f.get("group")||"")||null});e.currentTarget.reset();load()}catch(e){setError(String(e))}}
 async function addUnit(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);try{await invoke("add_org_unit",{userId:session.user_id,code:String(f.get("code")),name:String(f.get("name")),companyId:String(f.get("company")),parentId:String(f.get("parent")||"")||null,unitType:String(f.get("type"))});e.currentTarget.reset();load()}catch(e){setError(String(e))}}
 return <>{error&&<div className="error">{error}</div>}<section className="panel"><h2>โครงสร้างองค์กร</h2><p>กลุ่มบริษัท → บริษัท → ส่วน/แผนก/หน่วยงาน จาก SQLite</p><div className="org-grid">{companies.map(c=><article className="org-company" key={c.id}><b>{c.code} · {c.name}</b><small>{groups.find(g=>g.id===c.company_group_id)?.name||"ไม่ระบุกลุ่มบริษัท"}</small>{units.filter(u=>u.company_id===c.id).map(u=><div className="org-unit" key={u.id}>{u.code} · {u.name}<span>{u.unit_type}</span></div>)}</article>)}</div></section>{session.permissions.includes("department.manage")&&<section className="panel"><h2>เพิ่มโครงสร้าง</h2><div className="forms"><form onSubmit={addGroup}><h3>กลุ่มบริษัท</h3><input name="code" placeholder="รหัส" required/><input name="name" placeholder="ชื่อกลุ่มบริษัท" required/><button>เพิ่ม</button></form><form onSubmit={addCompany}><h3>บริษัท</h3><input name="code" placeholder="รหัส" required/><input name="name" placeholder="ชื่อบริษัท" required/><select name="group"><option value="">ไม่ระบุกลุ่ม</option>{groups.map(g=><option key={g.id} value={g.id}>{g.name}</option>)}</select><button>เพิ่ม</button></form><form onSubmit={addUnit}><h3>หน่วยงาน</h3><input name="code" placeholder="รหัส" required/><input name="name" placeholder="ชื่อหน่วยงาน" required/><select name="company" required><option value="">เลือกบริษัท</option>{companies.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select><select name="parent"><option value="">ไม่มีหน่วยงานแม่</option>{units.map(u=><option key={u.id} value={u.id}>{u.name}</option>)}</select><select name="type"><option value="SECTION">ส่วน</option><option value="DEPARTMENT">แผนก</option><option value="UNIT">หน่วยงาน</option></select><button>เพิ่ม</button></form></div></section>}</>
}
function Positions({session}:{session:Session}){
 const [positions,setPositions]=useState<Pos[]>([]),[units,setUnits]=useState<Unit[]>([]),[error,setError]=useState("");
 const load=()=>Promise.all([invoke<Pos[]>("list_positions",{userId:session.user_id}),invoke<Unit[]>("list_org_units",{userId:session.user_id})]).then(([p,u])=>{setPositions(p);setUnits(u)}).catch(e=>setError(String(e)));
 useEffect(()=>{load()},[]);
 async function add(e:FormEvent<HTMLFormElement>){e.preventDefault();const f=new FormData(e.currentTarget);try{await invoke("add_position",{userId:session.user_id,code:String(f.get("code")),name:String(f.get("name")),orgUnitId:String(f.get("unit")),grade:f.get("grade")?Number(f.get("grade")):null,targetHc:Number(f.get("target"))});e.currentTarget.reset();load()}catch(e){setError(String(e))}}
 return <>{error&&<div className="error">{error}</div>}<section className="panel"><h2>ตำแหน่งและกรอบอัตรา</h2><div className="table-wrap"><table><thead><tr><th>รหัส</th><th>ตำแหน่ง</th><th>หน่วยงาน</th><th>Grade</th><th>กรอบ HC</th></tr></thead><tbody>{positions.map(p=><tr key={p.id}><td>{p.code}</td><td>{p.name}</td><td>{units.find(u=>u.id===p.org_unit_id)?.name||"-"}</td><td>{p.grade??"-"}</td><td>{p.target_hc}</td></tr>)}</tbody></table></div></section>{session.permissions.includes("position.manage")&&<section className="panel"><h2>เพิ่มตำแหน่ง</h2><form className="inline-form" onSubmit={add}><input name="code" placeholder="รหัสตำแหน่ง" required/><input name="name" placeholder="ชื่อตำแหน่ง" required/><select name="unit" required><option value="">เลือกหน่วยงาน</option>{units.map(u=><option key={u.id} value={u.id}>{u.name}</option>)}</select><input name="grade" type="number" min="1" max="99" placeholder="Grade"/><input name="target" type="number" min="0" step="0.01" defaultValue="1" required/><button>เพิ่มตำแหน่ง</button></form></section>}</>
}
