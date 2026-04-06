<template>
  <div class="subscription-list">
    <div class="list-header">
      <h3>{{ title }}</h3>
      <button @click="$emit('add')" class="btn-add">+ 添加</button>
    </div>

    <div v-if="subscriptions.length === 0" class="empty-state">
      <p>暂无订阅</p>
      <p class="hint">{{ hintText }}</p>
    </div>

    <div v-else class="list-items">
      <div
          v-for="sub in subscriptions"
          :key="sub.id"
          :class="['subscription-item', { active: sub.id === activeId }]"
          @click="$emit('select', sub.id)"
      >
        <div class="item-info">
          <div class="item-name">{{ sub.name }}</div>
          <div class="item-meta">
            <span :class="['badge', getTypeBadgeClass(sub)]">
              {{ getTypeText(sub) }}
            </span>
            <span v-if="isStreamSubscription(sub) && sub.type === 'playlist'" class="channel-count">
              {{ sub.channels.length }} 个频道
            </span>
          </div>
        </div>

        <button
            class="btn-delete"
            @click.stop="$emit('delete', sub.id)"
            title="删除"
        >
          ×
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  subscriptions: any[];
  activeId?: string;
  title: string;
  hintText: string;
}>();

defineEmits<{
  add: [];
  select: [id: string];
  delete: [id: string];
}>();

const isStreamSubscription = (sub: any) => {
  return sub.type === 'single' || sub.type === 'playlist';
};

const getTypeBadgeClass = (sub: any) => {
  if (sub.type === 'playlist') return 'badge-playlist';
  if (sub.type === 'single') return 'badge-single';
  if (sub.type === 'xmltv') return 'badge-xmltv';
  if (sub.type === 'diyp') return 'badge-diyp';
  return 'badge-single';
};

const getTypeText = (sub: any) => {
  if (sub.type === 'playlist') return '播放列表';
  if (sub.type === 'single') return '直播流';
  if (sub.type === 'xmltv') return 'XMLTV';
  if (sub.type === 'diyp') return 'DIYP';
  return '未知';
};
</script>

<style scoped>
.subscription-list {
  background: var(--card-bg);
  border-radius: 8px;
  padding: 20px;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.list-header h3 {
  margin: 0;
}

.btn-add {
  padding: 8px 16px;
  background: #3b82f6;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: background 0.2s;
}

.btn-add:hover {
  background: #2563eb;
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-secondary);
}

.empty-state .hint {
  font-size: 13px;
  margin-top: 8px;
}

.list-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.subscription-item {
  display: flex;
  align-items: center;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
}

.subscription-item:hover {
  background: var(--border-color);
}

.subscription-item.active {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.1);
}

.item-info {
  flex: 1;
  min-width: 0;
}

.item-name {
  font-weight: 500;
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-meta {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 12px;
  color: var(--text-secondary);
}

.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
}

.badge-playlist {
  background: #3b82f6;
  color: white;
}

.badge-single {
  background: #10b981;
  color: white;
}

.badge-xmltv {
  background: #8b5cf6;
  color: white;
}

.badge-diyp {
  background: #f59e0b;
  color: white;
}

.channel-count {
  color: var(--text-secondary);
}

.btn-delete {
  width: 28px;
  height: 28px;
  border: none;
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  border-radius: 4px;
  font-size: 20px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-left: 8px;
}

.btn-delete:hover {
  background: #ef4444;
  color: white;
}
</style>

