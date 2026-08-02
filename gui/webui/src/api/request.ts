import type { ApiResponse } from '@/types';

type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

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
    options: { method: Method; body?: Record<string, unknown> | FormData },
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
    if (!payload.ok || payload.data == null) {
      throw new Error(payload.message || 'Request failed');
    }
    return payload.data;
  }

  get<T>(url: string, body?: any) {
    return this.request<T>(url, { method: 'GET', body });
  }

  post<T>(url: string, body?: any) {
    return this.request<T>(url, { method: 'POST', body });
  }

  put<T>(url: string, body?: any) {
    return this.request<T>(url, { method: 'PUT', body });
  }

  patch<T>(url: string, body?: any) {
    return this.request<T>(url, { method: 'PATCH', body });
  }

  delete<T>(url: string) {
    return this.request<T>(url, { method: 'DELETE' });
  }

  postForm<T>(url: string, body: FormData) {
    return this.request<T>(url, { method: 'POST', body });
  }
}
