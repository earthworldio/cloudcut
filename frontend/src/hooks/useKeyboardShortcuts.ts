import { useEffect } from 'react';
import { usePlaybackStore } from '../stores/playbackStore';
import { useProjectStore } from '../stores/projectStore';
import { useUIStore } from '../stores/uiStore';

export const useKeyboardShortcuts = () => {
  const { isPlaying, play, pause, currentTimeMs } = usePlaybackStore();
  const { selectedClipIds, deselectAll } = useUIStore();
  const { tracks, splitClip, deleteClips } = useProjectStore();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      /* Ignore if user is typing in an input */
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      const key = e.key.toLowerCase();

      /* Space: Play/Pause */
      if (key === ' ') {
        e.preventDefault();
        if (isPlaying) pause();
        else play();
      }

      /* S: Split selected clip */
      if (key === 's') {
        e.preventDefault();
        const selectedId = selectedClipIds[0];
        if (!selectedId) return;

        /* Find the clip to see if playhead is inside it */
        const allClips = tracks.flatMap(t => t.clips);
        const clip = allClips.find(c => c.id === selectedId);

        if (clip) {
          const start = clip.track_position_ms;
          const end = clip.track_position_ms + clip.duration_ms;

          if (currentTimeMs > start && currentTimeMs < end) {
            splitClip(clip.id, currentTimeMs);
          }
        }
      }

      /* Backspace/Delete: Delete selected clips */
      if (key === 'backspace' || key === 'delete') {
        if (selectedClipIds.length > 0) {
          e.preventDefault();
          deleteClips(selectedClipIds);
          deselectAll();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isPlaying, play, pause, currentTimeMs, selectedClipIds, tracks, splitClip]);
};
