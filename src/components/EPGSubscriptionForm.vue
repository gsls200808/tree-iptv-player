<template>
  <div class="subscription-form">
    <h3>添加EPG订阅</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label for="epg-name">名称（可选）</label>
        <input
            id="epg-name"
            v-model="form.name"
            type="text"
            placeholder="输入EPG订阅名称"
        />
      </div>

      <div class="form-group">
        <label for="epg-type">EPG类型 *</label>
        <select
            id="epg-type"
            v-model="form.type"
            required
        >
          <option value="">请选择类型</option>
          <option value="xmltv">XMLTV格式</option>
          <option value="diyp">DIYP格式</option>
        </select>
      </div>

      <div class="form-group">
        <label for="epg-url">EPG地址 *</label>
        <input
            id="epg-url"
            v-model="form.url"
            type="text"
            :placeholder="urlPlaceholder"
            required
        />
      </div>

      <div class="form-actions">
        <button type="submit" :disabled="loading || !form.url || !form.type">
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
import { fetchUrlContent } from '../utils/tauriApi';
import { parseXMLTV, parseDIYP } from '../utils/epgParser';

const emit = defineEmits<{
  submit: [data: { name: string; url: string; type: 'xmltv' | 'diyp'; programs: any[]; channels: any[] }];
  cancel: [];
}>();

const form = ref({
  name: '',
  url: '',
  type: '' as 'xmltv' | 'diyp' | '',
});

const loading = ref(false);
const error = ref('');

const urlPlaceholder = computed(() => {
  if (form.value.type === 'xmltv') {
    return 'http://example.com/epg.xml';
  } else if (form.value.type === 'diyp') {
    return 'http://example.com/epg.json';
  }
  return '请先选择EPG类型';
});

const handleSubmit = async () => {
  if (!form.value.url || !form.value.type) return;

  loading.value = true;
  error.value = '';

  try {
    const url = form.value.url.trim();
    const content = await fetchUrlContent(url);

    let result;
    if (form.value.type === 'xmltv') {
      result = parseXMLTV(content);
    } else {
      result = parseDIYP(content);
    }

    const name = form.value.name.trim() || `${form.value.type === 'xmltv' ? 'XMLTV' : 'DIYP'} EPG`;

    emit('submit', {
      name,
      url,
      type: form.value.type,
      programs: result.programs,
      channels: result.channels,
    });

    form.value = { name: '', url: '', type: '' };
  } catch (e) {
    error.value = '添加失败，请检查地址和类型是否正确';
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

.form-group input,
.form-group select {
  width: 100%;
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  box-sizing: border-box;
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.form-group select {
  cursor: pointer;
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

