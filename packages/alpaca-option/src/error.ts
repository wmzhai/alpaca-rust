export class OptionError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'OptionError';
    this.code = code;
  }
}

export function fail(code: string, message: string): never {
  throw new OptionError(code, message);
}
