<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import HlsPlayer from './components/HlsPlayer.vue';
import SubscriptionForm from './components/SubscriptionForm.vue';
import EPGSubscriptionForm from './components/EPGSubscriptionForm.vue';
import SubscriptionList from './components/SubscriptionList.vue';
import ChannelList from './components/ChannelList.vue';
import type { Subscription } from './types';
import {
  getSubscriptions,
  saveSubscription,
  deleteSubscription,
  updateSubscription,
  fetchPlaylist,
  getEPGSubscriptions,
  saveEPGSubscription,
  deleteEPGSubscription,
} from './utils/subscription';

const subscriptions = ref<Subscription[]>([]);
const epgSubscriptions = ref<any[]>([]);
const activeSubscriptionId = ref<string | null>(null);
const showAddStreamForm = ref(false);
const showAddEPGForm = ref(false);
const currentStreamUrl = ref<string>('');
const activeChannelIndex = ref(0);
const activeTab = ref<'stream' | 'epg'>('stream');

const activeSubscription = computed(() => {
  if (!activeSubscriptionId.value) return null;
  return subscriptions.value.find(s => s.id === activeSubscriptionId.value);
});

const currentChannels = computed(() => {
  return activeSubscription.value?.channels || [];
});
computed(() => {
  if (!activeSubscription.value || currentChannels.value.length === 0) return null;
  return currentChannels.value[activeChannelIndex.value] || null;
});
onMounted(() => {
  loadSubscriptions();
  loadEPGSubscriptions();
});

const loadSubscriptions = () => {
  subscriptions.value = getSubscriptions();
};

const loadEPGSubscriptions = () => {
  epgSubscriptions.value = getEPGSubscriptions();
};

const handleAddStreamSubscription = async (data: {
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

  if (data.type === 'playlist' && data.channels.length > 0) {
    saveSubscription(newSubscription);
  } else if (data.type === 'single') {
    saveSubscription(newSubscription);
  } else {
    saveSubscription(newSubscription);
  }

  loadSubscriptions();
  showAddStreamForm.value = false;

  activeSubscriptionId.value = newSubscription.id;
  if (data.type === 'single') {
    currentStreamUrl.value = data.url;
  } else if (data.channels.length > 0) {
    currentStreamUrl.value = data.channels[0].url;
  }
};

const handleAddEPGSubscription = (data: {
  name: string;
  url: string;
  type: 'xmltv' | 'diyp';
  programs: any[];
  channels: any[];
}) => {
  const now = Date.now();
  const newEPGSubscription = {
    id: crypto.randomUUID(),
    name: data.name,
    url: data.url,
    type: data.type,
    programs: data.programs,
    channels: data.channels,
    lastUpdate: now,
    createdAt: now,
    updatedAt: now,
  };

  saveEPGSubscription(newEPGSubscription);
  loadEPGSubscriptions();
  showAddEPGForm.value = false;
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

const handleDeleteEPGSubscription = (id: string) => {
  if (confirm('确定要删除这个EPG订阅吗？')) {
    deleteEPGSubscription(id);
    loadEPGSubscriptions();
  }
};

const handleSelectChannel = (index: number) => {
  if (!activeSubscription.value) return;

  activeChannelIndex.value = index;
  const sub = activeSubscription.value;
  currentStreamUrl.value = sub.channels[index]?.url || '';

  sub.currentChannelIndex = index;
  sub.updatedAt = Date.now();
  updateSubscription(sub);
};

const handlePlayback = (url: string) => {
  console.log('Starting playback:', url);
  currentStreamUrl.value = url;
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

const handleRefreshEPG = async (epgSub: any) => {
  try {
    const { fetchUrlContent } = await import('./utils/tauriApi');
    const { parseXMLTV, parseDIYP } = await import('./utils/epgParser');

    const content = await fetchUrlContent(epgSub.url);
    let result;

    if (epgSub.type === 'xmltv') {
      result = parseXMLTV(content);
    } else {
      result = parseDIYP(content);
    }

    epgSub.programs = result.programs;
    epgSub.channels = result.channels;
    epgSub.lastUpdate = Date.now();
    epgSub.updatedAt = Date.now();

    const { updateEPGSubscription } = await import('./utils/subscription');
    updateEPGSubscription(epgSub);
    loadEPGSubscriptions();
    alert('EPG已刷新');
  } catch (e) {
    alert('刷新EPG失败');
    console.error(e);
  }
};
</script>

<template>
  <main class="app-container">
    <header class="app-header">
      <h1>📺 IPTV 播放器</h1>
    </header>

    <div class="app-content">
      <aside class="sidebar">
        <div class="tab-buttons">
          <button
              :class="['tab-btn', { active: activeTab === 'stream' }]"
              @click="activeTab = 'stream'"
          >
            📡 直播源
          </button>
          <button
              :class="['tab-btn', { active: activeTab === 'epg' }]"
              @click="activeTab = 'epg'"
          >
            📋 EPG
          </button>
        </div>

        <div v-if="activeTab === 'stream'" class="tab-content">
          <SubscriptionList
              v-if="!showAddStreamForm"
              :subscriptions="subscriptions"
              :active-id="activeSubscriptionId || ''"
              title="直播源订阅"
              hint-text="点击上方添加按钮添加直播源订阅"
              @add="showAddStreamForm = true"
              @select="handleSelectSubscription"
              @delete="handleDeleteSubscription"
          />

          <SubscriptionForm
              v-else
              @submit="handleAddStreamSubscription"
              @cancel="showAddStreamForm = false"
          />
        </div>

        <div v-else class="tab-content">
          <SubscriptionList
              v-if="!showAddEPGForm"
              :subscriptions="epgSubscriptions"
              :active-id="''"
              title="EPG订阅"
              hint-text="点击上方添加按钮添加EPG订阅"
              @add="showAddEPGForm = true"
              @select="() => {}"
              @delete="handleDeleteEPGSubscription"
          />

          <EPGSubscriptionForm
              v-else
              @submit="handleAddEPGSubscription"
              @cancel="showAddEPGForm = false"
          />
        </div>

        <button
            v-if="activeSubscription?.type === 'playlist'"
            @click="handleRefreshPlaylist"
            class="btn-refresh"
        >
          🔄 刷新播放列表
        </button>

        <div v-if="epgSubscriptions.length > 0" class="epg-refresh-section">
          <button
              v-for="epgSub in epgSubscriptions"
              :key="epgSub.id"
              @click="handleRefreshEPG(epgSub)"
              class="btn-refresh-epg"
              :title="`刷新 ${epgSub.name}`"
          >
            🔄 {{ epgSub.name }}
          </button>
        </div>
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
            :epg-subscriptions="epgSubscriptions"
            @select="handleSelectChannel"
            @playback="handlePlayback"
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
  display: flex;
  flex-direction: column;
}

.tab-buttons {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.tab-btn {
  flex: 1;
  padding: 10px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.tab-btn:hover {
  background: var(--border-color);
  color: var(--text-primary);
}

.tab-btn.active {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.tab-content {
  flex: 1;
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

.epg-refresh-section {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.btn-refresh-epg {
  width: 100%;
  padding: 10px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-refresh-epg:hover {
  background: var(--border-color);
  border-color: #8b5cf6;
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

