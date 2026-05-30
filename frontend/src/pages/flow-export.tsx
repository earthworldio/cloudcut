export const FlowExportPage = () => {
  const steps = [
    {
      group: "Frontend (ExportModal.tsx)",
      color: "#3b82f6",
      items: [
        { id: "1.0", title: "ผู้ใช้กดปุ่ม Start Export", desc: "เลือก resolution (720p / 1080p / 4K 60fps) แล้วกดปุ่ม → startExport() ถูกเรียก" },
        { id: "1.1", title: "POST ไป API", desc: "ส่ง { format: 'mp4', resolution: '4k' } ไปที่ /api/projects/:id/exports" },
        { id: "1.2", title: "รับ exportId กลับมา", desc: "API ตอบกลับด้วย exportId → เริ่ม pollStatus() ทุก 2 วินาที" },
        { id: "1.3", title: "Poll สถานะ", desc: "GET /api/projects/:id/exports/:exportId ทุก 2 วิ รอจนสถานะไม่ใช่ queued/processing" },
        { id: "1.4", title: "แสดงผล", desc: "completed → แสดงปุ่ม Download พร้อม presigned URL จาก S3 / failed → แสดง error" },
      ]
    },
    {
      group: "API — projects.rs (create_export)",
      color: "#10b981",
      items: [
        { id: "2.0", title: "ตรวจสอบสิทธิ์", desc: "check_project_access() ตรวจว่า user มีสิทธิ์เข้าถึง project นี้ผ่าน workspace_members" },
        { id: "2.1", title: "ดึง Plan + Limit", desc: "ดึง plan ของ workspace (free=2, pro=10, team=30) เพื่อกำหนด concurrent export limit" },
        { id: "2.2", title: "เช็ค Concurrent Export", desc: "นับ export jobs ที่ status = queued/processing ถ้าเกิน limit → reject 403 ทันที" },
        { id: "2.3", title: "สร้าง Export Job ใน DB", desc: "INSERT export_jobs ด้วยสถานะ 'queued', progress 0%, พร้อม idempotency_key ป้องกันซ้ำ" },
        { id: "2.4", title: "แยก Route ตาม Resolution", desc: "resolution == '4k' → Lambda / resolution อื่น → Redis Queue → Worker Docker" },
        { id: "2.5", title: "อ่าน Lambda Function URL", desc: "อ่าน LAMBDA_EXPORT_4K_URL จาก env var — URL ของ Lambda บน AWS โดยตรง" },
        { id: "2.6", title: "Invoke Lambda แบบ Async", desc: "tokio::spawn POST ไปหา Lambda Function URL พร้อม project_id + export_id แล้ว return ทันที ไม่รอผล" },
        { id: "2.7", title: "Redis Queue (non-4K)", desc: "720p/1080p → LPUSH เข้า queue:video_pipeline → Worker Docker รับไปประมวลผล" },
        { id: "2.8", title: "Commit + Response", desc: "Commit DB transaction → ตอบ { exportId, status: 'queued' } กลับ Frontend" },
      ]
    },
    {
      group: "Lambda Handler (lambda/export-4k/main.rs)",
      color: "#f59e0b",
      items: [
        { id: "3.0", title: "รับ Event จาก Function URL", desc: "Lambda ได้รับ HTTP POST — Function URL ส่ง body มาใน field 'body' เป็น JSON string" },
        { id: "3.1", title: "Parse Event", desc: "รองรับ 2 format: Function URL (body field) และ direct SDK invoke (JSON ตรงๆ)" },
        { id: "3.2", title: "เรียก run_export()", desc: "ส่ง project_id + export_id เข้า pipeline หลัก ถ้า error → update DB status = failed" },
      ]
    },
    {
      group: "Lambda Pipeline — run_export()",
      color: "#ef4444",
      items: [
        { id: "4.0", title: "Connect DB + S3", desc: "เชื่อมต่อ Neon PostgreSQL ผ่าน DATABASE_URL และ build S3 client จาก Lambda execution role" },
        { id: "4.1", title: "Update Status = processing", desc: "UPDATE export_jobs SET status='processing', started_at=NOW() → Frontend เห็นสถานะเปลี่ยน" },
        { id: "4.2", title: "ดึง Resolution → FFmpeg Settings", desc: "4K: 3840x2160 60fps CRF18 preset slow / 1080p: 1920x1080 30fps CRF23 preset veryfast" },
        { id: "4.3", title: "ดึง Clips จาก Timeline", desc: "SELECT clips JOIN tracks WHERE type='video' ORDER BY track_position_ms — ได้ลำดับ clip บน timeline" },
        { id: "4.4", title: "สร้าง Temp Directory", desc: "mkdir /tmp/export_{export_id} — Lambda มี /tmp สูงสุด 10GB สำหรับเก็บไฟล์ระหว่าง process" },
        { id: "4.5-4.8", title: "Trim แต่ละ Clip ด้วย FFmpeg", desc: "สำหรับทุก clip: สร้าง presigned URL → ffmpeg trim ช่วง in-out → encode 4K 60fps → update progress 0-80%" },
        { id: "4.9", title: "Concat ทุก Segment", desc: "เขียน segments.txt → ffmpeg concat demuxer ด้วย -c copy (ไม่ encode ใหม่) → final_output.mp4" },
        { id: "4.10", title: "Upload ขึ้น S3", desc: "อ่านไฟล์เข้า memory → PUT ขึ้น S3 key: exports/{project_id}/{export_id}.mp4" },
        { id: "4.11", title: "Update Status = completed", desc: "UPDATE export_jobs SET status='completed', progress=100, output_url=key, completed_at=NOW()" },
        { id: "4.12", title: "Cleanup Temp Files", desc: "rm -rf /tmp/export_{export_id} คืน disk space ให้ Lambda invocation ถัดไป" },
      ]
    },
  ];

  return (
    <div style={{ background: "#0f0f0f", minHeight: "100vh", padding: "40px 24px", fontFamily: "monospace", color: "#e5e5e5" }}>
      <h1 style={{ fontSize: 28, fontWeight: 700, marginBottom: 8 }}>4K Export Flow</h1>
      <p style={{ color: "#6b7280", marginBottom: 48, fontSize: 14 }}>
        ติดตาม flow ได้ด้วยการ search หมายเลข เช่น <code style={{ background: "#1f1f1f", padding: "2px 6px", borderRadius: 4 }}>/* 2.6</code> ในโค้ด
      </p>

      {steps.map((group) => (
        <div key={group.group} style={{ marginBottom: 48 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
            <div style={{ width: 12, height: 12, borderRadius: "50%", background: group.color }} />
            <h2 style={{ fontSize: 16, fontWeight: 600, color: group.color, margin: 0 }}>{group.group}</h2>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 2, paddingLeft: 24, borderLeft: `2px solid ${group.color}33` }}>
            {group.items.map((step, i) => (
              <div key={step.id} style={{ display: "flex", gap: 16, padding: "12px 16px", background: "#1a1a1a", borderRadius: 8, position: "relative" }}>
                <div style={{ position: "absolute", left: -31, top: "50%", transform: "translateY(-50%)", width: 10, height: 10, borderRadius: "50%", background: group.color }} />

                <div style={{ minWidth: 48, fontSize: 12, color: group.color, fontWeight: 700, paddingTop: 2 }}>
                  {step.id}
                </div>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>{step.title}</div>
                  <div style={{ fontSize: 13, color: "#9ca3af", lineHeight: 1.6 }}>{step.desc}</div>
                </div>

                {i < group.items.length - 1 && (
                  <div style={{ position: "absolute", left: -27, top: "calc(50% + 8px)", width: 2, height: 18, background: `${group.color}33` }} />
                )}
              </div>
            ))}
          </div>
        </div>
      ))}

      <div style={{ background: "#1a1a1a", borderRadius: 12, padding: 24, marginTop: 48 }}>
        <h3 style={{ margin: "0 0 16px", fontSize: 14, color: "#6b7280" }}>Full Flow Summary</h3>
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap", fontSize: 13 }}>
          {["UI กดปุ่ม", "→", "POST /exports", "→", "API สร้าง Job", "→", "invoke Lambda URL", "→", "FFmpeg trim clips", "→", "concat", "→", "upload S3", "→", "DB completed", "→", "Frontend download"].map((s, i) => (
            <span key={i} style={{ color: s === "→" ? "#4b5563" : "#e5e5e5" }}>{s}</span>
          ))}
        </div>
      </div>
    </div>
  );
};
