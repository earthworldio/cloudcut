import React, { useRef, useState, useEffect } from "react";
import {
  Layers,
  Plus,
  Search,
  Video,
  Music,
  Image as ImageIcon,
  File,
  Loader2,
  AlertCircle,
} from "lucide-react";
import api from "../api/axios";
import { useProjectStore } from "../stores/projectStore";
import type { Asset, PresignedUrlResponse } from "../types";
import { toast } from "../lib/swal";
import axios from "axios";

export const AssetPool: React.FC = () => {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  const { currentProject, assets, setAssets, addAsset } = useProjectStore();

  useEffect(() => {
    if (currentProject) {
      fetchAssets();
    }
  }, [currentProject?.id]);

  const fetchAssets = async () => {
    try {
      const response = await api.get<Asset[]>(
        `/assets?projectId=${currentProject?.id}`,
      );
      setAssets(response.data);
    } catch (err) {
      console.error("Failed to fetch assets", err);
    }
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !currentProject) return;

    setIsUploading(true);
    try {
      /* Get Presigned URL */
      const { data: presigned } = await api.post<PresignedUrlResponse>(
        "/assets/presigned-url",
        {
          filename: file.name,
          contentType: file.type,
          sizeBytes: file.size,
          projectId: currentProject.id,
        },
      );

      /* Upload directly to S3 (MinIO) 
        Note: We use a clean axios instance here because we don't want the default auth headers
        or baseURL from our api utility when talking to S3/MinIO directly.
      */
      await axios.put(presigned.uploadUrl, file, {
        headers: { "Content-Type": file.type },
      });

      /* Confirm Upload to Backend */
      const { data: asset } = await api.post<Asset>("/assets/confirm-upload", {
        assetId: presigned.assetId,
        objectKey: presigned.objectKey,
        filename: file.name,
        contentType: file.type,
        sizeBytes: file.size,
        projectId: currentProject.id,
      });

      addAsset(asset);
      toast.fire({
        icon: "success",
        title: "Upload complete",
        text: "Video is being processed.",
      });
    } catch (err) {
      console.error("Upload failed", err);
      toast.fire({
        icon: "error",
        title: "Upload failed",
        text: "Please try again later.",
      });
    } finally {
      setIsUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  const filteredAssets = assets.filter((asset) =>
    asset.metadata?.filename?.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  return (
    <div className="flex flex-col h-full bg-card">
      <div className="p-3 border-b border-border flex items-center justify-between">
        <h2 className="text-sm font-bold uppercase tracking-wider text-muted-foreground flex items-center gap-2">
          <Layers className="w-4 h-4" /> Assets
        </h2>
        <button
          onClick={() => fileInputRef.current?.click()}
          disabled={isUploading}
          className="p-1 hover:bg-muted rounded disabled:opacity-50"
          title="Upload Asset"
        >
          {isUploading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Plus className="w-4 h-4" />
          )}
        </button>
        <input
          type="file"
          ref={fileInputRef}
          className="hidden"
          onChange={handleFileSelect}
          accept="video/*,audio/*,image/*"
        />
      </div>

      <div className="p-2">
        <div className="relative">
          <Search className="w-4 h-4 absolute left-2 top-2 text-muted-foreground" />
          <input
            placeholder="Search assets..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-input border border-border rounded pl-8 pr-2 py-1 text-sm outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {filteredAssets.length === 0 ? (
          <div className="h-40 border-2 border-dashed border-border rounded-lg flex flex-col items-center justify-center p-4 text-center">
            <p className="text-sm text-muted-foreground">No assets found.</p>
            <button
              onClick={() => fileInputRef.current?.click()}
              className="mt-2 text-xs text-primary hover:underline font-medium"
            >
              Upload media
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2">
            {filteredAssets.map((asset) => (
              <div
                key={asset.id}
                className="group relative aspect-video bg-muted rounded border border-border overflow-hidden cursor-move hover:border-primary transition-colors"
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("asset", JSON.stringify(asset));
                }}
              >
                {/* Preview Content */}
                <div className="absolute inset-0 flex items-center justify-center bg-black/20 group-hover:bg-black/40 transition-colors">
                  {asset.type === "video" && asset.url ? (
                    <video
                      src={asset.url}
                      className="w-full h-full object-cover"
                      preload="metadata"
                      onMouseOver={(e) => e.currentTarget.play()}
                      onMouseOut={(e) => {
                        e.currentTarget.pause();
                        e.currentTarget.currentTime = 0;
                      }}
                      muted
                    />
                  ) : asset.type === "image" && asset.url ? (
                    <img
                      src={asset.url}
                      className="w-full h-full object-cover"
                      alt=""
                    />
                  ) : (
                    <div className="flex flex-col items-center gap-1">
                      {asset.type === "video" && (
                        <Video className="w-8 h-8 text-white/70" />
                      )}
                      {asset.type === "audio" && (
                        <Music className="w-8 h-8 text-white/70" />
                      )}
                      {asset.type === "image" && (
                        <ImageIcon className="w-8 h-8 text-white/70" />
                      )}
                      {asset.type === "other" && (
                        <File className="w-8 h-8 text-white/70" />
                      )}
                    </div>
                  )}
                </div>

                {/* Status Badge */}
                {asset.status !== "ready" && (
                  <div className="absolute top-1 right-1 bg-black/60 backdrop-blur-sm px-1.5 py-0.5 rounded text-[10px] flex items-center gap-1 text-white">
                    {asset.status === "processing" && (
                      <Loader2 className="w-2.5 h-2.5 animate-spin" />
                    )}
                    {asset.status === "failed" && (
                      <AlertCircle className="w-2.5 h-2.5 text-red-500" />
                    )}
                    <span className="capitalize">{asset.status}</span>
                  </div>
                )}

                <div className="absolute bottom-0 inset-x-0 bg-gradient-to-t from-black/80 to-transparent p-1.5">
                  <p className="text-[10px] text-white truncate font-medium">
                    {asset.metadata?.filename || "Untitled"}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
