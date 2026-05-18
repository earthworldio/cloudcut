import React from "react";
import { useProjectStore } from "../stores/projectStore";
import { usePlaybackStore } from "../stores/playbackStore";
import { useUIStore } from "../stores/uiStore";
import {
  ZoomIn,
  ZoomOut,
  Scissors,
  Trash2,
  Move,
  X as CloseIcon,
} from "lucide-react";
import { toast } from "../lib/swal";
import type { Clip, Track } from "../types";
import { TimelineRuler } from "./timeline/TimelineRuler";
import { Playhead } from "./timeline/Playhead";

export const Timeline: React.FC = () => {
  const { tracks, addClip, moveClip, splitAllClipsAt, deleteClips } =
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
    splitAllClipsAt(currentTimeMs);
  };

  const handleDelete = async () => {
    if (selectedClipIds.length > 0) {
      await deleteClips(selectedClipIds);
      useUIStore.getState().deselectAll();
    }
  };

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    /* Deselect clips when clicking empty timeline area */
    if (e.target === e.currentTarget) {
      useUIStore.getState().deselectAll();
    }

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
    e.stopPropagation();
    e.dataTransfer.dropEffect = "move";
    if (dragOverTrackId !== trackId) {
      setDragOverTrackId(trackId);
    }
  };

  const handleGlobalDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = async (e: React.DragEvent, trackId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOverTrackId(null);

    const rect = e.currentTarget.getBoundingClientRect();
    /* Calculate position relative to the track area (clientX - track_label_width - rect_left) */
    const x = e.clientX - rect.left - 160;
    let trackPositionMs = (x / zoomLevel) * 1000;
    if (trackPositionMs < 0) trackPositionMs = 0;

    console.log(
      "Drop triggered on track:",
      trackId,
      "at x:",
      x,
      "ms:",
      trackPositionMs,
    );

    try {
      const rawData = e.dataTransfer.getData("application/json");
      if (!rawData) return;

      const data = JSON.parse(rawData);
      const track = tracks.find((t: any) => t.id === trackId);
      if (!track) return;

      if (data.assetId) {
        /* Dropping new asset */
        const asset = useProjectStore
          .getState()
          .assets.find((a: any) => a.id === data.assetId);

        if (!asset) {
          console.error("Asset not found in store:", data.assetId);
          return;
        }

        console.log(
          "Dropping asset:",
          asset.type,
          "on track:",
          track.type,
          "Status:",
          asset.status,
        );

        /* Validation: Check asset type vs track type (case-insensitive and trimmed) */
        const aType = (data.type || asset.type)?.toLowerCase().trim();
        const tType = track.type?.toLowerCase().trim();

        if (!aType || !tType) {
          console.error("Missing type info:", { aType, tType });
          return;
        }

        if (tType === "video" && aType !== "video") {
          toast.fire({
            icon: "warning",
            title: "Invalid track",
            text: `Please drop video assets on a video track. (Asset is ${aType})`,
          });
          return;
        }

        if (tType === "audio" && aType !== "audio") {
          toast.fire({
            icon: "warning",
            title: "Invalid track",
            text: `Please drop audio assets on an audio track. (Asset is ${aType})`,
          });
          return;
        }

        /* Logic: Snap to end of existing clips if dropped near/after */
        const clips = track.clips || [];
        if (clips.length === 0) {
          trackPositionMs = 0;
        } else {
          const sortedClips = [...clips].sort(
            (a, b) =>
              a.track_position_ms +
              a.duration_ms -
              (b.track_position_ms + b.duration_ms),
          );
          const lastClip = sortedClips[sortedClips.length - 1];
          const lastClipEnd = lastClip.track_position_ms + lastClip.duration_ms;

          if (trackPositionMs > lastClipEnd - 100) {
            trackPositionMs = lastClipEnd;
          }
        }

        if (aType === "video") {
          /* 1. Place Video Clip on current track */
          await addClip(
            data.assetId,
            trackId,
            trackPositionMs,
            data.durationMs,
          );

          /* 2. Find the first Audio track and place Audio Clip there at the same position */
          const audioTrack = tracks.find(
            (t: any) => t.type.toLowerCase().trim() === "audio",
          );
          if (audioTrack) {
            await addClip(
              data.assetId,
              audioTrack.id,
              trackPositionMs,
              data.durationMs,
            );
          }
        } else {
          /* Regular drop for audio/image */
          await addClip(
            data.assetId,
            trackId,
            trackPositionMs,
            data.durationMs,
          );
        }

        /* Force reload project after adding clips to ensure UI updates */
        const projectId = useProjectStore.getState().currentProject?.id;
        if (projectId) {
          await useProjectStore.getState().loadProject(projectId);
        }
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
        onDragOver={handleGlobalDragOver}
        onDragLeave={() => setDragOverTrackId(null)}
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
          <div className="pb-20 relative flex flex-col min-h-[300px]">
            {/* Playhead - Wrapped in a relative container that matches tracks width */}
            <div className="absolute inset-0 pointer-events-none z-40">
              <div className="ml-40 relative h-full">
                <Playhead />
              </div>
            </div>

            {tracks.map((track: any) => (
              <div
                key={track.id}
                onDragOver={(e) => handleDragOver(e, track.id)}
                onDrop={(e) => handleDrop(e, track.id)}
                className={`h-[140px] border-b border-zinc-900/50 relative flex group transition-colors ${
                  dragOverTrackId === track.id
                    ? "bg-white/40"
                    : "hover:bg-zinc-900/30"
                }`}
              >
                {/* Track Label Side */}
                <div className="sticky left-0 w-40 h-full bg-zinc-900/90 border-r border-zinc-800 z-20 flex flex-col justify-center px-3 shadow-xl pointer-events-none">
                  <span className="text-xs font-bold truncate text-zinc-100">
                    {track.label}
                  </span>
                  <span className="text-[10px] text-zinc-500 font-bold uppercase tracking-wider">
                    {track.type}
                  </span>
                </div>

                {/* Clips Container */}
                <div className="flex-1 h-full relative group/track">
                  {/* Drop Indicator Visual */}
                  <div
                    className={`absolute inset-0 pointer-events-none transition-colors ${
                      dragOverTrackId === track.id
                        ? "bg-white/40"
                        : "bg-primary/0 group-hover/track:bg-primary/5"
                    }`}
                  />

                  {/* Render Clips */}
                  {(track.clips || []).map((clip: Clip) => (
                    <TimelineClip
                      key={clip.id}
                      clip={clip}
                      track={track}
                      zoomLevel={zoomLevel}
                      isSelected={selectedClipIds.includes(clip.id)}
                      onSelect={(additive) => selectClip(clip.id, additive)}
                      onDelete={() => deleteClips([clip.id])}
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
  track: Track;
  zoomLevel: number;
  isSelected: boolean;
  onSelect: (additive: boolean) => void;
  onDelete: () => void;
}

const WaveformCanvas: React.FC<{
  waveform: any;
  width: number;
  height: number;
}> = ({ waveform, width, height }) => {
  const canvasRef = React.useRef<HTMLCanvasElement>(null);

  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    /* Clear and setup DPI */
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    const peaks = waveform.peaks; /* [[min, max], ...] */
    if (!peaks || peaks.length === 0) return;

    ctx.fillStyle = "#3b82f6"; /* Blue waveform */
    const barWidth = width / peaks.length;
    const centerY = height / 2;

    peaks.forEach((peak: number[], i: number) => {
      const x = i * barWidth;
      const min = peak[0] * centerY;
      const max = peak[1] * centerY;

      /* Draw vertical bar for this peak */
      ctx.fillRect(x, centerY + min, Math.max(1, barWidth - 0.5), max - min);
    });
  }, [waveform, width, height]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: `${width}px`, height: `${height}px` }}
    />
  );
};

const TimelineClip: React.FC<TimelineClipProps> = ({
  clip,
  track,
  zoomLevel,
  isSelected,
  onSelect,
  onDelete,
}) => {
  const widthPx = (clip.duration_ms / 1000) * zoomLevel;
  const leftPx = (clip.track_position_ms / 1000) * zoomLevel;
  const seek = usePlaybackStore((state: any) => state.seek);
  const assets = useProjectStore((state: any) => state.assets);
  const asset = assets.find((a: any) => a.id === clip.asset_id);

  const isVideo = track.type.toLowerCase() === "video";

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
        absolute top-2 bottom-2 rounded border transition-all cursor-pointer overflow-hidden group/clip
        ${
          isSelected
            ? "border-white ring-2 ring-white/30 z-20 scale-[1.01]"
            : "border-zinc-700 hover:border-zinc-500 z-10"
        }
        ${
          isVideo
            ? isSelected
              ? "bg-blue-600 shadow-lg shadow-blue-500/40"
              : "bg-blue-900/90"
            : isSelected
              ? "bg-emerald-600 shadow-lg shadow-emerald-500/40"
              : "bg-emerald-900/90"
        }
      `}
      style={{ left: `${leftPx}px`, width: `${widthPx}px` }}
    >
      {/* Video Thumbnail Preview (if available) */}
      {isVideo && asset?.url && (
        <div className="absolute inset-0 opacity-60 pointer-events-none">
          <video
            src={asset.url}
            className="w-full h-full object-cover"
            preload="metadata"
            onLoadedMetadata={(e) => {
              /* Seek to a representative frame (e.g., 1s or 10% of duration) */
              e.currentTarget.currentTime = Math.min(
                1,
                e.currentTarget.duration * 0.1,
              );
            }}
          />
          {/* Overlay to make text more readable */}
          <div className="absolute inset-0 bg-gradient-to-b from-black/40 to-transparent" />
        </div>
      )}

      {/* Waveform Background for Audio */}
      {!isVideo && clip.waveform && (
        <div className="absolute inset-0 opacity-70 pointer-events-none flex items-center">
          <WaveformCanvas
            waveform={clip.waveform}
            width={widthPx}
            height={120}
          />
        </div>
      )}

      {/* Clip Label */}
      <div className="relative z-10 px-2 py-1.5 flex flex-col justify-between h-full pointer-events-none">
        <div className="flex items-start justify-between">
          <span className="text-[10px] font-bold truncate text-white drop-shadow-md bg-black/60 px-1.5 py-0.5 rounded border border-white/10 max-w-[80%]">
            {clip.name}
          </span>

          {/* Delete Button (X) - Only visible when selected or hovered */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            className={`
              p-1 rounded bg-red-500 text-white hover:bg-red-600 transition-all pointer-events-auto shadow-lg border border-red-400/50
              ${isSelected ? "opacity-100 scale-100" : "opacity-0 scale-90 group-hover/clip:opacity-100 group-hover/clip:scale-100"}
            `}
            title="Remove Clip"
          >
            <CloseIcon className="w-3 h-3" />
          </button>
        </div>

        {/* Clip Duration Text at Bottom */}
        <div className="flex justify-end">
          <span className="text-[9px] font-medium text-white/70 bg-black/40 px-1 rounded">
            {(clip.duration_ms / 1000).toFixed(1)}s
          </span>
        </div>
      </div>

      {/* Trim Handles */}
      <div className="absolute left-0 top-0 bottom-0 w-2 hover:bg-white/40 cursor-ew-resize z-20 pointer-events-auto" />
      <div className="absolute right-0 top-0 bottom-0 w-2 hover:bg-white/40 cursor-ew-resize z-20 pointer-events-auto" />
    </div>
  );
};
