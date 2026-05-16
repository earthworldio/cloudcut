-- 1. SEED USERS (จำลอง Password hash รหัส 'password123')
INSERT INTO users (id, email, password_hash, name) VALUES
('018e3a20-0001-7000-8000-000000000001', 'alice@cloudcut.com', '$argon2id$v=19$m=19456,t=2,p=1$bW9ja3NhbHQ$M0pZdVptVlBkaW05clY4M05Wb0F6UT09', 'Alice Baker'),
('018e3a20-0002-7000-8000-000000000002', 'bob@cloudcut.com', '$argon2id$v=19$m=19456,t=2,p=1$bW9ja3NhbHQ$M0pZdVptVlBkaW05clY4M05Wb0F6UT09', 'Bob Carter');

-- 2. SEED WORKSPACE
INSERT INTO workspaces (id, name, slug, plan, owner_id) VALUES
('018e3a20-0003-7000-8000-000000000003', 'Production Studio A', 'prod-studio-a', 'pro', '018e3a20-0001-7000-8000-000000000001');

-- 3. SEED WORKSPACE MEMBERS
INSERT INTO workspace_members (id, workspace_id, user_id, role) VALUES
('018e3a20-0004-7000-8000-000000000004', '018e3a20-0003-7000-8000-000000000003', '018e3a20-0001-7000-8000-000000000001', 'owner'),
('018e3a20-0005-7000-8000-000000000005', '018e3a20-0003-7000-8000-000000000003', '018e3a20-0002-7000-8000-000000000002', 'editor');

-- 4. SEED PROJECTS
INSERT INTO projects (id, workspace_id, name, description, created_by) VALUES
('018e3a20-0006-7000-8000-000000000006', '018e3a20-0003-7000-8000-000000000003', 'Promo Video v1', 'Main commercial project', '018e3a20-0001-7000-8000-000000000001'),
('018e3a20-0007-7000-8000-000000000007', '018e3a20-0003-7000-8000-000000000003', 'TikTok Cuts', 'Short-form dynamic edits', '018e3a20-0001-7000-8000-000000000001');

-- 5. SEED ASSETS
INSERT INTO assets (id, project_id, uploaded_by, type, original_url, status, metadata) VALUES
('018e3a20-0008-7000-8000-000000000008', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0001-7000-8000-000000000001', 'video', 'https://storage.cloudcut.internal/raw/interview.mp4', 'ready', '{"duration_ms": 60000, "width": 1920, "height": 1080, "codec": "h264"}'),
('018e3a20-0009-7000-8000-000000000009', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0001-7000-8000-000000000001', 'video', 'https://storage.cloudcut.internal/raw/b-roll.mp4', 'ready', '{"duration_ms": 30000, "width": 1920, "height": 1080, "codec": "h264"}'),
('018e3a20-0010-7000-8000-000000000010', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0002-7000-8000-000000000002', 'audio', 'https://storage.cloudcut.internal/raw/bg-music.mp3', 'ready', '{"duration_ms": 120000, "codec": "mp3"}');

-- 6. SEED TRACKS (4 Tracksตามเงื่อนไข: วิดีโอ 2, เสียง 2)
INSERT INTO tracks (id, project_id, type, label, order_index) VALUES
('018e3a20-0011-7000-8000-000000000011', '018e3a20-0006-7000-8000-000000000006', 'video', 'Video 2 (Overlays)', 1),
('018e3a20-0012-7000-8000-000000000012', '018e3a20-0006-7000-8000-000000000006', 'video', 'Video 1 (Main A-Roll)', 2),
('018e3a20-0013-7000-8000-000000000013', '018e3a20-0006-7000-8000-000000000006', 'audio', 'Audio 1 (Voiceover)', 1),
('018e3a20-0014-7000-8000-000000000014', '018e3a20-0006-7000-8000-000000000006', 'audio', 'Audio 2 (Background Music)', 2);

-- 7. SEED CLIPS (5 Clips)
INSERT INTO clips (id, project_id, track_id, asset_id, name, track_position_ms, in_point_ms, out_point_ms, duration_ms) VALUES
-- Main A-roll Clips
('018e3a20-0015-7000-8000-000000000015', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0012-7000-8000-000000000012', '018e3a20-0008-7000-8000-000000000008', 'Interview Part 1', 0, 0, 10000, 10000),
('018e3a20-0016-7000-8000-000000000016', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0012-7000-8000-000000000012', '018e3a20-0008-7000-8000-000000000008', 'Interview Part 2', 15000, 20000, 30000, 10000),
-- B-Roll overlay Clip
('018e3a20-0017-7000-8000-000000000017', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0011-7000-8000-000000000011', '018e3a20-0009-7000-8000-000000000009', 'B-Roll Tokyo Traffic', 5000, 0, 5000, 5000),
-- Audio Background Track Clips
('018e3a20-0018-7000-8000-000000000018', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0014-7000-8000-000000000014', '018e3a20-0010-7000-8000-000000000010', 'Lo-Fi Ambient Music', 0, 0, 30000, 30000),
('018e3a20-0019-7000-8000-000000000019', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0013-7000-8000-000000000013', '018e3a20-0008-7000-8000-000000000008', 'Voice Wave Sync', 0, 0, 10000, 10000);

-- 8. SEED EFFECTS
INSERT INTO clip_effects (id, clip_id, type, order_index, params) VALUES
('018e3a20-0020-7000-8000-000000000020', '018e3a20-0017-7000-8000-000000000017', 'brightness', 1, '{"value": 1.1}'),
('018e3a20-0021-7000-8000-000000000021', '018e3a20-0017-7000-8000-000000000017', 'contrast', 2, '{"value": 1.2}');

-- 9. SEED EXPORT JOB
INSERT INTO export_jobs (id, project_id, requested_by, format, resolution, quality, status, idempotency_key) VALUES
('018e3a20-0022-7000-8000-000000000022', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0001-7000-8000-000000000001', 'mp4', '1080p', 'standard', 'completed', 'idem_key_exp_test_001');

-- 10. SEED OPERATION LOGS (จำลองประวัติลำดับที่ 1 และ 2 ของโปรเจกต์นี้)
INSERT INTO operation_logs (id, project_id, user_id, operation_type, payload, client_seq, server_seq) VALUES
('018e3a20-0023-7000-8000-000000000023', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0001-7000-8000-000000000001', 'clip.add', '{"clipId": "018e3a20-0015-7000-8000-000000000015"}', 1, 1),
('018e3a20-0024-7000-8000-000000000024', '018e3a20-0006-7000-8000-000000000006', '018e3a20-0001-7000-8000-000000000001', 'clip.move', '{"clipId": "018e3a20-0017-7000-8000-000000000017", "trackPositionMs": 5000}', 2, 2);