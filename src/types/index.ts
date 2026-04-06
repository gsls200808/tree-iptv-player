export interface StreamItem {
    id: string;
    name: string;
    url: string;
    type: 'single' | 'playlist';
    channels?: ChannelItem[];
    createdAt: number;
}

export interface ChannelItem {
    name: string;
    url: string;
    logo?: string;
    group?: string;
}

export interface Subscription {
    id: string;
    url: string;
    name: string;
    type: 'single' | 'playlist';
    channels: ChannelItem[];
    currentChannelIndex: number;
    createdAt: number;
    updatedAt: number;
}
