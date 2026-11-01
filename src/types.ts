export interface CurrentInfo {
  success: boolean;
  email?: string;
  displayName?: string;
  organization?: string;
  orgRole?: string;
  accountUuid?: string;
  billing?: string;
  error?: string;
}

export interface ProfileEntry {
  name: string;
  email: string;
  organization: string;
  displayName: string;
  billing: string;
  expiresAt: number;
  isExpired: boolean;
  /** Subscription plan, e.g. "Max 20x", "Pro"; empty if unknown. */
  plan: string;
}

export interface UsageResult {
  success: boolean;
  error?: string;
  /** On HTTP 429: seconds the server asked us to wait. */
  retryAfterSecs?: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data?: any;
}

export interface OpResult {
  success: boolean;
  message?: string;
  error?: string;
  email?: string;
  organization?: string;
  from?: string;
  to?: string;
  expiresAt?: string;
  accessTokenPreview?: string;
  /** Refresh was not needed (token still valid); no request was made. */
  skipped?: boolean;
}
