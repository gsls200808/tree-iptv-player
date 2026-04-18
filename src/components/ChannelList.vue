<template>
  <div v-if="channels.length > 0" class="channel-list">
    <div class="channel-list-header">
      <h4>频道列表 ({{ channels.length }})</h4>

      <div class="tab-buttons">
        <button
            :class="['tab-btn', { active: displayMode === 'all' && activeTab === 'channels' }]"
            @click="displayMode = 'all'; activeTab = 'channels'"
        >
          📺 全部
        </button>
        <button
            :class="['tab-btn', { active: displayMode === 'group' && activeTab === 'channels' }]"
            @click="displayMode = 'group'; activeTab = 'channels'"
        >
          📁 分组
        </button>
        <button
            v-if="hasEPGSupport"
            :class="['tab-btn', { active: activeTab === 'epg' }]"
            @click="activeTab = 'epg'"
        >
          📋 节目表
        </button>
      </div>
    </div>

    <div v-if="activeTab === 'channels' && displayMode === 'all'" class="channels">
      <div
          v-for="(channel, index) in channels"
          :key="index"
          :class="['channel-item', { active: index === activeIndex }]"
          @click="$emit('select', index)"
      >
        <img
            v-if="channel.logo"
            :src="channel.logo"
            :alt="channel.name"
            class="channel-logo"
            @error="handleImageError"
        />
        <div class="channel-info">
          <div class="channel-name">{{ channel.name }}</div>
          <div v-if="channel.group" class="channel-group">{{ channel.group }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'channels' && displayMode === 'group'" class="groups">
      <div v-if="!selectedGroup" class="group-list">
        <div
            v-for="(groupChannels, groupName) in groupedChannels"
            :key="groupName"
            class="group-item"
            @click="selectedGroup = groupName"
        >
          <div class="group-icon">📁</div>
          <div class="group-info">
            <div class="group-name">{{ groupName }}</div>
            <div class="group-count">{{ groupChannels.length }} 个频道</div>
          </div>
        </div>
      </div>

      <div v-else class="group-channels">
        <div class="group-header">
          <button class="btn-back" @click="selectedGroup = null">
            ◀ 返回分组列表
          </button>
          <h5>{{ selectedGroup }} ({{ groupedChannels[selectedGroup]?.length || 0 }})</h5>
        </div>

        <div class="channels">
          <div
              v-for="channel in groupedChannels[selectedGroup]"
              :key="channel.originalIndex"
              :class="['channel-item', { active: channel.originalIndex === activeIndex }]"
              @click="$emit('select', channel.originalIndex)"
          >
            <img
                v-if="channel.logo"
                :src="channel.logo"
                :alt="channel.name"
                class="channel-logo"
                @error="handleImageError"
            />
            <div class="channel-info">
              <div class="channel-name">{{ channel.name }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'epg'" class="epg-tab">
      <div class="epg-date-selector">
        <button
            class="date-nav-btn"
            @click="changeDate(-1)"
            :disabled="selectedDateOffset <= -6"
        >
          ◀
        </button>

        <div class="date-display">
          <span class="date-text">{{ formattedDate }}</span>
          <span class="relative-date">{{ relativeDateText }}</span>
        </div>

        <button
            class="date-nav-btn"
            @click="changeDate(1)"
            :disabled="selectedDateOffset >= 2"
        >
          ▶
        </button>
      </div>

      <div v-if="loading" class="epg-loading">
        <div class="loading-spinner">⏳</div>
        <p>加载节目表中...</p>
      </div>

      <div v-else-if="error" class="epg-error">
        <p>❌ {{ error }}</p>
        <button @click="loadEPGData" class="btn-retry">重试</button>
      </div>

      <div v-else-if="diypPrograms.length > 0" class="diyp-programs">
        <div class="channel-name-display-only">{{ currentChannelName }}</div>

        <div
            v-for="(program, index) in diypPrograms"
            :key="index"
            :class="['program-item', { 'is-current': isCurrentProgram(program) }]"
        >
          <div class="program-time">
            <span class="start">{{ program.start }}</span>
            <span class="separator">-</span>
            <span class="end">{{ program.end }}</span>
          </div>
          <div class="program-info">
            <div class="program-title">
              <span v-if="isCurrentProgram(program)" class="playing-icon">▶️</span>
              {{ program.title }}
            </div>
            <div v-if="program.desc" class="program-desc">{{ program.desc }}</div>
          </div>
          <button
              v-if="hasPlaybackSupport && canPlayback(program)"
              class="btn-program-playback"
              @click="handleProgramPlayback(program)"
              title="回看此节目"
          >
            ⏮️
          </button>
        </div>
      </div>

      <div v-else class="epg-empty">
        <p>暂无节目信息</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">import { ref, computed, watch } from 'vue';
import type { ChannelItem, EPGSubscription } from '../types';
import { matchChannelWithEPG, findProgramsByChannel } from '../utils/epgParser';

const props = defineProps<{
  channels: ChannelItem[];
  activeIndex?: number;
  epgSubscriptions?: EPGSubscription[];
}>();

const emit = defineEmits<{
  select: [index: number];
  playback: [url: string];
}>();

const activeTab = ref<'channels' | 'epg'>('channels');
const displayMode = ref<'all' | 'group'>('all');
const selectedGroup = ref<string | null>(null);
const loading = ref(false);
const error = ref<string>('');
const diypPrograms = ref<any[]>([]);
const selectedDateOffset = ref(0);

const hasEPGSupport = computed(() => {
  return props.epgSubscriptions && props.epgSubscriptions.length > 0;
});

const hasPlaybackSupport = computed(() => {
  if (!hasEPGSupport.value || props.activeIndex === undefined) return false;

  const currentChannel = props.channels[props.activeIndex];
  if (!currentChannel) return false;

  const url = currentChannel.url.toUpperCase();
  return url.includes('PLTV') || url.includes('TVOD');
});

const currentChannelName = computed(() => {
  if (props.activeIndex !== undefined && props.channels[props.activeIndex]) {
    return props.channels[props.activeIndex].name;
  }
  return '';
});

const groupedChannels = computed(() => {
  const groups: Record<string, Array<ChannelItem & { originalIndex: number }>> = {};

  props.channels.forEach((channel, index) => {
    const groupName = channel.group || '未分组';
    if (!groups[groupName]) {
      groups[groupName] = [];
    }
    groups[groupName].push({
      ...channel,
      originalIndex: index
    });
  });

  return groups;
});


const selectedDate = computed(() => {
  const now = new Date();
  now.setDate(now.getDate() + selectedDateOffset.value);
  return now;
});

const formattedDate = computed(() => {
  const date = selectedDate.value;
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const weekDays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
  const weekDay = weekDays[date.getDay()];
  return `${year}-${month}-${day} ${weekDay}`;
});

const relativeDateText = computed(() => {
  const offset = selectedDateOffset.value;
  if (offset === 0) return '今天';
  if (offset === 1) return '明天';
  if (offset === 2) return '后天';
  if (offset === -1) return '昨天';
  if (offset < 0) return `${Math.abs(offset)}天前`;
  return '';
});

const isCurrentProgram = (program: any) => {
  if (selectedDateOffset.value !== 0) return false;

  const now = new Date();
  const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const startTime = program.start;
  const endTime = program.end;

  return startTime <= currentTime && endTime >= currentTime;
};

const canPlayback = (program: any) => {
  if (selectedDateOffset.value > 0) return false;

  if (selectedDateOffset.value < 0) {
    return true;
  }

  const now = new Date();
  const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const endTime = program.end;

  return endTime <= currentTime;
};

const changeDate = (delta: number) => {
  const newOffset = selectedDateOffset.value + delta;
  if (newOffset >= -6 && newOffset <= 2) {
    selectedDateOffset.value = newOffset;
    loadEPGData();
  }
};

const loadEPGData = async () => {
  if (!currentChannelName.value || !props.epgSubscriptions?.length) {
    error.value = '没有可用的EPG订阅';
    return;
  }

  loading.value = true;
  error.value = '';
  diypPrograms.value = [];

  let foundPrograms = false;

  for (const epgSub of props.epgSubscriptions) {
    try {
      if (epgSub.type === 'diyp') {
        const url = epgSub.url;
        const date = selectedDate.value;
        const year = date.getFullYear();
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        const dateStr = `${year}-${month}-${day}`;
        const channelName = encodeURIComponent(currentChannelName.value);
        const requestUrl = `${url}?ch=${channelName}&date=${dateStr}`;

        const response = await fetch(requestUrl);

        if (!response.ok) {
          console.warn(`DIYP订阅 ${epgSub.name} 请求失败: ${response.status}`);
          continue;
        }

        const data = await response.json();

        if (data.code === 200 && data.epg_data && data.epg_data.length > 0) {
          diypPrograms.value = data.epg_data;
          foundPrograms = true;
          break;
        } else {
          console.warn(`DIYP订阅 ${epgSub.name} 没有返回节目数据`);
        }
      } else if (epgSub.type === 'xmltv') {
        if (epgSub.programs && epgSub.programs.length > 0) {
          const matchedChannelId = matchChannelWithEPG(currentChannelName.value, epgSub.channels);

          if (matchedChannelId) {
            const channelPrograms = findProgramsByChannel(epgSub.programs, matchedChannelId);

            if (channelPrograms.length > 0) {
              const targetDate = selectedDate.value;
              const targetYear = targetDate.getFullYear();
              const targetMonth = targetDate.getMonth();
              const targetDay = targetDate.getDate();

              const filteredPrograms = channelPrograms.filter(prog => {
                const progDate = prog.start instanceof Date ? prog.start : new Date(prog.start);
                return progDate.getFullYear() === targetYear &&
                    progDate.getMonth() === targetMonth &&
                    progDate.getDate() === targetDay;
              });

              if (filteredPrograms.length > 0) {
                diypPrograms.value = filteredPrograms.map(prog => ({
                  start: formatTime(prog.start),
                  end: formatTime(prog.stop),
                  title: prog.title,
                  desc: prog.description
                }));
                foundPrograms = true;
                break;
              } else {
                console.warn(`XMLTV订阅 ${epgSub.name} 在选定日期没有找到节目`);
              }
            } else {
              console.warn(`XMLTV订阅 ${epgSub.name} 中没有找到频道 ${currentChannelName.value} 的节目`);
            }
          } else {
            console.warn(`XMLTV订阅 ${epgSub.name} 中没有匹配到频道 ${currentChannelName.value}`);
          }
        } else {
          console.warn(`XMLTV订阅 ${epgSub.name} 还没有加载节目数据，请先刷新EPG`);
        }
      }
    } catch (e: any) {
      console.error(`EPG订阅 ${epgSub.name} 加载失败:`, e);
      continue;
    }
  }

  if (!foundPrograms) {
    error.value = '所有EPG订阅均未找到节目信息，请检查订阅配置或刷新EPG';
  }

  loading.value = false;
};

const formatTime = (date: Date | string): string => {
  let d: Date;
  if (date instanceof Date) {
    d = date;
  } else {
    d = new Date(date);
  }

  if (isNaN(d.getTime())) {
    return '00:00';
  }

  const hours = String(d.getHours()).padStart(2, '0');
  const minutes = String(d.getMinutes()).padStart(2, '0');
  return `${hours}:${minutes}`;
};

watch(activeTab, (newTab) => {
  if (newTab === 'epg' && currentChannelName.value) {
    selectedDateOffset.value = 0;
    loadEPGData();
  }
});

watch(() => props.activeIndex, () => {
  if (activeTab.value === 'epg' && currentChannelName.value) {
    selectedDateOffset.value = 0;
    loadEPGData();
  }
});

watch(displayMode, () => {
  selectedGroup.value = null;
});

const handleImageError = (e: Event) => {
  const target = e.target as HTMLImageElement;
  target.style.display = 'none';
};

const handleProgramPlayback = (program: any) => {
  if (props.activeIndex === undefined || !props.channels[props.activeIndex]) return;

  const currentChannel = props.channels[props.activeIndex];

  try {
    const baseUrl = currentChannel.url.split('?')[0];

    const targetDate = selectedDate.value;
    const year = targetDate.getFullYear();
    const month = String(targetDate.getMonth() + 1).padStart(2, '0');
    const day = String(targetDate.getDate()).padStart(2, '0');

    const startParts = program.start.split(':');
    const endParts = program.end.split(':');

    const startTime = `${year}${month}${day}${startParts[0]}${startParts[1]}00`;
    const endTime = `${year}${month}${day}${endParts[0]}${endParts[1]}00`;

    const playbackUrl = `${baseUrl}?playseek=${startTime}-${endTime}`;

    console.log('Playback URL:', playbackUrl);
    emit('playback', playbackUrl);
  } catch (e) {
    console.error('Failed to generate playback URL:', e);
    error.value = '生成回看地址失败';
  }
};
</script>

<style scoped>.channel-list {
  background: var(--card-bg);
  border-radius: 8px;
  padding: 20px;
  margin-top: 20px;
}

.channel-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.channel-list-header h4 {
  margin: 0;
}

.tab-buttons {
  display: flex;
  gap: 8px;
}

.tab-btn {
  padding: 6px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.tab-btn:hover {
  background: var(--border-color);
  color: var(--text-primary);
}

.tab-btn.active {
  background: #3b82f6;
  border-color: #3b82f6;
  color: white;
}

.channels {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 400px;
  overflow-y: auto;
}

.channel-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
}

.channel-item:hover {
  background: var(--border-color);
}

.channel-item.active {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.1);
}

.channel-logo {
  width: 40px;
  height: 40px;
  object-fit: contain;
  border-radius: 4px;
  margin-right: 12px;
  background: var(--bg-secondary);
}

.channel-info {
  flex: 1;
  min-width: 0;
}

.channel-name {
  font-weight: 500;
  margin-bottom: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.channel-group {
  font-size: 12px;
  color: var(--text-secondary);
}

.groups {
  max-height: 400px;
  overflow-y: auto;
}

.group-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.group-item {
  display: flex;
  align-items: center;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
}

.group-item:hover {
  background: var(--border-color);
  border-color: #3b82f6;
}

.group-icon {
  font-size: 24px;
  margin-right: 12px;
}

.group-info {
  flex: 1;
}

.group-name {
  font-weight: 500;
  font-size: 15px;
  margin-bottom: 4px;
}

.group-count {
  font-size: 12px;
  color: var(--text-secondary);
}

.group-channels {
  display: flex;
  flex-direction: column;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.btn-back {
  padding: 6px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.btn-back:hover {
  background: var(--border-color);
}

.group-header h5 {
  margin: 0;
  flex: 1;
}

.epg-tab {
  max-height: 400px;
  overflow-y: auto;
}

.epg-date-selector {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  margin-bottom: 12px;
}

.date-nav-btn {
  padding: 6px 12px;
  background: #3b82f6;
  border: none;
  color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
  transition: all 0.2s;
}

.date-nav-btn:hover:not(:disabled) {
  background: #2563eb;
}

.date-nav-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.date-display {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.date-text {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.relative-date {
  font-size: 12px;
  color: var(--accent);
}

.channel-name-display-only {
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  margin-bottom: 8px;
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
  text-align: center;
}

.epg-loading, .epg-error, .epg-empty {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-secondary);
}

.loading-spinner {
  font-size: 32px;
  margin-bottom: 12px;
  animation: spin 1.5s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.btn-retry {
  margin-top: 12px;
  padding: 8px 16px;
  background: #3b82f6;
  border: none;
  color: white;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.btn-retry:hover {
  background: #2563eb;
}

.diyp-programs {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.program-item {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  transition: all 0.2s;
  border-left: 3px solid transparent;
  align-items: center;
}

.program-item.is-current {
  background: rgba(59, 130, 246, 0.15);
  border-left-color: #3b82f6;
}

.program-time {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 60px;
  font-size: 13px;
  color: var(--text-secondary);
}

.program-time .start {
  color: var(--accent);
  font-weight: 500;
}

.program-time .separator {
  margin: 2px 0;
  opacity: 0.5;
}

.program-info {
  flex: 1;
  min-width: 0;
}

.program-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.playing-icon {
  font-size: 12px;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.program-desc {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-program-playback {
  padding: 6px 10px;
  background: #8b5cf6;
  border: none;
  color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
  transition: all 0.2s;
  flex-shrink: 0;
}

.btn-program-playback:hover {
  background: #7c3aed;
  transform: scale(1.1);
}

.btn-program-playback:active {
  transform: scale(0.95);
}
</style>
