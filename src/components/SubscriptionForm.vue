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
        <div class="url-input-group">
          <input
              id="sub-url"
              v-model="form.url"
              type="text"
              placeholder="http://example.com/playlist.m3u8 或选择本地文件"
              required
          />
          <button type="button" class="file-select-btn" @click="handleSelectFile" :disabled="loading">
            📁 选择文件
          </button>
        </div>
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
import { detectStreamType, fetchPlaylist, parseM3U } from '../utils/subscription';
import { selectPlaylistFile, readLocalFile } from '../utils/tauriApi';
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

const handleSelectFile = async () => {
  try {
    const filePath = await selectPlaylistFile();
    if (filePath) {
      form.value.url = filePath;
      
      // 读取本地文件内容并检测类型
      loading.value = true;
      error.value = '';
      
      try {
        const content = await readLocalFile(filePath);
        
        // 根据文件扩展名和内容判断类型
        let type: 'single' | 'playlist' = 'single';
        const trimmedContent = content.trim();
        
        // M3U/M3U8 文件通常是播放列表
        if (filePath.endsWith('.m3u') || filePath.endsWith('.m3u8')) {
          if (trimmedContent.startsWith('#EXTM3U')) {
            type = 'playlist';
          }
        } else if (filePath.endsWith('.mpd')) {
          // MPD 文件可能是单个流或包含在 M3U 中
          // 如果内容是 XML 格式的 MPD，则是单流
          if (trimmedContent.startsWith('<?xml') || trimmedContent.includes('<MPD')) {
            type = 'single';
          } else {
            // 可能是一个 M3U 文件包含了 MPD 链接
            type = trimmedContent.startsWith('#EXTM3U') ? 'playlist' : 'single';
          }
        }
        
        detectedType.value = type;
        
        // 如果是播放列表，解析它
        if (type === 'playlist') {
          try {
            if (filePath.endsWith('.m3u') || filePath.endsWith('.m3u8')) {
              const channels = parseM3U(content);
              console.log(`Parsed ${channels.length} channels from local file`);
            }
          } catch (e) {
            console.error('Failed to parse local playlist:', e);
          }
        }
      } catch (e) {
        console.error('Failed to process local file:', e);
        error.value = '读取文件失败';
      } finally {
        loading.value = false;
      }
    }
  } catch (e) {
    console.error('File selection failed:', e);
    error.value = '选择文件失败';
  }
};

const handleSubmit = async () => {
  if (!form.value.url) return;

  loading.value = true;
  error.value = '';

  try {
    const url = form.value.url.trim();
    
    // 使用已检测的类型，如果没有则重新检测
    let type = detectedType.value;
    if (!type) {
      type = await detectStreamType(url);
      detectedType.value = type;
    }

    let channels: ChannelItem[] = [];
    if (type === 'playlist') {
      try {
        // 如果是本地文件，需要特殊处理
        if (url.startsWith('file://') || !url.startsWith('http')) {
          // 本地文件，读取并解析
          const content = await readLocalFile(url);
          if (url.endsWith('.m3u') || url.endsWith('.m3u8')) {
            channels = parseM3U(content);
          }
        } else {
          // 网络 URL
          channels = await fetchPlaylist(url);
        }
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

.url-input-group {
  display: flex;
  gap: 8px;
}

.url-input-group input {
  flex: 1;
}

.file-select-btn {
  padding: 10px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.file-select-btn:hover:not(:disabled) {
  background: var(--border-color);
}

.file-select-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
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

