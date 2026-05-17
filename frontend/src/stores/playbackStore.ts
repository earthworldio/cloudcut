import { create } from 'zustand';

interface PlaybackState {
  currentTimeMs: number;
  isPlaying: boolean;
  volume: number;
  isMuted: boolean;
  
  play: () => void;
  pause: () => void;
  seek: (timeMs: number) => void;
  setVolume: (vol: number) => void;
  toggleMute: () => void;
}

export const usePlaybackStore = create<PlaybackState>((set) => ({
  currentTimeMs: 0,
  isPlaying: false,
  volume: 1.0,
  isMuted: false,

  play: () => set({ isPlaying: true }),
  pause: () => set({ isPlaying: false }),
  seek: (timeMs) => set({ currentTimeMs: Math.max(0, timeMs) }),
  setVolume: (vol) => set({ volume: Math.min(1, Math.max(0, vol)) }),
  toggleMute: () => set((state) => ({ isMuted: !state.isMuted })),
}));
