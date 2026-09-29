import { useState } from "react";

type Page = "dashboard" | "organization" | "departments" | "positions" | "employees" | "settings";

const menu: {id:Page;label:string}[] = [
  {id:"dashboard",label:"Dashboard"},
  {id:"organization",label:"Organization Chart"},
  {id:"departments",label:"หน่วยงาน"},
  {id:"positions",label:"ตำแหน่ง"},
  {id:"employees",label:"ข้อมูลพนักงาน"},
  {id:"settings",label:"ตั้งค่า"}
];

export function App(){
  const [loggedIn,setLoggedIn]=useState(false);
  const [page,setPage]=useState<Page>("dashboard");

  if(!loggedIn) return <main className="login-shell">
    <section className="login-card">
      <div className="brand-mark">RM</div>
      <h1>Roster Master Desktop</h1>
      <p>ระบบบริหารอัตรากำลัง • Offline-first</p>
      <label>ชื่อผู้ใช้<input placeholder="Username" /></label>
      <label>รหัสผ่าน<input type="password" placeholder="Password" /></label>
      <button onClick={()=>setLoggedIn(true)}>เข้าสู่ระบบ (Prototype)</button>
      <small>Phase 1 UI shell — authentication backend is not connected yet.</small>
    </section>
  </main>;

  return <div className="app-shell">
    <aside>
      <div className="app-brand"><b>Roster Master</b><span>Desktop v0.1</span></div>
      <nav>{menu.map(m=><button key={m.id} className={page===m.id?"active":""} onClick={()=>setPage(m.id)}>{m.label}</button>)}</nav>
      <div className="user-box"><b>ผู้ใช้งานตัวอย่าง</b><span>สิทธิ์: Prototype</span><button onClick={()=>setLoggedIn(false)}>ออกจากระบบ</button></div>
    </aside>
    <main className="content">
      <header><div><h1>{menu.find(x=>x.id===page)?.label}</h1><p>ต้นแบบ Desktop อ้างอิง Roster Master v5.55</p></div><div className="offline">● Offline Ready</div></header>
      {page==="dashboard"?<Dashboard/>:<Placeholder title={menu.find(x=>x.id===page)?.label||""}/>}
    </main>
  </div>
}

function Dashboard(){
  return <>
    <section className="cards">
      <Metric title="พนักงานทั้งหมด" value="—" note="รอเชื่อม SQLite"/>
      <Metric title="กรอบอัตรา" value="—" note="Target HC"/>
      <Metric title="FTE ปัจจุบัน" value="—" note="คำนวณจาก Assignment"/>
      <Metric title="ตำแหน่งว่าง" value="—" note="Target - Actual"/>
    </section>
    <section className="panel"><h2>อัตรากำลังรายบริษัท</h2><p>พื้นที่กราฟจาก v5.55 — จะเชื่อม Company / Target HC / Actual HC / FTE ในขั้นถัดไป</p><div className="chart-placeholder">Company Workforce Chart</div></section>
    <section className="panel"><h2>พนักงานใกล้ครบทดลองงาน 90 วัน</h2><p>คงแนวคิด Dashboard จาก v5.55 และจะคำนวณจากวันที่เริ่มงาน</p></section>
  </>
}
function Metric(p:{title:string;value:string;note:string}){return <article className="metric"><span>{p.title}</span><strong>{p.value}</strong><small>{p.note}</small></article>}
function Placeholder({title}:{title:string}){return <section className="panel"><h2>{title}</h2><p>โมดูลนี้จะย้ายจาก Roster Master v5.55 แบบ incremental migration</p></section>}
