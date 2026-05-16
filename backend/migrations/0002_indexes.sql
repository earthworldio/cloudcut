-- ตรวจสอบสิทธิ์ผู้ใช้และค้นหาผ่าน Email ตอนทำระบบ Auth
CREATE UNIQUE INDEX idx_users_email ON users(email);

-- ค้นหา Workspace ผ่าน URL Friendly Slug
CREATE UNIQUE INDEX idx_workspaces_slug ON workspaces(slug);

-- ตรวจสอบสิทธิ์และดึงข้อมูลโปรเจกต์แยกตาม Workspace โดยเรียงจากโปรเจกต์ที่อัปเดตล่าสุด (Cursor Pagination)
CREATE INDEX idx_projects_workspace_updated ON projects (workspace_id, updated_at DESC) WHERE deleted_at IS NULL;

-- ดึงรายการวัตถุดิบ (Assets) ภายในโปรเจกต์ เพื่อคัดกรองเฉพาะตัวที่ประมวลผลเสร็จแล้วมาแสดงบน UI
CREATE INDEX idx_assets_project_status ON assets (project_id, status) WHERE deleted_at IS NULL;

-- ดึงข้อมูล Track ทั้งหมดในโปรเจกต์ เรียงตามลำดับความสูง-ต่ำ บน Timeline (Layering)
CREATE INDEX idx_tracks_project_order ON tracks (project_id, order_index);

-- ดึงข้อมูลคลิปวิดีโอ เพื่อดูว่าคลิปไหนตั้งอยู่ตำแหน่งมิลลิวินาทีใด บนแทร็กไหนในหน้า Timeline
CREATE INDEX idx_clips_project_track_pos ON clips (project_id, track_id, track_position_ms) WHERE deleted_at IS NULL;

CREATE INDEX idx_clips_active ON clips (project_id) WHERE deleted_at IS NULL;
-- ใช้สำหรับดึงเฉพาะคลิปที่ยังไม่ถูกลบทั้งหมดในโปรเจกต์นั้นๆ ออกมาสร้างสถานะเริ่มต้นบนหน้าจอ

-- เรียงลำดับการซ้อนทับกันของเอฟเฟกต์ (Effect Pipeline) ภายในคลิปเดี่ยว
CREATE INDEX idx_clip_effects_clip_order ON clip_effects (clip_id, order_index);

-- ใช้ในการทำ Offline Reconnect ดึงประวัติคำสั่งแก้ไขย้อนหลังหลังจากหลุดการเชื่อมต่อ (Sync)
CREATE INDEX idx_operation_logs_project_seq ON operation_logs (project_id, server_seq DESC);

-- ป้องกันการกด Export ซ้ำซ้อน (Idempotency Guard) ในระบบคิว
CREATE UNIQUE INDEX idx_export_jobs_idempotency ON export_jobs (idempotency_key);
CREATE UNIQUE INDEX idx_processing_jobs_idempotency ON processing_jobs (idempotency_key);