import { invoke } from '@tauri-apps/api/core';

export async function fetchUrlContent(url: string): Promise<string> {
    try {
        return await invoke<string>('fetch_url_content', { url });
    } catch (error) {
        console.error('Failed to fetch URL via Tauri:', error);
        throw error;
    }
}

export async function proxyHlsRequest(url: string): Promise<ArrayBuffer> {
    try {
        const bytes = await invoke<number[]>('proxy_hls_request', { url });
        return Uint8Array.from(bytes).buffer;
    } catch (error) {
        console.error('Failed to proxy HLS request:', error);
        throw error;
    }
}

export interface CacheResult {
    success: boolean;
    local_url?: string;
    error?: string;
}

export async function cacheAndGetLocalUrl(url: string): Promise<CacheResult> {
    try {
        return await invoke<CacheResult>('cache_and_get_local_url', { url });
    } catch (error) {
        console.error('Failed to cache and get local URL:', error);
        throw error;
    }
}

let proxyServerPort: number | null = null;

export async function getProxyUrl(originalUrl: string): Promise<string> {
    if (!proxyServerPort) {
        proxyServerPort = await invoke<number>('start_hls_proxy_server');
        console.log('HLS proxy server started on port:', proxyServerPort);
    }

    const encodedUrl = encodeURIComponent(originalUrl);
    return `http://127.0.0.1:${proxyServerPort}/proxy/${encodedUrl}`;
}

