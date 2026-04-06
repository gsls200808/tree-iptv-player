<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import HlsPlayer from './components/HlsPlayer.vue';
import SubscriptionForm from './components/SubscriptionForm.vue';
import SubscriptionList from './components/SubscriptionList.vue';
import ChannelList from './components/ChannelList.vue';
import type { Subscription } from './types';
import {
  getSubscriptions,
  saveSubscription,
  deleteSubscription,
  updateSubscription,
  fetchPlaylist,
} from './utils/subscription';

const subscriptions = ref<Subscription[]>([]);
const activeSubscriptionId = ref<string | null>(null);
const showAddForm = ref(false);
const currentStreamUrl = ref<string>('');
const activeChannelIndex = ref(0);

const activeSubscription = computed(() => {
  if (!activeSubscriptionId.value) return null;
  return subscriptions.value.find(s => s.id === activeSubscriptionId.value);
});

const currentChannels = computed(() => {
  return activeSubscription.value?.channels || [];
});

onMounted(() => {
  loadSubscriptions();
});

const loadSubscriptions = () => {
  subscriptions.value = getSubscriptions();
};

const handleAddSubscription = async (data: {
  name: string;
  url: string;
  type: 'single' | 'playlist';
  channels: any[];
}) => {
  const now = Date.now();
  const newSubscription: Subscription = {
    id: crypto.randomUUID(),
    url: data.url,
    name: data.name,
    type: data.type,
    channels: data.channels,
    currentChannelIndex: 0,
    createdAt: now,
    updatedAt: now,
  };

  // 如果是播放列表且成功获取了频道，保存频道信息
  if (data.type === 'playlist' && data.channels.length > 0) {
    saveSubscription(newSubscription);
  } else if (data.type === 'single') {
    // 单一直播流
    saveSubscription(newSubscription);
  } else {
    // 播放列表但未能获取频道，保存为空频道列表
    saveSubscription(newSubscription);
  }

  loadSubscriptions();
  showAddForm.value = false;

  // 自动选中新添加的订阅
  activeSubscriptionId.value = newSubscription.id;
  if (data.type === 'single') {
    currentStreamUrl.value = data.url;
  } else if (data.channels.length > 0) {
    currentStreamUrl.value = data.channels[0].url;
  }
};

const handleSelectSubscription = (id: string) => {
  activeSubscriptionId.value = id;
  const sub = subscriptions.value.find(s => s.id === id);

  if (!sub) return;

  if (sub.type === 'single') {
    currentStreamUrl.value = sub.url;
  } else if (sub.channels.length > 0) {
    const index = sub.currentChannelIndex || 0;
    activeChannelIndex.value = index;
    currentStreamUrl.value = sub.channels[index]?.url || '';
  }
};

const handleDeleteSubscription = (id: string) => {
  if (confirm('确定要删除这个订阅吗？')) {
    deleteSubscription(id);
    loadSubscriptions();

    if (activeSubscriptionId.value === id) {
      activeSubscriptionId.value = null;
      currentStreamUrl.value = '';
    }
  }
};

const handleSelectChannel = (index: number) => {
  if (!activeSubscription.value) return;

  activeChannelIndex.value = index;
  const sub = activeSubscription.value;
  currentStreamUrl.value = sub.channels[index]?.url || '';

  // 保存当前频道索引
  sub.currentChannelIndex = index;
  sub.updatedAt = Date.now();
  updateSubscription(sub);
};

const handleRefreshPlaylist = async () => {
  if (!activeSubscription.value || activeSubscription.value.type !== 'playlist') return;

  try {
    const channels = await fetchPlaylist(activeSubscription.value.url);
    activeSubscription.value.channels = channels;
    activeSubscription.value.updatedAt = Date.now();
    updateSubscription(activeSubscription.value);
    loadSubscriptions();
    alert('播放列表已刷新');
  } catch (e) {
    alert('刷新播放列表失败');
    console.error(e);
  }
};
</script>

<template>
  <main class="app-container">
    <header class="app-header">
      <h1>📺 HLS 流媒体播放器</h1>
    </header>

    <div class="app-content">
      <aside class="sidebar">
        <SubscriptionList
            v-if="!showAddForm"
            :subscriptions="subscriptions"
            :active-id="activeSubscriptionId || ''"
            @add="showAddForm = true"
            @select="handleSelectSubscription"
            @delete="handleDeleteSubscription"
        />

        <SubscriptionForm
            v-else
            @submit="handleAddSubscription"
            @cancel="showAddForm = false"
        />

        <button
            v-if="activeSubscription?.type === 'playlist'"
            @click="handleRefreshPlaylist"
            class="btn-refresh"
        >
          🔄 刷新播放列表
        </button>
      </aside>

      <section class="main-content">
        <div v-if="currentStreamUrl" class="player-section">
          <HlsPlayer :src="currentStreamUrl" />

          <div v-if="activeSubscription" class="player-info">
            <h2>{{ activeSubscription.name }}</h2>
            <p class="stream-url">{{ currentStreamUrl }}</p>

            <div v-if="activeSubscription.type === 'playlist' && currentChannels.length > 0" class="current-channel">
              正在播放: <strong>{{ currentChannels[activeChannelIndex]?.name }}</strong>
              <span class="channel-index">({{ activeChannelIndex + 1 }} / {{ currentChannels.length }})</span>
            </div>
          </div>
        </div>

        <div v-else class="empty-player">
          <div class="placeholder">
            <div class="icon">📺</div>
            <p>选择一个订阅开始播放</p>
            <p class="hint">或添加新的订阅地址</p>
          </div>
        </div>

        <ChannelList
            v-if="currentChannels.length > 0"
            :channels="currentChannels"
            :active-index="activeChannelIndex"
            @select="handleSelectChannel"
        />
      </section>
    </div>
  </main>
</template>

<style>
:root {
  --bg-primary: #0f0f0f;
  --bg-secondary: #1a1a1a;
  --card-bg: #1e1e1e;
  --border-color: #2a2a2a;
  --text-primary: #f6f6f6;
  --text-secondary: #a0a0a0;
  --accent: #3b82f6;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
  line-height: 1.6;
}

#app {
  width: 100%;
  height: 100vh;
}

.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.app-header {
  background: var(--card-bg);
  padding: 20px;
  border-bottom: 1px solid var(--border-color);
}

.app-header h1 {
  font-size: 24px;
  margin: 0;
}

.app-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 320px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-color);
  padding: 20px;
  overflow-y: auto;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 20px;
}

.player-section {
  margin-bottom: 20px;
}

.player-info {
  margin-top: 16px;
  padding: 16px;
  background: var(--card-bg);
  border-radius: 8px;
}

.player-info h2 {
  margin-bottom: 8px;
  font-size: 20px;
}

.stream-url {
  font-size: 13px;
  color: var(--text-secondary);
  word-break: break-all;
  margin-bottom: 12px;
}

.current-channel {
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  font-size: 14px;
}

.current-channel strong {
  color: var(--accent);
}

.channel-index {
  margin-left: 8px;
  color: var(--text-secondary);
  font-size: 13px;
}

.empty-player {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}

.placeholder {
  text-align: center;
  color: var(--text-secondary);
}

.placeholder .icon {
  font-size: 64px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.placeholder p {
  margin: 8px 0;
}

.placeholder .hint {
  font-size: 14px;
  opacity: 0.7;
}

.btn-refresh {
  width: 100%;
  margin-top: 12px;
  padding: 10px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.btn-refresh:hover {
  background: var(--border-color);
  border-color: var(--accent);
}

@media (max-width: 768px) {
  .app-content {
    flex-direction: column;
  }

  .sidebar {
    width: 100%;
    max-height: 40vh;
    border-right: none;
    border-bottom: 1px solid var(--border-color);
  }
}
</style>
