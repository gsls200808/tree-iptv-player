import type { Subscription, ChannelItem } from '../types';
import { fetchUrlContent } from './tauriApi';

const STORAGE_KEY = 'stream_subscriptions';

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

export async function fetchPlaylist(url: string): Promise<ChannelItem[]> {
    try {
        const content = await fetchUrlContent(url);
        return parseM3U(content);
    } catch (error) {
        console.error('Failed to fetch playlist:', error);
        throw error;
    }
}

export async function detectStreamType(url: string): Promise<'single' | 'playlist'> {
    try {
        const content = await fetchUrlContent(url);

        const lines = content.split('\n').map(line => line.trim());

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];

            if (line.startsWith('#EXTINF:')) {
                const nextLine = lines[i + 1];

                if (nextLine && !nextLine.startsWith('#')) {
                    if (nextLine.startsWith('http://') || nextLine.startsWith('https://')) {
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

