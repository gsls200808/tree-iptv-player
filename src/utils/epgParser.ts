import type { EPGProgram, EPGChannel } from '../types';

export function parseXMLTV(content: string): { programs: EPGProgram[]; channels: EPGChannel[] } {
    const parser = new DOMParser();
    const xml = parser.parseFromString(content, 'text/xml');

    const channels: EPGChannel[] = [];
    const channelElements = xml.getElementsByTagName('channel');

    for (let i = 0; i < channelElements.length; i++) {
        const channelEl = channelElements[i];
        const id = channelEl.getAttribute('id') || '';
        const displayNameEl = channelEl.getElementsByTagName('display-name')[0];
        const iconEl = channelEl.getElementsByTagName('icon')[0];

        if (displayNameEl) {
            channels.push({
                id,
                displayName: displayNameEl.textContent || '',
                icon: iconEl?.getAttribute('src'),
            });
        }
    }

    const programs: EPGProgram[] = [];
    const programmeElements = xml.getElementsByTagName('programme');

    for (let i = 0; i < programmeElements.length; i++) {
        const progEl = programmeElements[i];
        const channel = progEl.getAttribute('channel') || '';
        const start = progEl.getAttribute('start') || '';
        const stop = progEl.getAttribute('stop') || '';
        const titleEl = progEl.getElementsByTagName('title')[0];
        const descEl = progEl.getElementsByTagName('desc')[0];
        const iconEl = progEl.getElementsByTagName('icon')[0];

        if (titleEl) {
            programs.push({
                channel,
                start: parseXMLTVDate(start),
                stop: parseXMLTVDate(stop),
                title: titleEl.textContent || '',
                description: descEl?.textContent,
                icon: iconEl?.getAttribute('src'),
            });
        }
    }

    return { programs, channels };
}

function parseXMLTVDate(dateStr: string): Date {
    if (!dateStr) return new Date();

    const year = dateStr.substring(0, 4);
    const month = dateStr.substring(4, 6);
    const day = dateStr.substring(6, 8);
    const hour = dateStr.substring(8, 10);
    const minute = dateStr.substring(10, 12);
    const second = dateStr.substring(12, 14);

    return new Date(`${year}-${month}-${day}T${hour}:${minute}:${second}Z`);
}

export function parseDIYP(content: string): { programs: EPGProgram[]; channels: EPGChannel[] } {
    try {
        const data = JSON.parse(content);
        const programs: EPGProgram[] = [];
        const channels: EPGChannel[] = [];

        if (data.epg_data && Array.isArray(data.epg_data)) {
            const channelSet = new Set<string>();

            data.epg_data.forEach((item: any) => {
                const channelName = item.channel_name || '';
                if (channelName && !channelSet.has(channelName)) {
                    channels.push({
                        id: channelName,
                        displayName: channelName,
                    });
                    channelSet.add(channelName);
                }

                if (item.program && Array.isArray(item.program)) {
                    item.program.forEach((prog: any) => {
                        const startStr = prog.start_time || prog.start || '';
                        const endStr = prog.end_time || prog.end || '';

                        if (startStr && channelName) {
                            programs.push({
                                channel: channelName,
                                start: new Date(startStr),
                                stop: new Date(endStr || startStr),
                                title: prog.title || '',
                                description: prog.desc,
                            });
                        }
                    });
                }
            });
        }

        return { programs, channels };
    } catch (e) {
        console.error('Failed to parse DIYP EPG:', e);
        return { programs: [], channels: [] };
    }
}

export function findCurrentProgram(programs: EPGProgram[], channelName: string): EPGProgram | null {
    const now = new Date();
    const channelPrograms = programs.filter(p =>
        p.channel.toLowerCase() === channelName.toLowerCase()
    );

    const currentProgram = channelPrograms.find(p => p.start <= now && p.stop >= now);
    return currentProgram || null;
}

export function findProgramsByChannel(programs: EPGProgram[], channelName: string): EPGProgram[] {
    return programs
        .filter(p => {
            const pChannel = typeof p.channel === 'string' ? p.channel : '';
            const searchChannel = typeof channelName === 'string' ? channelName : '';
            return pChannel.toLowerCase() === searchChannel.toLowerCase();
        })
        .sort((a, b) => {
            const aTime = a.start instanceof Date ? a.start.getTime() : new Date(a.start).getTime();
            const bTime = b.start instanceof Date ? b.start.getTime() : new Date(b.start).getTime();
            return aTime - bTime;
        });
}

export function matchChannelWithEPG(channelName: string, epgChannels: EPGChannel[]): string | null {
    const normalizedChannel = channelName.toLowerCase().trim();

    const exactMatch = epgChannels.find(ch =>
        ch.displayName.toLowerCase().trim() === normalizedChannel
    );

    if (exactMatch) {
        return exactMatch.id;
    }

    const partialMatch = epgChannels.find(ch =>
        ch.displayName.toLowerCase().includes(normalizedChannel) ||
        normalizedChannel.includes(ch.displayName.toLowerCase().trim())
    );

    if (partialMatch) {
        return partialMatch.id;
    }

    return null;
}

