import {
  uploadChunkWithProgress,
  getCompletedChunks,
  completeTransfer,
  cancelTransferApi,
} from "./api";

const DEFAULT_CHUNK_SIZE = 512 * 1024;
const PROGRESS_INTERVAL_MS = 100;

export interface UploadProgress {
  fileName: string;
  transferredBytes: number;
  totalBytes: number;
  progress: number;
  speedBytesPerSecond: number;
  remainingSeconds: number | null;
  elapsedSeconds: number;
  status: "uploading" | "verifying" | "completed" | "cancelled" | "failed";
  error?: string;
}

/**
 * One uploader represents one logical transfer, even when it contains several
 * files. Keeping telemetry at that scope prevents the progress bar, speed and
 * remaining time from resetting at every file boundary.
 */
export class FileUploader {
  private cancelled = false;
  private readonly sessionToken: string;
  private readonly onProgress: (p: UploadProgress) => void;
  private transferTotalBytes = 0;
  private transferredBeforeCurrentFile = 0;
  private transferStartedAt = 0;
  private samples: Array<{ at: number; bytes: number }> = [];
  private lastProgressAt = 0;

  constructor(sessionToken: string, onProgress: (p: UploadProgress) => void) {
    this.sessionToken = sessionToken;
    this.onProgress = onProgress;
  }

  beginTransfer(totalBytes: number): void {
    this.transferTotalBytes = Math.max(totalBytes, 0);
    this.transferredBeforeCurrentFile = 0;
    this.transferStartedAt = Date.now();
    this.samples = [];
    this.lastProgressAt = 0;
  }

  async uploadFile(
    transferId: string,
    file: File,
    fileId?: string,
    chunkSize = DEFAULT_CHUNK_SIZE
  ): Promise<void> {
    if (this.cancelled) throw new Error("cancelled");
    if (!this.transferStartedAt) this.beginTransfer(file.size);

    const safeChunkSize = Math.max(1, chunkSize);
    const totalChunks = Math.ceil(file.size / safeChunkSize);
    let completedChunks: number[] = [];
    try {
      completedChunks = await getCompletedChunks(transferId, this.sessionToken, fileId);
    } catch {
      // Resume metadata is an optimization. A new transfer can still start.
    }

    const completed = new Set(completedChunks);
    let fileTransferredBytes = completedChunks.reduce((total, index) => {
      const start = index * safeChunkSize;
      return total + Math.max(0, Math.min(safeChunkSize, file.size - start));
    }, 0);

    this.emitProgress(file.name, fileTransferredBytes, file.size, true);

    for (let index = 0; index < totalChunks; index += 1) {
      if (this.cancelled) throw new Error("cancelled");
      if (completed.has(index)) continue;

      const start = index * safeChunkSize;
      const end = Math.min(start + safeChunkSize, file.size);
      const buffer = await file.slice(start, end).arrayBuffer();
      const completedBeforeChunk = fileTransferredBytes;

      await this.uploadChunkWithRetry(
        transferId,
        index,
        buffer,
        fileId,
        (loaded, total) => {
          const chunkBytes = Math.max(0, Math.min(loaded, total));
          this.emitProgress(
            file.name,
            completedBeforeChunk + chunkBytes,
            file.size
          );
        }
      );

      fileTransferredBytes = completedBeforeChunk + (end - start);
      this.emitProgress(file.name, fileTransferredBytes, file.size, true);
    }

    this.transferredBeforeCurrentFile += file.size;
    this.emitProgress(file.name, file.size, file.size, true);
  }

  async complete(transferId: string): Promise<void> {
    if (this.cancelled) throw new Error("cancelled");
    await completeTransfer(transferId, this.sessionToken);
  }

  private emitProgress(
    fileName: string,
    fileTransferredBytes: number,
    fileSize: number,
    force = false
  ): void {
    const now = Date.now();
    if (!force && now - this.lastProgressAt < PROGRESS_INTERVAL_MS) return;

    const totalBytes = this.transferTotalBytes || fileSize;
    const transferredBytes = Math.min(
      totalBytes,
      Math.max(0, this.transferredBeforeCurrentFile + fileTransferredBytes)
    );
    this.samples.push({ at: now, bytes: transferredBytes });
    this.samples = this.samples.filter((sample) => now - sample.at <= 3_000);

    const first = this.samples[0];
    const last = this.samples[this.samples.length - 1];
    const elapsedSeconds = first ? (last.at - first.at) / 1000 : 0;
    const speedBytesPerSecond =
      elapsedSeconds > 0 ? Math.max(0, (last.bytes - first.bytes) / elapsedSeconds) : 0;
    const remainingBytes = Math.max(0, totalBytes - transferredBytes);
    const remainingSeconds =
      speedBytesPerSecond > 0 && remainingBytes > 0
        ? Math.ceil(remainingBytes / speedBytesPerSecond)
        : null;

    this.lastProgressAt = now;
    this.onProgress({
      fileName,
      transferredBytes,
      totalBytes,
      progress: totalBytes === 0 ? 100 : (transferredBytes / totalBytes) * 100,
      speedBytesPerSecond: Math.round(speedBytesPerSecond),
      remainingSeconds,
      elapsedSeconds: Math.max(0, Math.floor((now - this.transferStartedAt) / 1_000)),
      status: "uploading",
    });
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => window.setTimeout(resolve, ms));
  }

  private async uploadChunkWithRetry(
    transferId: string,
    index: number,
    buffer: ArrayBuffer,
    fileId: string | undefined,
    onProgress: (loadedBytes: number, totalBytes: number) => void
  ): Promise<void> {
    const maxAttempts = 5;
    let delay = 500;
    for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
      if (this.cancelled) throw new Error("cancelled");
      try {
        await uploadChunkWithProgress(
          transferId,
          index,
          buffer,
          this.sessionToken,
          onProgress,
          fileId
        );
        return;
      } catch (error) {
        if (attempt === maxAttempts || this.cancelled) throw error;
        await this.sleep(delay);
        delay *= 2;
      }
    }
  }

  async cancelTransfer(transferId: string): Promise<void> {
    this.cancelled = true;
    await cancelTransferApi(transferId, this.sessionToken);
  }

  cancel(): void {
    this.cancelled = true;
  }
}
