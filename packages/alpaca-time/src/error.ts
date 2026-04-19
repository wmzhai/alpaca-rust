export class TimeError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'TimeError';
    this.code = code;
  }
}

export function fail(code: string, message: string): never {
  throw new TimeError(code, message);
}
