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
 return <div className="app-shell"><aside><div className="app-brand"><b>Roster Master</b><span>Desktop v0.1</span></div><nav>{allowed.map(m=><button key={m.id} className={page===m.id?"active":""} onClick={()=>setPage(m.id)}>{m.label}</button>)}</nav><div className="user-box"><b>{session.display_name}</b><span>{session.username}</span><button onClick={()=>setSession(null)}>ออกจากระบบ</button></div></aside><main className="content"><header><div><h1>{menu.find(x=>x.id===page)?.label}</h1><p>Desktop จาก Roster Master v5.55</p></div><div className="offline">● Local SQLite</div></header>{page==="dashboard"?<Dashboard/>:<Placeholder title={menu.find(x=>x.id===page)?.label||""}/>}</main></div>
}
function Dashboard(){
 const [s,setS]=useState<Summary|null>(null),[error,setError]=useState("");
 useEffect(()=>{invoke<Summary>("dashboard_summary").then(setS).catch(e=>setError(String(e)))},[]);
 return <>{error&&<div className="error">{error}</div>}<section className="cards"><Metric title="พนักงานทั้งหมด" value={s?String(s.employees):"…"} note="Active HC"/><Metric title="กรอบอัตรา" value={s?s.target_hc.toFixed(2):"…"} note="Target HC"/><Metric title="FTE ปัจจุบัน" value={s?s.fte.toFixed(2):"…"} note="Effective Assignment"/><Metric title="ตำแหน่งว่าง" value={s?s.vacancies.toFixed(2):"…"} note="Target - FTE"/></section><section className="panel"><h2>อัตรากำลังรายบริษัท</h2><p>โมดูลถัดไปจะอ่าน Company / Target HC / Actual HC / FTE จาก SQLite</p><div className="chart-placeholder">Company Workforce Chart</div></section><section className="panel"><h2>พนักงานใกล้ครบทดลองงาน 90 วัน</h2><p>คงกฎจาก v5.55 และจะคำนวณจากวันที่เริ่มงานใน SQLite</p></section></>
}
function Metric(p:{title:string;value:string;note:string}){return <article className="metric"><span>{p.title}</span><strong>{p.value}</strong><small>{p.note}</small></article>}
function Placeholder({title}:{title:string}){return <section className="panel"><h2>{title}</h2><p>โมดูลนี้กำลังย้ายจาก Roster Master v5.55</p></section>}
