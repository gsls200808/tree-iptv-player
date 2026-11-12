import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { readTextFile } from '@tauri-apps/plugin-fs';

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

export async function startRtspProxy(rtspUrl: string): Promise<string> {
    const port = await invoke<number>('start_rtsp_proxy', { rtspUrl });
    console.log('RTSP proxy started on port:', port);
    return `http://127.0.0.1:${port}/playlist.m3u8`;
}

export async function stopRtspProxy(rtspUrl: string): Promise<void> {
    await invoke('stop_rtsp_proxy', { rtspUrl });
    console.log('RTSP proxy stopped for:', rtspUrl);
}

// 检测视频流是否为隔行扫描
export async function detectInterlaced(url: string): Promise<boolean> {
    return await invoke<boolean>('detect_interlaced', { url });
}

// 通过 FFmpeg 转码为本地 HLS：deinterlace 开启反交错，音频统一转 AAC 双声道
export async function startFfmpegProxy(url: string, deinterlace: boolean): Promise<string> {
    const port = await invoke<number>('start_ffmpeg_proxy', { url, deinterlace });
    console.log('FFmpeg proxy started on port:', port, 'deinterlace:', deinterlace);
    return `http://127.0.0.1:${port}/playlist.m3u8`;
}

export async function stopFfmpegProxy(url: string): Promise<void> {
    await invoke('stop_ffmpeg_proxy', { url });
    console.log('FFmpeg proxy stopped for:', url);
}

export async function selectPlaylistFile(): Promise<string | null> {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Playlist Files',
                extensions: ['m3u', 'm3u8', 'mpd']
            }]
        });
        
        if (selected && typeof selected === 'string') {
            return selected;
        }
        return null;
    } catch (error) {
        console.error('Failed to select file:', error);
        throw error;
    }
}

export async function readLocalFile(filePath: string): Promise<string> {
    try {
        return await readTextFile(filePath);
    } catch (error) {
        console.error('Failed to read file:', error);
        throw error;
    }
}

