import React from "react";
import { useProjectStore } from "../stores/projectStore";
import { usePlaybackStore } from "../stores/playbackStore";
import { useUIStore } from "../stores/uiStore";
import { ZoomIn, ZoomOut, Scissors, Trash2, Move } from "lucide-react";
import type { Clip } from "../types";
import { TimelineRuler } from "./timeline/TimelineRuler";
import { Playhead } from "./timeline/Playhead";

export const Timeline: React.FC = () => {
  const { tracks, addClip, moveClip, splitClip, deleteClips } =
    useProjectStore();
  const { zoomLevel, setZoom, selectedClipIds, selectClip } = useUIStore();
  const { currentTimeMs, seek } = usePlaybackStore();

  const [dragOverTrackId, setDragOverTrackId] = React.useState<string | null>(
    null,
  );

  const handleZoom = (delta: number) => {
    setZoom(zoomLevel + delta * 100);
  };

  const handleSplit = () => {
    const selectedId = selectedClipIds[0];

    if (!selectedId) return;

    const allClips = tracks.flatMap((t) => t.clips);
    const clip = allClips.find((c) => c.id === selectedId);

    if (clip) {
      const start = clip.track_position_ms;
      const end = clip.track_position_ms + clip.duration_ms;

      if (currentTimeMs > start && currentTimeMs < end) {
        splitClip(clip.id, currentTimeMs - start);
      }
    }
  };

  const handleDelete = async () => {
    if (selectedClipIds.length > 0) {
      await deleteClips(selectedClipIds);
      useUIStore.getState().deselectAll();
    }
  };

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    /* Calculate time based on click position relative to the scrollable container */
    const rect = e.currentTarget.getBoundingClientRect();
    const x =
      e.clientX -
      rect.left +
      e.currentTarget.scrollLeft -
      160; /* 160 is track label width */
    if (x >= 0) {
      const timeMs = (x / zoomLevel) * 1000;
      seek(timeMs);
    }
  };

  const handleDragOver = (e: React.DragEvent, trackId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    if (dragOverTrackId !== trackId) {
      setDragOverTrackId(trackId);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOverTrackId(null);
  };

  const handleDrop = async (e: React.DragEvent, trackId: string) => {
    e.preventDefault();
    setDragOverTrackId(null);

    const rect = e.currentTarget.getBoundingClientRect();
    let trackPositionMs = ((e.clientX - rect.left) / zoomLevel) * 1000;

    try {
      const rawData = e.dataTransfer.getData("application/json");
      if (!rawData) return;

      const data = JSON.parse(rawData);
      const track = tracks.find((t) => t.id === trackId);

      /* Logic: If dropping into empty space or specifically requested, snap to end of existing clips */
      if (track) {
        if (track.clips.length === 0) {
          /* 1. If track is empty, go to 00:00:00 */
          trackPositionMs = 0;
        } else {
          /* 2. Check if we are dropping far after the last clip */
          const sortedClips = [...track.clips].sort(
            (a, b) =>
              a.track_position_ms +
              a.duration_ms -
              (b.track_position_ms + b.duration_ms),
          );
          const lastClip = sortedClips[sortedClips.length - 1];
          const lastClipEnd = lastClip.track_position_ms + lastClip.duration_ms;

          /* If dropping near or after the end of the last clip, snap to it */
          if (trackPositionMs > lastClipEnd - 100) {
            trackPositionMs = lastClipEnd;
          }
        }
      }

      if (data.assetId) {
        /* Dropping new asset */
        await addClip(data.assetId, trackId, trackPositionMs, data.durationMs);
      } else if (data.clipId) {
        /* Moving existing clip */
        await moveClip(data.clipId, trackPositionMs, trackId);
      }
    } catch (err) {
      console.error("Failed to process drop", err);
    }
  };

  return (
    <div className="flex flex-col h-full select-none">
      {/* Timeline Toolbar */}
      <div className="h-10 border-b border-border flex items-center justify-between px-4 bg-muted/30">
        <div className="flex items-center gap-4">
          <button
            onClick={handleSplit}
            className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-primary"
            title="Split (S)"
          >
            <Scissors className="w-4 h-4" />
          </button>
          <button
            onClick={handleDelete}
            className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-red-500"
            title="Delete (Backspace/Delete)"
          >
            <Trash2 className="w-4 h-4" />
          </button>
          <div className="h-4 w-px bg-border mx-2" />
          <button className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-primary">
            <Move className="w-4 h-4" />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => handleZoom(-0.5)}
            className="p-1 hover:bg-muted rounded"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <input
            type="range"
            min="10"
            max="500"
            step="10"
            value={zoomLevel}
            onChange={(e) => setZoom(parseInt(e.target.value))}
            className="w-24 accent-primary"
          />
          <button
            onClick={() => handleZoom(0.5)}
            className="p-1 hover:bg-muted rounded"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Timeline Grid */}
      <div
        className="flex-1 overflow-auto relative bg-zinc-950/50"
        onClick={handleTimelineClick}
      >
        <div className="min-w-max relative">
          {/* Ruler Area */}
          <div className="sticky top-0 z-30 h-10 flex border-b border-border bg-zinc-900">
            <div className="w-40 h-full border-r border-border bg-zinc-900/90 z-40 sticky left-0" />
            <div className="flex-1">
              <TimelineRuler />
            </div>
          </div>

          {/* Tracks List */}
          <div className="pb-20 relative">
            {/* Playhead - Wrapped in a relative container that matches tracks width */}
            <div className="absolute inset-0 pointer-events-none z-40">
              <div className="ml-40 relative h-full">
                <Playhead />
              </div>
            </div>

            {tracks.map((track) => (
              <div
                key={track.id}
                className={`h-20 border-b border-zinc-900/50 relative flex items-center group transition-colors ${
                  dragOverTrackId === track.id
                    ? "bg-white/10"
                    : "hover:bg-zinc-900/30"
                }`}
              >
                {/* Track Label Side */}
                <div className="sticky left-0 w-40 h-full bg-zinc-900/90 border-r border-zinc-800 z-20 flex flex-col justify-center px-3 shadow-xl">
                  <span className="text-xs font-medium truncate">
                    {track.label}
                  </span>
                  <span className="text-[10px] text-zinc-500 uppercase">
                    {track.type}
                  </span>
                </div>

                {/* Clips Container */}
                <div
                  className="flex-1 h-full relative group/track"
                  onDragOver={(e) => handleDragOver(e, track.id)}
                  onDragLeave={handleDragLeave}
                  onDrop={(e) => handleDrop(e, track.id)}
                >
                  {/* Drop Indicator Visual */}
                  <div
                    className={`absolute inset-0 pointer-events-none transition-colors ${
                      dragOverTrackId === track.id
                        ? "bg-white/20"
                        : "bg-primary/0 group-hover/track:bg-primary/5"
                    }`}
                  />

                  {track.clips.map((clip) => (
                    <TimelineClip
                      key={clip.id}
                      clip={clip}
                      zoomLevel={zoomLevel}
                      isSelected={selectedClipIds.includes(clip.id)}
                      onSelect={(additive) => selectClip(clip.id, additive)}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

interface TimelineClipProps {
  clip: Clip;
  zoomLevel: number;
  isSelected: boolean;
  onSelect: (additive: boolean) => void;
}

const TimelineClip: React.FC<TimelineClipProps> = ({
  clip,
  zoomLevel,
  isSelected,
  onSelect,
}) => {
  const widthPx = (clip.duration_ms / 1000) * zoomLevel;
  const leftPx = (clip.track_position_ms / 1000) * zoomLevel;
  const seek = usePlaybackStore((state) => state.seek);

  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(
          "application/json",
          JSON.stringify({ clipId: clip.id }),
        );
      }}
      onClick={(e) => {
        e.stopPropagation();
        onSelect(e.shiftKey || e.metaKey || e.ctrlKey);
        /* เมื่อคลิกคลิป ให้ Playhead กระโดดไปที่จุดเริ่มต้นของคลิปนั้น */
        seek(clip.track_position_ms);
      }}
      className={`
        absolute top-2 bottom-2 rounded border transition-all cursor-pointer overflow-hidden
        ${
          isSelected
            ? "bg-blue-500/30 border-blue-400 ring-2 ring-blue-500/20 z-10"
            : "bg-zinc-800 border-zinc-700 hover:bg-zinc-700 hover:border-zinc-600"
        }
      `}
      style={{ left: `${leftPx}px`, width: `${widthPx}px` }}
    >
      <div className="px-2 py-1 text-[11px] font-medium truncate">
        {clip.name}
      </div>

      {/* Trim Handles */}
      <div className="absolute left-0 top-0 bottom-0 w-1 hover:bg-white/20 cursor-ew-resize" />
      <div className="absolute right-0 top-0 bottom-0 w-1 hover:bg-white/20 cursor-ew-resize" />
    </div>
  );
};
