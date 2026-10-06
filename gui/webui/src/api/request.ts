import type { ApiResponse } from '@/types';
import { toast } from '@/components/toast';

type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export type RequestCallOptions = {
  suppressInfoMessage?: boolean;
};

type RequestOptions = {
  base?: string;
  bearer?: string;
  timeout?: number;
  beforeRequest?: () => void;
};

export class Request {
  base: string;
  bearer: string;
  timeout: number;
  beforeRequest: () => void;

  constructor(options: RequestOptions = {}) {
    this.base = options.base || '';
    this.bearer = options.bearer || '';
    this.timeout = options.timeout || 10_000;
    this.beforeRequest = options.beforeRequest || (() => undefined);
  }

  private async request<T>(
    url: string,
    options: {
      method: Method;
      body?: Record<string, unknown> | FormData;
    } & RequestCallOptions,
  ) {
    this.beforeRequest();

    const init: RequestInit = {
      method: options.method,
      signal: AbortSignal.timeout(this.timeout),
      headers: {},
    };

    if (this.base) {
      url = this.base + url;
    }

    if (this.bearer) {
      Object.assign(init.headers as Record<string, string>, {
        Authorization: `Bearer ${this.bearer}`,
      });
    }

    if (options.method === 'GET' && options.body) {
      const query = new URLSearchParams(
        options.body as Record<string, string>,
      ).toString();
      if (query) {
        url += `?${query}`;
      }
    }

    if (['POST', 'PUT', 'PATCH'].includes(options.method)) {
      if (options.body instanceof FormData) {
        init.body = options.body;
      } else {
        Object.assign(init.headers as Record<string, string>, {
          'content-type': 'application/json',
        });
        init.body = JSON.stringify(options.body || {});
      }
    }

    const response = await fetch(url, init);
    if (response.status === 204) {
      return null as T;
    }
    const payload = (await response.json()) as ApiResponse<T>;
    if (!payload.ok) {
      throw new Error(payload.message || 'Request failed');
    }

    if (
      payload.message &&
      !(
        options.suppressInfoMessage &&
        (payload.message_level === undefined ||
          payload.message_level === 'info')
      )
    ) {
      switch (payload.message_level) {
        case 'warn':
          toast.warn({ title: payload.message });
          break;
        case 'error':
          toast.error({ title: payload.message });
          break;
        default:
          toast.info({ title: payload.message });
          break;
      }
    }

    // Keep existing non-null wrapper contracts; no-data operations discard this
    // value at their API boundary.
    return payload.data as T;
  }

  get<T>(url: string, body?: any, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'GET', body, ...options });
  }

  post<T>(url: string, body?: any, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'POST', body, ...options });
  }

  put<T>(url: string, body?: any, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'PUT', body, ...options });
  }

  patch<T>(url: string, body?: any, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'PATCH', body, ...options });
  }

  delete<T>(url: string, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'DELETE', ...options });
  }

  postForm<T>(url: string, body: FormData, options?: RequestCallOptions) {
    return this.request<T>(url, { method: 'POST', body, ...options });
  }
}
