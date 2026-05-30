export const FlowExportPage = () => {
  const steps = [
    {
      group: "Frontend",
      file: "ExportModal.tsx",
      color: "#3b82f6",
      items: [
        { id: "1.1", title: "POST /api/projects/:id/exports", desc: "ส่ง { format, resolution } ไปที่ API — ถ้า resolution='4k' API จะโยนงานไป Lambda" },
        { id: "1.2", title: "รับ exportId กลับมา", desc: "API return ทันที พร้อม exportId → เริ่ม pollStatus() ทุก 2 วิ" },
        { id: "1.3", title: "GET /exports/:exportId", desc: "ดึงสถานะทุก 2 วิ จนกว่าจะไม่ใช่ queued/processing" },
        { id: "1.4", title: "ยังไม่เสร็จ → poll รอบถัดไป", desc: "setTimeout 2000ms แล้วเรียก pollStatus() ซ้ำ" },
        { id: "1.5", title: "completed → แสดงปุ่ม Download", desc: "React render ปุ่ม Download พร้อม presigned URL จาก S3" },
      ]
    },
    {
      group: "API — create_export()",
      file: "backend/api/src/handlers/projects.rs",
      color: "#10b981",
      items: [
        { id: "2.1", title: "check_project_access()", desc: "ตรวจสิทธิ์ — user ต้องเป็น member ของ workspace ที่ project อยู่" },
        { id: "2.2", title: "เช็ค concurrent limit", desc: "นับ job ที่ยัง queued/processing — free=2, pro=10, team=30" },
        { id: "2.3", title: "INSERT export_jobs", desc: "บันทึก job ลง DB สถานะ 'queued' พร้อม idempotency_key ป้องกัน duplicate" },
        { id: "2.4", title: "resolution='4k' → POST Lambda URL", desc: "tokio::spawn HTTP POST ไปหา Lambda Function URL แบบ async (ไม่รอผล)" },
        { id: "2.5", title: "resolution อื่น → Redis Queue", desc: "LPUSH เข้า queue:video_pipeline → Worker Docker BRPOP ไปประมวลผล" },
        { id: "2.6", title: "commit + return exportId", desc: "Commit transaction แล้ว return { exportId, status: 'queued' } ให้ Frontend" },
      ]
    },
    {
      group: "Lambda Handler",
      file: "lambda/export-4k/src/main.rs → handler()",
      color: "#f59e0b",
      items: [
        { id: "3.1", title: "Parse body จาก Function URL", desc: "Function URL ห่อ body ใน field 'body' — ดึงออกมา parse เป็น { project_id, export_id }" },
        { id: "3.2", title: "เรียก run_export()", desc: "ส่งเข้า pipeline หลัก — ถ้า error return status='failed' ไม่ retry" },
      ]
    },
    {
      group: "Lambda Pipeline",
      file: "lambda/export-4k/src/main.rs → run_export()",
      color: "#ef4444",
      items: [
        { id: "4.1", title: "Connect Neon DB + S3", desc: "เชื่อมต่อ PostgreSQL (Neon) และ build S3 client จาก Lambda execution role" },
        { id: "4.2", title: "UPDATE status = 'processing'", desc: "บันทึกเวลาเริ่ม + เปลี่ยนสถานะ → Frontend เห็นผ่าน poll รอบถัดไป" },
        { id: "4.3", title: "ดึง resolution → ffmpeg settings", desc: "4k: 3840×2160 60fps CRF18 preset:slow / 1080p: 1920×1080 30fps CRF23" },
        { id: "4.4", title: "SELECT clips จาก timeline", desc: "ดึง clips จาก video track เรียงตาม track_position_ms (ลำดับบน timeline)" },
        { id: "4.5", title: "mkdir /tmp/export_{id}", desc: "สร้าง temp dir ใน /tmp ของ Lambda (มี disk สูงสุด 10GB)" },
        { id: "4.6", title: "Presigned URL ของ asset", desc: "สร้าง presigned URL จาก S3 key ใน DB อายุ 1 ชั่วโมง — ffmpeg ใช้ download ต้นฉบับ" },
        { id: "4.7", title: "ffmpeg trim clip", desc: "ตัดตาม in_point-out_point แล้ว encode เป็น 4K 60fps — ทุก segment ต้องมี format เดียวกัน" },
        { id: "4.8", title: "UPDATE progress 0-80%", desc: "อัปเดต progress หลัง trim แต่ละ clip เสร็จ — Frontend เห็นค่าเพิ่มขึ้นทุกรอบ poll" },
        { id: "4.9", title: "ffmpeg concat -c copy", desc: "รวมทุก segment ด้วย concat demuxer + stream copy (ไม่ encode ใหม่ เร็วมาก)" },
        { id: "4.10", title: "Upload S3 → exports/.../id.mp4", desc: "อ่านไฟล์เข้า memory → PUT ขึ้น S3 bucket" },
        { id: "4.11", title: "UPDATE status = 'completed'", desc: "บันทึก output_url (S3 key) + progress=100 → Frontend แสดงปุ่ม Download" },
        { id: "4.12", title: "ลบ /tmp/export_{id}", desc: "คืน disk space ให้ Lambda invocation ถัดไป" },
      ]
    },
  ];

  const summary = [
    "1.1 กดปุ่ม", "→", "2.1 ตรวจสิทธิ์", "→", "2.3 สร้าง Job", "→",
    "2.4 POST Lambda", "→", "3.1 Parse event", "→", "4.2 processing",
    "→", "4.7 ffmpeg trim×N", "→", "4.9 concat", "→", "4.10 S3 upload",
    "→", "4.11 completed", "→", "1.5 Download"
  ];

  return (
    <div style={{ background: "#0a0a0a", minHeight: "100vh", padding: "48px 32px", fontFamily: "ui-monospace, monospace", color: "#e5e5e5", maxWidth: 900, margin: "0 auto" }}>
      <div style={{ marginBottom: 48 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, margin: "0 0 8px" }}>4K Export Flow</h1>
        <p style={{ color: "#6b7280", margin: 0, fontSize: 13 }}>
          Search <code style={{ background: "#1c1c1c", padding: "2px 8px", borderRadius: 4, color: "#f59e0b" }}>/* 2.4</code> ในโค้ดเพื่อ jump ไปยัง step นั้นได้เลย
        </p>
      </div>

      {/* Summary bar */}
      <div style={{ background: "#111", border: "1px solid #222", borderRadius: 10, padding: "16px 20px", marginBottom: 48, overflowX: "auto" }}>
        <div style={{ fontSize: 11, color: "#6b7280", marginBottom: 8, textTransform: "uppercase", letterSpacing: 1 }}>Full Flow</div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap", fontSize: 12 }}>
          {summary.map((s, i) => (
            <span key={i} style={{ color: s === "→" ? "#374151" : "#d1d5db", whiteSpace: "nowrap" }}>{s}</span>
          ))}
        </div>
      </div>

      {steps.map((group) => (
        <div key={group.group} style={{ marginBottom: 40 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 10, marginBottom: 16 }}>
            <span style={{ fontSize: 13, fontWeight: 700, color: group.color }}>{group.group}</span>
            <span style={{ fontSize: 11, color: "#4b5563" }}>{group.file}</span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {group.items.map((step, i) => (
              <div key={step.id} style={{ display: "grid", gridTemplateColumns: "52px 1fr", gap: 0, position: "relative" }}>
                {/* left line */}
                <div style={{ display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 14 }}>
                  <div style={{ width: 8, height: 8, borderRadius: "50%", background: group.color, flexShrink: 0 }} />
                  {i < group.items.length - 1 && (
                    <div style={{ width: 1, flexGrow: 1, background: `${group.color}30`, marginTop: 4 }} />
                  )}
                </div>

                {/* content */}
                <div style={{ padding: "10px 16px 10px 8px", marginBottom: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 3 }}>
                    <span style={{ fontSize: 11, color: group.color, fontWeight: 700, minWidth: 32 }}>{step.id}</span>
                    <span style={{ fontSize: 13, fontWeight: 600, color: "#f3f4f6" }}>{step.title}</span>
                  </div>
                  <div style={{ fontSize: 12, color: "#6b7280", paddingLeft: 42, lineHeight: 1.6 }}>{step.desc}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};
