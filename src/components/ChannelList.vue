<template>
  <div v-if="channels.length > 0" class="channel-list">
    <h4>频道列表 ({{ channels.length }})</h4>
    <div class="channels">
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
  </div>
</template>

<script setup lang="ts">
import type { ChannelItem } from '../types';

defineProps<{
  channels: ChannelItem[];
  activeIndex?: number;
}>();

defineEmits<{
  select: [index: number];
}>();

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

.channel-list h4 {
  margin-top: 0;
  margin-bottom: 16px;
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
</style>
