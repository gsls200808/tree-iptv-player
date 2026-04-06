<template>
  <div class="subscription-form">
    <h3>添加订阅</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label for="sub-name">名称（可选）</label>
        <input
            id="sub-name"
            v-model="form.name"
            type="text"
            placeholder="输入订阅名称"
        />
      </div>

      <div class="form-group">
        <label for="sub-url">流地址 *</label>
        <input
            id="sub-url"
            v-model="form.url"
            type="text"
            placeholder="http://example.com/playlist.m3u8 或 http://example.com/stream.m3u8"
            required
        />
      </div>

      <div v-if="previewType" class="type-preview">
        类型: <span :class="['badge', previewType === 'playlist' ? 'badge-playlist' : 'badge-single']">
          {{ previewType === 'playlist' ? '多频道列表' : '单一直播流' }}
        </span>
      </div>

      <div class="form-actions">
        <button type="submit" :disabled="loading || !form.url">
          {{ loading ? '处理中...' : '添加' }}
        </button>
        <button type="button" @click="$emit('cancel')" :disabled="loading">
          取消
        </button>
      </div>

      <div v-if="error" class="error-message">{{ error }}</div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { detectStreamType, fetchPlaylist } from '../utils/subscription';
import type { ChannelItem } from '../types';

const emit = defineEmits<{
  submit: [data: { name: string; url: string; type: 'single' | 'playlist'; channels: any[] }];
  cancel: [];
}>();

const form = ref({
  name: '',
  url: '',
});

const loading = ref(false);
const error = ref('');
const detectedType = ref<'single' | 'playlist' | null>(null);

const previewType = computed(() => {
  return detectedType.value;
});

const handleSubmit = async () => {
  if (!form.value.url) return;

  loading.value = true;
  error.value = '';

  try {
    const url = form.value.url.trim();
    const type = await detectStreamType(url);
    detectedType.value = type;

    let channels: ChannelItem[] = [];
    if (type === 'playlist') {
      try {
        channels = await fetchPlaylist(url);
      } catch (e) {
        console.error('Failed to fetch playlist, treating as single stream:', e);
        channels = [];
      }
    }

    const name = form.value.name.trim()
    emit('submit', {
      name,
      url,
      type,
      channels,
    });

    form.value = { name: '', url: '' };
    detectedType.value = null;
  } catch (e) {
    error.value = '添加失败，请检查地址是否正确';
    console.error(e);
  } finally {
    loading.value = false;
  }
};
</script>

<style scoped>
.subscription-form {
  background: var(--card-bg);
  padding: 20px;
  border-radius: 8px;
  margin-bottom: 20px;
}

.subscription-form h3 {
  margin-top: 0;
  margin-bottom: 16px;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  font-weight: 500;
}

.form-group input {
  width: 100%;
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  box-sizing: border-box;
}

.type-preview {
  margin-bottom: 16px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
}

.badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  margin-left: 8px;
}

.badge-playlist {
  background: #3b82f6;
  color: white;
}

.badge-single {
  background: #10b981;
  color: white;
}

.form-actions {
  display: flex;
  gap: 10px;
}

.form-actions button {
  flex: 1;
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.form-actions button[type="submit"] {
  background: #3b82f6;
  color: white;
}

.form-actions button[type="submit"]:hover:not(:disabled) {
  background: #2563eb;
}

.form-actions button[type="submit"]:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.form-actions button[type="button"] {
  background: var(--bg-secondary);
  color: var(--text-color);
}

.form-actions button[type="button"]:hover:not(:disabled) {
  background: var(--border-color);
}

.error-message {
  margin-top: 12px;
  padding: 10px;
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  border-radius: 6px;
  font-size: 14px;
}
</style>

