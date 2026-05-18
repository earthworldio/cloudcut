import { create } from 'zustand';
import type { Project, Track, Clip, TimelineData, Asset } from '../types';
import api from '../api/axios';
import { useUIStore } from './uiStore';

interface ProjectState {
  currentProject: Project | null;
  tracks: Array<Track & { clips: Clip[] }>;
  assets: Asset[];
  
  loadProject: (projectId: string) => Promise<void>;
  setTimeline: (data: TimelineData) => void;
  setAssets: (assets: Asset[]) => void;
  addAsset: (asset: Asset) => void;
  updateAssetStatus: (assetId: string, status: Asset['status']) => void;
  
  addClip: (assetId: string, trackId: string, positionMs: number, assetDurationMs: number) => Promise<void>;
  moveClip: (clipId: string, positionMs: number, trackId: string) => Promise<void>;
  trimClip: (clipId: string, inPointMs: number, outPointMs: number) => Promise<void>;
  splitClip: (clipId: string, atTimelineTimeMs: number) => Promise<void>;
  splitAllClipsAt: (atTimelineTimeMs: number) => Promise<void>;
  deleteClips: (clipIds: string[]) => Promise<void>;
  
  updateProjectNameLocal: (name: string) => void;
  deleteProject: (projectId: string) => Promise<void>;
  deleteWorkspace: (workspaceId: string) => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  currentProject: null,
  tracks: [],
  assets: [],

  loadProject: async (projectId) => {
    try {
      const response = await api.get<TimelineData>(`/projects/${projectId}/timeline`);
      set({ 
        currentProject: response.data.project, 
        tracks: response.data.tracks 
      });
    } catch (err) {
      console.error("Failed to load project", err);
    }
  },

  setTimeline: (data) => set({ 
    currentProject: data.project, 
    tracks: data.tracks 
  }),

  setAssets: (assets) => set({ assets }),
  
  addAsset: (asset) => set((state) => ({ 
    assets: [asset, ...state.assets] 
  })),

  updateAssetStatus: (assetId: string, status: Asset['status']) => set((state) => ({
    assets: state.assets.map(a => a.id === assetId ? { ...a, status } : a)
  })),

  addClip: async (assetId, trackId, positionMs, assetDurationMs) => {
    const projectId = get().currentProject?.id;
    if (!projectId) return;

    /* Use full duration if available, otherwise default to 5000ms */
    const durationMs = assetDurationMs || 5000;
    const asset = get().assets.find((a) => a.id === assetId);

    try {
      await api.post(`/projects/${projectId}/clips`, {
        asset_id: assetId,
        track_id: trackId,
        name: asset?.metadata?.filename || "New Clip",
        track_position_ms: Math.max(0, positionMs),
        in_point_ms: 0,
        out_point_ms: durationMs,
        duration_ms: durationMs,
      });
      get().loadProject(projectId);
    } catch (err) {
      console.error("Failed to add clip", err);
    }
  },

  moveClip: async (clipId, positionMs, trackId) => {
    const projectId = get().currentProject?.id;
    if (!projectId) return;

    /* Optimistic update */
    const oldTracks = get().tracks;
    const movedClip = oldTracks.flatMap(t => t.clips).find(c => c.id === clipId);
    
    if (!movedClip) return;

    set((state) => ({
      tracks: state.tracks.map(track => {
        /* Remove the clip from its current track (if it's not the target track) */
        let newClips = track.clips.filter(c => c.id !== clipId);
        
        /* If this is the target track, add the clip at its new position */
        if (track.id === trackId) {
          const updatedClip = { 
            ...movedClip, 
            track_position_ms: Math.max(0, positionMs), 
            track_id: trackId 
          };
          newClips.push(updatedClip);
        }
        
        return { ...track, clips: newClips };
      })
    }));

    try {
      await api.patch(`/projects/${projectId}/clips/${clipId}`, {
        track_id: trackId,
        track_position_ms: Math.max(0, positionMs)
      });
    } catch (err) {
      console.error("Failed to move clip", err);
      set({ tracks: oldTracks });
    }
  },

  trimClip: async (clipId, inPointMs, outPointMs) => {
    const projectId = get().currentProject?.id;
    if (!projectId) return;
    const durationMs = outPointMs - inPointMs;
    try {
      await api.patch(`/projects/${projectId}/clips/${clipId}`, {
        in_point_ms: inPointMs,
        out_point_ms: outPointMs,
        duration_ms: durationMs
      });
      get().loadProject(projectId);
    } catch (err) {
      console.error("Failed to trim clip", err);
    }
  },

  splitClip: async (clipId, atTimelineTimeMs) => {
    const projectId = get().currentProject?.id;
    if (!projectId) return;
    try {
      const response = await api.post<{ part1: Clip, part2: Clip }>(
        `/projects/${projectId}/clips/${clipId}/split`, 
        { split_time_ms: Math.floor(atTimelineTimeMs) }
      );
      
      /* Reload project to get updated state from server */
      await get().loadProject(projectId);
      
      /* UX: Select the newly created second part */
      const { part2 } = response.data;
      if (part2) {
        useUIStore.getState().selectClip(part2.id, false);
      }
    } catch (err) {
      console.error("Failed to split clip", err);
    }
  },

  splitAllClipsAt: async (atTimelineTimeMs) => {
    const projectId = get().currentProject?.id;
    if (!projectId) return;

    const tracks = get().tracks;
    const clipsToSplit = tracks.flatMap((t) => t.clips || []).filter((clip) => {
      const start = clip.track_position_ms;
      const end = clip.track_position_ms + clip.duration_ms;
      /* Buffer of 1ms to avoid precision issues */
      return atTimelineTimeMs > start + 1 && atTimelineTimeMs < end - 1;
    });

    console.log("Global Split triggered at:", atTimelineTimeMs, "Found clips:", clipsToSplit.length);

    if (clipsToSplit.length === 0) return;

    try {
      /* Split all clips sequentially to avoid DB lock/race conditions in backend */
      for (const clip of clipsToSplit) {
        await api.post(`/projects/${projectId}/clips/${clip.id}/split`, {
          split_time_ms: Math.floor(atTimelineTimeMs),
        });
      }

      /* Reload project to get final state */
      await get().loadProject(projectId);
    } catch (err) {
      console.error("Failed to split all clips", err);
    }
  },

  deleteClips: async (clipIds) => {
    const projectId = get().currentProject?.id;
    if (!projectId || clipIds.length === 0) return;

    /* Optimistic update: Remove from UI immediately */
    const oldTracks = get().tracks;
    set((state) => ({
      tracks: state.tracks.map(track => ({
        ...track,
        clips: track.clips.filter(c => !clipIds.includes(c.id))
      }))
    }));

    try {
      await Promise.all(
        clipIds.map((id) => api.delete(`/projects/${projectId}/clips/${id}`))
      );
      /* Clear selection */
      useUIStore.getState().deselectAll();
      /* Final sync with server */
      get().loadProject(projectId);
    } catch (err) {
      console.error("Failed to delete clips", err);
      /* Rollback on error */
      set({ tracks: oldTracks });
    }
  },

  updateProjectNameLocal: (name: string) => set((state) => ({
    currentProject: state.currentProject ? { ...state.currentProject, name } : null
  })),

  deleteProject: async (projectId) => {
    try {
      await api.delete(`/projects/${projectId}`);
      /* If the deleted project is the current one, clear it */
      if (get().currentProject?.id === projectId) {
        set({ currentProject: null, tracks: [] });
      }
    } catch (err) {
      console.error("Failed to delete project", err);
      throw err;
    }
  },

  deleteWorkspace: async (workspaceId) => {
    try {
      await api.delete(`/workspaces/${workspaceId}`);
      /* Clear state since workspace is gone */
      set({ currentProject: null, tracks: [], assets: [] });
    } catch (err) {
      console.error("Failed to delete workspace", err);
      throw err;
    }
  }
}));
