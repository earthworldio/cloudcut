export interface User {
  id: string;
  email: string;
  name: string;
  workspace_id?: string;
}

export interface Workspace {
  id: string;
  name: string;
  slug: string;
  plan: 'free' | 'pro' | 'team';
  owner_id: string;
}

export interface Project {
  id: string;
  workspace_id: string;
  name: string;
  description?: string;
  settings: ProjectSettings;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectSettings {
  resolution: '720p' | '1080p' | '4k';
  fps: number;
  aspect_ratio: string;
}

export interface Track {
  id: string;
  project_id: string;
  type: 'video' | 'audio';
  label: string;
  order_index: number;
  is_locked: boolean;
  is_muted: boolean;
  color?: string;
}

export interface Clip {
  id: string;
  project_id: string;
  track_id: string;
  asset_id: string;
  name: string;
  track_position_ms: number;
  in_point_ms: number;
  out_point_ms: number;
  duration_ms: number;
  transform: ClipTransform;
  version: number;
}

export interface ClipTransform {
  x: number;
  y: number;
  scale: number;
  rotation: number;
  opacity: number;
}

export interface TimelineData {
  project: Project;
  tracks: Array<{
    id: string;
    project_id: string;
    type: 'video' | 'audio';
    label: string;
    order_index: number;
    is_locked: boolean;
    is_muted: boolean;
    color?: string;
    clips: Clip[];
  }>;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface Asset {
  id: string;
  project_id: string;
  uploaded_by: string;
  type: 'video' | 'audio' | 'image' | 'other';
  original_url: string;
  url?: string; /* เพิ่มฟิลด์นี้สำหรับแสดงผล Preview */
  status: 'uploading' | 'processing' | 'ready' | 'failed';
  metadata: any;
  created_at: string;
  updated_at: string;
}

export interface PresignedUrlResponse {
  uploadUrl: string;
  assetId: string;
  objectKey: string;
}
