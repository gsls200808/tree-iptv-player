import type { ChannelItem } from '../types';

export interface CatchupProgram {
    // 节目单里的时间，格式 HH:mm
    start: string;
    end: string;
}

// 未声明 catchup-days 时默认允许回看 7 天
const DEFAULT_CATCHUP_DAYS = 7;

const pad = (n: number) => String(n).padStart(2, '0');

function formatDateTime(date: Date): string {
    return (
        `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}` +
        `${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`
    );
}

function parseProgramRange(program: CatchupProgram, date: Date) {
    const [sh, sm] = program.start.split(':').map(Number);
    const [eh, em] = program.end.split(':').map(Number);

    const start = new Date(date.getFullYear(), date.getMonth(), date.getDate(), sh || 0, sm || 0, 0, 0);
    let end = new Date(date.getFullYear(), date.getMonth(), date.getDate(), eh || 0, em || 0, 0, 0);

    // 跨零点的节目（如 23:30 - 00:30）结束时间顺延到次日
    if (end.getTime() <= start.getTime()) {
        end = new Date(end.getTime() + 24 * 60 * 60 * 1000);
    }

    return { start, end };
}

/** 频道是否支持回看：声明了 catchup 属性，或 URL 是 PLTV / TVOD 时移地址 */
export function supportsCatchup(channel?: ChannelItem | null): boolean {
    if (!channel) return false;
    if (channel.catchupSource || channel.catchup) return true;

    const url = channel.url.toUpperCase();
    return url.includes('PLTV') || url.includes('TVOD');
}

/** 可回看天数，未配置时按 7 天处理 */
export function catchupDays(channel?: ChannelItem | null): number {
    const days = channel?.catchupDays;
    return typeof days === 'number' && days > 0 ? days : DEFAULT_CATCHUP_DAYS;
}

/**
 * 生成回看地址
 * - 有 catchup-source：按模板替换 {start} {end} {utc} {utcend} {lutc} {duration} {channel}
 * - catchup=flussonic：/timeshift/{start}/{end}/{原文件名}
 * - catchup=shift：追加 start / end
 * - catchup=append / default：追加 utc / lutc（kodi 约定）
 * - 兜底：PLTV / TVOD 使用 playseek
 * 返回 null 表示该频道不支持回看
 */
export function buildCatchupUrl(
    channel: ChannelItem,
    program: CatchupProgram,
    date: Date
): string | null {
    const { start, end } = parseProgramRange(program, date);
    const correction = channel.catchupCorrection || 0;

    const startSec = Math.floor(start.getTime() / 1000) + correction;
    const endSec = Math.floor(end.getTime() / 1000) + correction;
    const nowSec = Math.floor(Date.now() / 1000);

    const vars: Record<string, string> = {
        start: formatDateTime(new Date(startSec * 1000)),
        end: formatDateTime(new Date(endSec * 1000)),
        utc: String(startSec),
        utcend: String(endSec),
        lutc: String(nowSec),
        duration: String(endSec - startSec),
        channel: channel.tvgId || channel.name,
    };

    const applyVars = (template: string) =>
        template.replace(/\$\{(\w+)\}|\{(\w+)\}/g, (raw, braced, plain) => {
            const key = (braced || plain) as string;
            return key in vars ? vars[key] : raw;
        });

    // 1) 显式模板，kodi / tivimate 通用
    if (channel.catchupSource) {
        return applyVars(channel.catchupSource);
    }

    const type = (channel.catchup || '').toLowerCase();
    const base = channel.url.split('?')[0];
    const withParams = (params: string) =>
        `${channel.url}${channel.url.includes('?') ? '&' : '?'}${params}`;

    if (type === 'flussonic') {
        const segments = base.split('/');
        const file = segments.pop() || '';
        return `${segments.join('/')}/timeshift/${vars.start}/${vars.end}/${file}`;
    }

    if (type === 'shift') {
        return withParams(`start=${vars.start}&end=${vars.end}`);
    }

    if (type === 'append' || type === 'default') {
        return withParams(`utc=${vars.utc}&lutc=${vars.lutc}`);
    }

    // 2) PLTV / TVOD 兜底，保持原有行为
    const upper = channel.url.toUpperCase();
    if (upper.includes('PLTV') || upper.includes('TVOD')) {
        return `${base}?playseek=${vars.start}-${vars.end}`;
    }

    return null;
}
