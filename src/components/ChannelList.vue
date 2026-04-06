<template>
  <div v-if="channels.length > 0" class="channel-list">
    <div class="channel-list-header">
      <h4>频道列表 ({{ channels.length }})</h4>

      <div v-if="hasEPGSupport" class="tab-buttons">
        <button
            :class="['tab-btn', { active: activeTab === 'channels' }]"
            @click="activeTab = 'channels'"
        >
          📺 频道
        </button>
        <button
            :class="['tab-btn', { active: activeTab === 'epg' }]"
            @click="activeTab = 'epg'"
        >
          📋 节目表
        </button>
      </div>
    </div>

    <div v-if="activeTab === 'channels'" class="channels">
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

    <div v-else-if="activeTab === 'epg'" class="epg-tab">
      <div v-if="loading" class="epg-loading">
        <div class="loading-spinner">⏳</div>
        <p>加载节目表中...</p>
      </div>

      <div v-else-if="error" class="epg-error">
        <p>❌ {{ error }}</p>
        <button @click="loadEPGData" class="btn-retry">重试</button>
      </div>

      <div v-else-if="diypPrograms.length > 0" class="diyp-programs">
        <div class="epg-date-info">
          <span class="date">{{ currentDate }}</span>
          <span class="channel-name-display">{{ currentChannelName }}</span>
        </div>

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

defineEmits<{
  select: [index: number];
}>();

const activeTab = ref<'channels' | 'epg'>('channels');
const loading = ref(false);
const error = ref<string>('');
const diypPrograms = ref<any[]>([]);

const hasEPGSupport = computed(() => {
  return props.epgSubscriptions && props.epgSubscriptions.length > 0;
});

const currentChannelName = computed(() => {
  if (props.activeIndex !== undefined && props.channels[props.activeIndex]) {
    return props.channels[props.activeIndex].name;
  }
  return '';
});

const currentDate = computed(() => {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
});

const isCurrentProgram = (program: any) => {
  const now = new Date();
  const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const startTime = program.start;
  const endTime = program.end;

  return startTime <= currentTime && endTime >= currentTime;
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
        const dateStr = currentDate.value;
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
              diypPrograms.value = channelPrograms.map(prog => ({
                start: formatTime(prog.start),
                end: formatTime(prog.stop),
                title: prog.title,
                desc: prog.description
              }));
              foundPrograms = true;
              break;
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
    loadEPGData();
  }
});

const handleImageError = (e: Event) => {
  const target = e.target as HTMLImageElement;
  target.style.display = 'none';
};
</script>

<style scoped>
.channel-list {
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

.epg-tab {
  max-height: 400px;
  overflow-y: auto;
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

.epg-date-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  margin-bottom: 8px;
}

.date {
  font-size: 14px;
  color: var(--accent);
  font-weight: 500;
}

.channel-name-display {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.program-item {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  transition: all 0.2s;
  border-left: 3px solid transparent;
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
</style>
