import type { Subscription, ChannelItem } from '../types';
import { fetchUrlContent } from './tauriApi';

const STORAGE_KEY = 'stream_subscriptions';
const EPG_STORAGE_KEY = 'epg_subscriptions';

export function parseM3U(content: string): ChannelItem[] {
    const lines = content.split('\n').map(line => line.trim());
    const channels: ChannelItem[] = [];

    let currentChannel: Partial<ChannelItem> = {};

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];

        if (line.startsWith('#EXTINF:')) {
            const nameMatch = line.match(/,(.+)$/);
            const logoMatch = line.match(/tvg-logo="([^"]*)"/);
            const groupMatch = line.match(/group-title="([^"]*)"/);

            currentChannel = {
                name: nameMatch ? nameMatch[1].trim() : `Channel ${channels.length + 1}`,
                logo: logoMatch ? logoMatch[1] : undefined,
                group: groupMatch ? groupMatch[1] : undefined,
            };
        } else if (line && !line.startsWith('#') && currentChannel.name) {
            currentChannel.url = line;
            channels.push(currentChannel as ChannelItem);
            currentChannel = {};
        }
    }

    return channels;
}

export function parseDIYPTxt(content: string): ChannelItem[] {
    const lines = content.split('\n').map(line => line.trim()).filter(line => line);
    const channels: ChannelItem[] = [];
    let currentGroup = '未分组';

    for (const line of lines) {
        if (line.includes(',#genre#')) {
            const parts = line.split(',');
            const groupName = parts[0].trim();
            currentGroup = groupName || '未分组';
        } else if (line.includes(',')) {
            const parts = line.split(',');
            const name = parts[0].trim();
            const url = parts[1].trim();

            if (name && url) {
                channels.push({
                    name,
                    url,
                    group: currentGroup,
                });
            }
        }
    }

    return channels;
}

export function parseStandardTxt(content: string): ChannelItem[] {
    const lines = content.split('\n').map(line => line.trim()).filter(line => line);
    const channels: ChannelItem[] = [];

    for (const line of lines) {
        if (line.includes(',')) {
            const parts = line.split(',');
            const name = parts[0].trim();
            const url = parts[1].trim();

            if (name && url) {
                channels.push({
                    name,
                    url,
                    group: '未分组',
                });
            }
        }
    }

    return channels;
}

export function parsePlaylist(content: string): ChannelItem[] {
    const trimmedContent = content.trim();

    if (trimmedContent.startsWith('#EXTM3U')) {
        return parseM3U(content);
    }

    if (trimmedContent.includes(',#genre#')) {
        return parseDIYPTxt(content);
    }

    return parseStandardTxt(content);
}

export async function fetchPlaylist(url: string): Promise<ChannelItem[]> {
    try {
        const content = await fetchUrlContent(url);
        return parsePlaylist(content);
    } catch (error) {
        console.error('Failed to fetch playlist:', error);
        throw error;
    }
}

export async function detectStreamType(url: string): Promise<'single' | 'playlist'> {
    try {
        const content = await fetchUrlContent(url);
        const trimmedContent = content.trim();

        // 检测 M3U 格式
        if (trimmedContent.startsWith('#EXTM3U')) {
            return 'playlist';
        }

        const lines = content.split('\n').map(line => line.trim()).filter(line => line);

        // 检测 DIYP TXT 格式（包含 ,#genre#）
        if (trimmedContent.includes(',#genre#')) {
            return 'playlist';
        }

        // 检测标准 TXT 格式（多行包含逗号的格式：名称,URL）
        let commaLineCount = 0;
        for (const line of lines) {
            if (line.includes(',') && !line.startsWith('#')) {
                const parts = line.split(',');
                // 检查是否是 名称,URL 格式
                if (parts.length >= 2 && (parts[1].trim().startsWith('http') || parts[1].trim().startsWith('rtsp://'))) {
                    commaLineCount++;
                    // 如果找到至少两行这样的格式，判定为播放列表
                    if (commaLineCount >= 2) {
                        return 'playlist';
                    }
                }
            }
        }

        // 检测 M3U 格式的另一种方式
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];

            if (line.startsWith('#EXTINF:')) {
                const nextLine = lines[i + 1];

                if (nextLine && !nextLine.startsWith('#')) {
                    if (nextLine.startsWith('http://') || nextLine.startsWith('https://') || nextLine.startsWith('rtsp://')) {
                        return 'playlist';
                    } else {
                        return 'single';
                    }
                }
            }
        }

        return 'single';
    } catch (error) {
        console.error('Failed to detect stream type:', error);
        return 'single';
    }
}

export function isPlaylistUrl(url: string): boolean {
    return url.endsWith('.m3u') || url.endsWith('.m3u8');
}

export function getSubscriptions(): Subscription[] {
    const data = localStorage.getItem(STORAGE_KEY);
    return data ? JSON.parse(data) : [];
}

export function saveSubscription(subscription: Subscription): void {
    const subscriptions = getSubscriptions();
    const index = subscriptions.findIndex(s => s.id === subscription.id);

    if (index >= 0) {
        subscriptions[index] = subscription;
    } else {
        subscriptions.push(subscription);
    }

    localStorage.setItem(STORAGE_KEY, JSON.stringify(subscriptions));
}

export function deleteSubscription(id: string): void {
    const subscriptions = getSubscriptions();
    const filtered = subscriptions.filter(s => s.id !== id);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(filtered));
}

export function updateSubscription(subscription: Subscription): void {
    saveSubscription(subscription);
}

export function getEPGSubscriptions(): any[] {
    const data = localStorage.getItem(EPG_STORAGE_KEY);
    return data ? JSON.parse(data) : [];
}

export function saveEPGSubscription(epgSub: any): void {
    const subscriptions = getEPGSubscriptions();
    const index = subscriptions.findIndex(s => s.id === epgSub.id);

    if (index >= 0) {
        subscriptions[index] = epgSub;
    } else {
        subscriptions.push(epgSub);
    }

    localStorage.setItem(EPG_STORAGE_KEY, JSON.stringify(subscriptions));
}

export function deleteEPGSubscription(id: string): void {
    const subscriptions = getEPGSubscriptions();
    const filtered = subscriptions.filter(s => s.id !== id);
    localStorage.setItem(EPG_STORAGE_KEY, JSON.stringify(filtered));
}

export function updateEPGSubscription(epgSub: any): void {
    saveEPGSubscription(epgSub);
}


