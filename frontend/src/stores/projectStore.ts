import { create } from 'zustand';
import type { Project, Track, Clip, TimelineData } from '../types';

interface ProjectState {
  currentProject: Project | null;
  tracks: Array<Track & { clips: Clip[] }>;
  selectedClipId: string | null;
  currentTimeMs: number;
  zoomScale: number; /* pixels per millisecond */
  
  setTimeline: (data: TimelineData) => void;
  selectClip: (clipId: string | null) => void;
  setCurrentTime: (timeMs: number) => void;
  setZoomScale: (scale: number) => void;
  updateClipPositionLocal: (clipId: string, trackPositionMs: number, trackId: string) => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  currentProject: null,
  tracks: [],
  selectedClipId: null,
  currentTimeMs: 0,
  zoomScale: 0.1, // 100ms = 10px by default

  setTimeline: (data) => set({ 
    currentProject: data.project, 
    tracks: data.tracks 
  }),
  
  selectClip: (clipId) => set({ selectedClipId: clipId }),
  
  setCurrentTime: (timeMs) => set({ currentTimeMs: timeMs }),
  
  setZoomScale: (scale) => set({ zoomScale: scale }),

  updateClipPositionLocal: (clipId, trackPositionMs, trackId) => set((state) => ({
    tracks: state.tracks.map(track => ({
      ...track,
      clips: track.id === trackId 
        ? track.clips.map(clip => clip.id === clipId ? { ...clip, track_position_ms: trackPositionMs } : clip)
        : track.clips.filter(clip => clip.id !== clipId)
    }))
  }))
}));
