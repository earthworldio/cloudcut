import React, { useState } from "react";
import { X, Download, Loader2, AlertCircle, CheckCircle2 } from "lucide-react";
import api from "../api/axios";
import { toast } from "../lib/swal";

interface ExportModalProps {
  projectId: string;
  onClose: () => void;
}

interface ExportJob {
  id: string;
  status: "queued" | "processing" | "completed" | "failed" | "cancelled";
  progress_percent: number;
  output_url?: string;
  error_message?: string;
}

export const ExportModal: React.FC<ExportModalProps> = ({
  projectId,
  onClose,
}) => {
  const [isStarting, setIsStarting] = useState(false);
  const [job, setJob] = useState<ExportJob | null>(null);

  const startExport = async () => {
    setIsStarting(true);
    try {
      const { data: jobResponse } = await api.post<{ exportId: string; status: string }>(
        `/projects/${projectId}/exports`,
        {
          format: "mp4",
          resolution: "1080p",
          quality: "standard",
        },
      );
      pollStatus(jobResponse.exportId);
    } catch (err: any) {
      toast.fire({
        icon: "error",
        title: "Export Failed",
        text: err.response?.data?.message || "Could not start export",
      });
      setIsStarting(false);
    }
  };

  const pollStatus = async (exportId: string) => {
    try {
      /* In this simplified implementation, we find the export job in the project data 
         if the backend provides it. Or we can create a specific endpoint.
         For now, let's assume we have an endpoint GET /api/projects/:id/exports/:exportId
      */
      const jobRes = await api.get<ExportJob>(
        `/projects/${projectId}/exports/${exportId}`,
      );
      setJob(jobRes.data);
      setIsStarting(false);

      if (
        jobRes.data.status === "queued" ||
        jobRes.data.status === "processing"
      ) {
        setTimeout(() => pollStatus(exportId), 2000);
      }
    } catch (err) {
      console.error("Polling failed", err);
    }
  };

  const cancelExport = async () => {
    if (!job) return;
    try {
      await api.post(`/projects/${projectId}/exports/${job.id}/cancel`);
      setJob({ ...job, status: "cancelled" });
      toast.fire({
        icon: "info",
        title: "Export Cancelled",
      });
    } catch (err) {
      toast.fire({
        icon: "error",
        title: "Cancel Failed",
      });
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div className="bg-zinc-900 border border-zinc-800 w-full max-w-md rounded-xl shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between p-4 border-b border-zinc-800">
          <h3 className="text-lg font-bold">Export Video</h3>
          <button
            onClick={onClose}
            className="p-1 hover:bg-zinc-800 rounded-full transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-8">
          {!job && !isStarting && (
            <div className="text-center space-y-6">
              <div className="w-20 h-20 bg-primary/10 rounded-full flex items-center justify-center mx-auto">
                <Download className="w-10 h-10 text-primary" />
              </div>
              <div>
                <h4 className="font-semibold text-lg">Ready to render?</h4>
                <p className="text-sm text-zinc-400 mt-2">
                  Your project will be rendered into a high-quality MP4 video.
                </p>
              </div>
              <button
                onClick={startExport}
                className="w-full py-3 bg-primary text-primary-foreground font-bold rounded-lg hover:opacity-90 transition-opacity"
              >
                Start Export
              </button>
            </div>
          )}

          {(isStarting ||
            (job &&
              (job.status === "queued" || job.status === "processing"))) && (
            <div className="space-y-6 text-center">
              <div className="relative w-24 h-24 mx-auto">
                <Loader2 className="w-24 h-24 text-primary animate-spin" />
                <div className="absolute inset-0 flex items-center justify-center text-sm font-mono font-bold">
                  {job?.progress_percent || 0}%
                </div>
              </div>
              <div>
                <h4 className="font-semibold">Rendering...</h4>
                <p className="text-xs text-zinc-500 mt-1 uppercase tracking-widest">
                  {job?.status || "Starting"}
                </p>
              </div>
              <div className="w-full bg-zinc-800 h-2 rounded-full overflow-hidden">
                <div
                  className="bg-primary h-full transition-all duration-500"
                  style={{ width: `${job?.progress_percent || 0}%` }}
                />
              </div>
              <button
                onClick={cancelExport}
                className="text-sm text-red-400 hover:text-red-300 transition-colors"
              >
                Cancel Export
              </button>
            </div>
          )}

          {job?.status === "completed" && (
            <div className="text-center space-y-6">
              <div className="w-20 h-20 bg-green-500/10 rounded-full flex items-center justify-center mx-auto">
                <CheckCircle2 className="w-10 h-10 text-green-500" />
              </div>
              <div>
                <h4 className="font-semibold text-lg">Export Complete!</h4>
                <p className="text-sm text-zinc-400 mt-2">
                  Your video is ready to download.
                </p>
              </div>
              <a
                href={job.output_url}
                download={`video_${projectId.slice(0, 8)}.mp4`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center justify-center gap-2 w-full py-3 bg-green-600 text-white font-bold rounded-lg hover:bg-green-500 transition-colors"
              >
                <Download className="w-5 h-5" /> Download Video
              </a>
            </div>
          )}

          {(job?.status === "failed" || job?.status === "cancelled") && (
            <div className="text-center space-y-6">
              <div className="w-20 h-20 bg-red-500/10 rounded-full flex items-center justify-center mx-auto">
                <AlertCircle className="w-10 h-10 text-red-500" />
              </div>
              <div>
                <h4 className="font-semibold text-lg">
                  {job.status === "cancelled"
                    ? "Export Cancelled"
                    : "Export Failed"}
                </h4>
                <p className="text-sm text-zinc-400 mt-2">
                  {job.error_message ||
                    "Something went wrong during the render process."}
                </p>
              </div>
              <button
                onClick={() => {
                  setJob(null);
                  setIsStarting(false);
                }}
                className="w-full py-3 bg-zinc-800 text-white font-bold rounded-lg hover:bg-zinc-700 transition-colors"
              >
                Try Again
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
