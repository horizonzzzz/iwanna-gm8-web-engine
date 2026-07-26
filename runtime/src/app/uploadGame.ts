export type UploadResponse = {
  id: string;
  status: 'ready';
  compatibility: 'supported' | 'partial' | 'blocked';
  package_url: string;
  warnings: string[];
};

export type UploadProgress = {
  /** `uploading` while request bytes are in flight, `processing` once the server is parsing. */
  phase: 'uploading' | 'processing';
  /** Upload completion in the 0-100 range; stays at 100 during `processing`. */
  percent: number;
};

type UploadErrorBody = {
  error?: string;
};

function parseResponseBody(raw: unknown): UploadResponse | UploadErrorBody | null {
  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw) as UploadResponse | UploadErrorBody;
    } catch {
      return null;
    }
  }
  return raw && typeof raw === 'object' ? raw as UploadResponse | UploadErrorBody : null;
}

/**
 * Upload a game package with real upload progress. `fetch` cannot report
 * request-body progress, so this uses `XMLHttpRequest` directly.
 */
export function uploadGamePackage(
  file: File,
  onProgress: (progress: UploadProgress) => void
): Promise<UploadResponse> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open('POST', '/api/v1/games');

    request.upload.addEventListener('progress', (event) => {
      if (!event.lengthComputable || event.total <= 0) {
        return;
      }
      const percent = Math.min(100, Math.round((event.loaded / event.total) * 100));
      onProgress({ phase: percent >= 100 ? 'processing' : 'uploading', percent });
    });
    request.upload.addEventListener('load', () => {
      onProgress({ phase: 'processing', percent: 100 });
    });

    request.addEventListener('error', () => {
      reject(new Error('网络错误，上传中断。'));
    });
    request.addEventListener('abort', () => {
      reject(new Error('上传已取消。'));
    });
    request.addEventListener('load', () => {
      const body = parseResponseBody(request.response);
      if (request.status >= 200 && request.status < 300 && body && 'package_url' in body) {
        resolve(body);
        return;
      }
      const message = body && 'error' in body && body.error
        ? body.error
        : `上传失败（HTTP ${request.status}）`;
      reject(new Error(message));
    });

    const form = new FormData();
    form.append('game', file);
    request.send(form);
  });
}
