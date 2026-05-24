# Pattern E: http-client-wrapper.ts

Generate when **HttpClient** (at-realtime-common or similar) is detected. Wrap get/post/put/delete individually (different arities). Use `parsePathAndQuery` for URLs with `%3F` in pathname.

## Implementation

```typescript
// src/wiretap/adapters/http-client-wrapper.ts

import {
  HttpClient,
  IHttpRequestOptions,
} from "at-realtime-common/http-client";
import { Wiretap } from "../wiretap";

export function wrapHttpClient(httpClient: HttpClient, wiretap: Wiretap): void {
  const originalGet = httpClient.get.bind(httpClient);
  httpClient.get = async (url: string, options?: IHttpRequestOptions) => {
    try {
      const response = await originalGet(url, options);
      recordCall(wiretap, "GET", url, response);
      return response;
    } catch (error) {
      recordFailedCall(wiretap, "GET", url, error);
      throw error;
    }
  };

  const originalPost = httpClient.post.bind(httpClient);
  httpClient.post = async (
    url: string,
    data: unknown,
    options?: IHttpRequestOptions,
  ) => {
    try {
      const response = await originalPost(url, data, options);
      recordCall(wiretap, "POST", url, response, data);
      return response;
    } catch (error) {
      recordFailedCall(wiretap, "POST", url, error, data);
      throw error;
    }
  };

  const originalPut = httpClient.put.bind(httpClient);
  httpClient.put = async (url: string, options?: IHttpRequestOptions) => {
    try {
      const response = await originalPut(url, options);
      recordCall(wiretap, "PUT", url, response);
      return response;
    } catch (error) {
      recordFailedCall(wiretap, "PUT", url, error);
      throw error;
    }
  };

  const originalDelete = httpClient.delete.bind(httpClient);
  httpClient.delete = async (url: string, options?: IHttpRequestOptions) => {
    try {
      const response = await originalDelete(url, options);
      recordCall(wiretap, "DELETE", url, response);
      return response;
    } catch (error) {
      recordFailedCall(wiretap, "DELETE", url, error);
      throw error;
    }
  };
}

function parseIfString(value: unknown): string | object | undefined {
  if (typeof value === "undefined") return undefined;
  if (typeof value !== "string") return value as object;
  try { return JSON.parse(value) as object; } catch { return value; }
}

function parsePathAndQuery(parsed: URL): { path: string; query: string } {
  const decoded = decodeURIComponent(parsed.pathname);
  const idx = decoded.indexOf("?");
  if (idx !== -1) {
    const extraQuery = decoded.substring(idx + 1);
    const existingQuery = parsed.search ? parsed.search.substring(1) : "";
    return {
      path: decoded.substring(0, idx),
      query: existingQuery ? `${existingQuery}&${extraQuery}` : extraQuery,
    };
  }
  return {
    path: parsed.pathname,
    query: parsed.search ? parsed.search.substring(1) : "",
  };
}

function recordCall(
  wiretap: Wiretap,
  method: string,
  url: string,
  response: { status: number; data: unknown },
  requestBody?: unknown,
): void {
  try {
    const session = wiretap.getCurrentSession();
    if (!session) return;
    const parsed = new URL(url);
    const { path, query } = parsePathAndQuery(parsed);
    session.captureHttpCall({
      authority: parsed.host,
      method,
      path,
      request: parseIfString(requestBody) || query,
      response: { status: response.status, body: response.data },
    });
  } catch {
    /* never break the actual HTTP call */
  }
}

function recordFailedCall(
  wiretap: Wiretap,
  method: string,
  url: string,
  error: unknown,
  requestBody?: unknown,
): void {
  try {
    const session = wiretap.getCurrentSession();
    if (!session) return;
    const parsed = new URL(url);
    const { path, query } = parsePathAndQuery(parsed);
    const axiosResponse = (error as { response?: { status: number; data?: unknown; headers?: { etag?: string } } })?.response;
    let response: { status: number; body?: unknown; headers?: { etag: string } } | { error: string };
    if (axiosResponse) {
      if (axiosResponse.status === 304) {
        const etag = axiosResponse.headers?.etag as string | undefined;
        response = { status: 304, ...(etag ? { headers: { etag } } : {}) };
      } else {
        response = { status: axiosResponse.status, body: axiosResponse.data };
      }
    } else {
      response = { error: error instanceof Error ? error.message : String(error) };
    }
    session.captureHttpCall({
      authority: parsed.host,
      method,
      path,
      request: parseIfString(requestBody) || query,
      response,
    });
  } catch {
    /* never break the actual HTTP call */
  }
}
```

## Bootstrap

```typescript
wrapHttpClient(Config.httpClient, wiretap);
```
