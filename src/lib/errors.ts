export interface BackendErrorPayload {
  code: string;
  message: string;
}

export class AppError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'AppError';
    this.code = code;
  }
}

const FALLBACK_MESSAGE = 'Something went wrong while contacting the local backend. Please retry.';

export function toAppError(error: unknown): AppError {
  if (error instanceof AppError) return error;

  if (isBackendError(error)) {
    return new AppError(error.code, sanitizeMessage(error.message));
  }

  if (error instanceof Error) {
    return new AppError('UNKNOWN_ERROR', sanitizeMessage(error.message));
  }

  return new AppError('UNKNOWN_ERROR', FALLBACK_MESSAGE);
}

export function safeMessage(error: unknown): string {
  return toAppError(error).message;
}

export function isBackendError(value: unknown): value is BackendErrorPayload {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as BackendErrorPayload).code === 'string' &&
    typeof (value as BackendErrorPayload).message === 'string'
  );
}

function sanitizeMessage(message: string): string {
  const trimmed = message.trim();
  if (!trimmed) return FALLBACK_MESSAGE;
  return trimmed.slice(0, 280);
}
