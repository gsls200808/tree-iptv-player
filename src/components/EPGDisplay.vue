<template>
  <div v-if="currentProgram" class="epg-display">
    <div class="epg-header">
      <h4>📺 节目信息</h4>
    </div>
    <div class="epg-content">
      <div class="program-current">
        <div class="program-badge">正在播放</div>
        <div class="program-title">{{ currentProgram.title }}</div>
        <div class="program-time">
          {{ formatTime(currentProgram.start) }} - {{ formatTime(currentProgram.stop) }}
        </div>
        <div v-if="currentProgram.description" class="program-description">
          {{ currentProgram.description }}
        </div>
      </div>

      <div v-if="upcomingPrograms.length > 0" class="program-upcoming">
        <h5>后续节目</h5>
        <div
            v-for="(program, index) in upcomingPrograms.slice(0, 5)"
            :key="index"
            class="program-item"
        >
          <div class="program-item-time">{{ formatTime(program.start) }}</div>
          <div class="program-item-title">{{ program.title }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { EPGProgram } from '../types';
import { findProgramsByChannel } from '../utils/epgParser';

const props = defineProps<{
  programs: EPGProgram[];
  channelName: string;
}>();

const currentProgram = computed(() => {
  const now = new Date();
  const channelPrograms = props.programs.filter(p =>
      p.channel.toLowerCase() === props.channelName.toLowerCase()
  );

  return channelPrograms.find(p => p.start <= now && p.stop >= now) || null;
});

const upcomingPrograms = computed(() => {
  const now = new Date();
  const channelPrograms = props.programs.filter(p =>
      p.channel.toLowerCase() === props.channelName.toLowerCase() && p.start >= now
  );

  return channelPrograms.sort((a, b) => a.start.getTime() - b.start.getTime());
});

const formatTime = (date: Date) => {
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
};
</script>

<style scoped>
.epg-display {
  background: var(--card-bg);
  border-radius: 8px;
  padding: 20px;
  margin-top: 20px;
}

.epg-header {
  margin-bottom: 16px;
}

.epg-header h4 {
  margin: 0;
  font-size: 16px;
}

.epg-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.program-current {
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: 8px;
  border-left: 4px solid #3b82f6;
}

.program-badge {
  display: inline-block;
  padding: 4px 12px;
  background: #3b82f6;
  color: white;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  margin-bottom: 8px;
}

.program-title {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
}

.program-time {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.program-description {
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.program-upcoming h5 {
  margin: 0 0 12px 0;
  font-size: 14px;
  color: var(--text-secondary);
}

.program-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  margin-bottom: 8px;
}

.program-item-time {
  font-size: 13px;
  color: var(--accent);
  font-weight: 500;
  min-width: 60px;
}

.program-item-title {
  font-size: 14px;
  flex: 1;
}
</style>

