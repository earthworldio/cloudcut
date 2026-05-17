import { create } from 'zustand';

interface UIState {
  selectedClipIds: string[];
  zoomLevel: number; /* pixels per second (ms * (zoomLevel / 1000)) */
  snapEnabled: boolean;
  
  selectClip: (id: string, additive?: boolean) => void;
  deselectAll: () => void;
  setZoom: (level: number) => void;
  toggleSnap: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  selectedClipIds: [],
  zoomLevel: 100, /* 1 second = 100px */
  snapEnabled: true,

  selectClip: (id, additive) => set((state) => ({
    selectedClipIds: additive 
      ? (state.selectedClipIds.includes(id) 
          ? state.selectedClipIds.filter(i => i !== id) 
          : [...state.selectedClipIds, id])
      : [id]
  })),

  deselectAll: () => set({ selectedClipIds: [] }),
  
  setZoom: (level) => set({ zoomLevel: Math.max(10, Math.min(1000, level)) }),
  
  toggleSnap: () => set((state) => ({ snapEnabled: !state.snapEnabled })),
}));
