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
    epgName?: string;
    tvgId?: string;
    // 回看配置（kodi / tivimate m3u 约定）
    catchup?: string;
    catchupSource?: string;
    catchupDays?: number;
    catchupCorrection?: number;
}

export interface EPGProgram {
    channel: string;
    start: Date;
    stop: Date;
    title: string;
    description?: string;
    icon?: string;
}

export interface EPGChannel {
    id: string;
    displayName: string;
    icon?: string;
}

export interface EPGSubscription {
    id: string;
    name: string;
    url: string;
    type: 'xmltv' | 'diyp';
    programs: EPGProgram[];
    channels: EPGChannel[];
    lastUpdate?: number;
    createdAt: number;
    updatedAt: number;
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

