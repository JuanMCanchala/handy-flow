import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { commands, events } from "@/bindings";

/** Extensions accepted by the import dialog and drop zone (kept in sync with
 * `file_import::SUPPORTED_EXTENSIONS` on the backend). */
export const SUPPORTED_IMPORT_EXTENSIONS = [
  "wav",
  "mp3",
  "m4a",
  "flac",
  "ogg",
  "mp4",
  "mov",
  "webm",
  "mkv",
];

const isSupportedFile = (fileName: string): boolean => {
  const ext = fileName.split(".").pop()?.toLowerCase();
  return !!ext && SUPPORTED_IMPORT_EXTENSIONS.includes(ext);
};

export interface FileImportProgress {
  currentChunk: number;
  totalChunks: number;
}

/**
 * Drives the "import audio/video file" flow: native file picker or drop
 * zone, chunked progress reporting, and cancellation. Saving the result as a
 * history entry happens on the backend; the caller only needs to know when
 * an import starts/finishes so it can show progress and errors.
 */
export const useFileImport = () => {
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState<FileImportProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  const startProgressListener = useCallback(async () => {
    const unlisten = await events.fileImportProgressEvent.listen((event) => {
      setProgress({
        currentChunk: event.payload.current_chunk,
        totalChunks: event.payload.total_chunks,
      });
    });
    unlistenRef.current = unlisten;
  }, []);

  const stopProgressListener = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  }, []);

  const importFile = useCallback(
    async (path: string) => {
      setImporting(true);
      setProgress(null);
      setError(null);
      await startProgressListener();

      try {
        const result = await commands.importAudioFile(path);
        if (result.status !== "ok") {
          setError(String(result.error));
        }
      } catch (e) {
        setError(String(e));
      } finally {
        stopProgressListener();
        setImporting(false);
        setProgress(null);
      }
    },
    [startProgressListener, stopProgressListener],
  );

  const pickAndImportFile = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Audio/Video",
          extensions: SUPPORTED_IMPORT_EXTENSIONS,
        },
      ],
    });
    if (typeof selected === "string") {
      await importFile(selected);
    }
  }, [importFile]);

  const importDroppedFile = useCallback(
    async (fileName: string, path: string) => {
      if (!isSupportedFile(fileName)) {
        setError(fileName);
        return;
      }
      await importFile(path);
    },
    [importFile],
  );

  // Native drag-and-drop gives real filesystem paths (unlike the browser
  // File API, which Tauri does not populate with a usable path).
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          setIsDragOver(true);
        } else if (event.payload.type === "drop") {
          setIsDragOver(false);
          const droppedPath = event.payload.paths[0];
          if (droppedPath) {
            const fileName = droppedPath.split(/[/\\]/).pop() ?? droppedPath;
            void importDroppedFile(fileName, droppedPath);
          }
        } else {
          setIsDragOver(false);
        }
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => unlisten?.();
  }, [importDroppedFile]);

  const cancelImport = useCallback(async () => {
    await commands.cancelAudioImport();
  }, []);

  return {
    importing,
    progress,
    error,
    isDragOver,
    clearError: () => setError(null),
    pickAndImportFile,
    cancelImport,
  };
};
